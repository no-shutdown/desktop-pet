import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';

type ProgressPayload = {
  runId: string;
  phase: 'state' | 'assemble';
  state?: string;
  current: number;
  total: number;
};

type ProgressHandler = (event: { payload: ProgressPayload }) => void;

const { mockInvoke, mockListen, mockSaveSettings } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn(),
  mockSaveSettings: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mockListen }));
vi.mock('../../../../lib/settings', () => ({
  loadSettings: vi.fn().mockReturnValue({
    visionProvider: 'skip',
    visionApiKey: '',
    visionModel: '',
    imageProvider: 'siliconflow',
    rowImageProvider: 'siliconflow',
    imageApiKey: 'image-api-key',
    imageModel: 'legacy-model',
    imageBaseModel: 'base-model',
    imageReferenceModel: 'reference-model',
    wanxiangApiKey: '',
    wanxiangBaseModel: 'wanx2.1-t2i-turbo',
    wanxiangEditModel: 'wanx2.1-imageedit',
    localSdUrl: 'http://localhost:7860',
    localSdDenoisingStrength: 0.55,
  }),
  saveSettings: mockSaveSettings,
  apiKeyForProvider: (settings: { imageApiKey: string; wanxiangApiKey: string }, provider: string) =>
    provider === 'wanxiang' ? settings.wanxiangApiKey : settings.imageApiKey,
  rowModelForProvider: (
    settings: { imageReferenceModel: string; wanxiangEditModel: string },
    provider: string,
  ) => (provider === 'wanxiang' ? settings.wanxiangEditModel : settings.imageReferenceModel),
  SILICONFLOW_REFERENCE_MODELS: [
    { value: 'reference-model', label: 'Reference model' },
  ],
  WANXIANG_EDIT_MODELS: [
    { value: 'wanx2.1-imageedit', label: 'wanx2.1-imageedit' },
  ],
}));
vi.mock('../../../Pet/SpriteAnimator', () => ({
  default: (props: { sheetSrc: string }) => (
    <div data-testid="sprite-animator" data-src={props.sheetSrc} />
  ),
}));

import StateGenerationStep from '../StateGenerationStep';

describe('StateGenerationStep', () => {
  let progressHandler: ProgressHandler | undefined;

  const defaultProps = {
    runId: 'run-1',
    onNext: vi.fn(),
    onBack: vi.fn(),
  };

  function row(state: string) {
    return {
      runId: 'run-1',
      state,
      dataUrl: `data:image/png;base64,${state}`,
      frameW: 128,
      frameH: 128,
      frameCount: 8,
    };
  }

  const assembled = {
    runId: 'run-1',
    dataUrl: 'data:image/png;base64,combined',
    frameW: 128,
    frameH: 128,
    frameCount: 8,
    rowGap: 0,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    progressHandler = undefined;
    mockListen.mockImplementation((_event: string, handler: ProgressHandler) => {
      progressHandler = handler;
      return Promise.resolve(() => {});
    });
    mockInvoke.mockResolvedValue(row('idle'));
  });

  it('generates all four states, assembles a preview, and stays on the page until the user confirms', async () => {
    mockInvoke.mockImplementation(async (command: string, args: { state?: string }) => {
      if (command === 'generate_state_row') return row(args.state!);
      if (command === 'assemble_run_preview') return assembled;
      throw new Error(`unexpected command: ${command}`);
    });

    const onNext = vi.fn();
    render(<StateGenerationStep {...defaultProps} onNext={onNext} />);
    fireEvent.click(screen.getByRole('button', { name: '生成所有状态' }));

    const confirmButton = await screen.findByRole('button', { name: '下一步' });
    expect(onNext).not.toHaveBeenCalled();

    const rowCalls = mockInvoke.mock.calls
      .filter(([name]) => name === 'generate_state_row')
      .map(([, args]) => args as { state: string; runId: string });
    expect(rowCalls.map(({ state }) => state)).toEqual(['idle', 'sleeping', 'acting_cute', 'working']);
    expect(rowCalls.every(({ runId }) => runId === 'run-1')).toBe(true);
    expect(mockInvoke.mock.calls.filter(([name]) => name === 'generate_state_row').every(([, args]) => (
      args.imageProvider === 'siliconflow'
      && args.imageApiKey === 'image-api-key'
      && args.referenceModel === 'reference-model'
      && args.localSdUrl === 'http://localhost:7860'
      && args.denoisingStrength === 0.55
    ))).toBe(true);
    expect(mockInvoke).toHaveBeenLastCalledWith('assemble_run_preview', { runId: 'run-1' });

    fireEvent.click(confirmButton);
    expect(onNext).toHaveBeenCalledWith('data:image/png;base64,combined', {
      petId: 'run-1',
      runId: 'run-1',
      frameW: 128,
      frameH: 128,
      rowGap: 0,
      layout: 'horizontalRows',
      idleFrames: 8,
      sleepingFrames: 8,
      actingCuteFrames: 8,
      workingFrames: 8,
    });
  });

  it('reports state generation busy while the row command is pending', async () => {
    let resolveFirstRow!: (value: ReturnType<typeof row>) => void;
    let firstCall = true;
    mockInvoke.mockImplementation((command: string, args: { state?: string }) => {
      if (command === 'generate_state_row' && firstCall) {
        firstCall = false;
        return new Promise((resolve) => { resolveFirstRow = resolve; });
      }
      if (command === 'generate_state_row') return Promise.resolve(row(args.state!));
      return Promise.resolve(assembled);
    });
    const onBusyChange = vi.fn();
    render(<StateGenerationStep {...defaultProps} onBusyChange={onBusyChange} />);

    fireEvent.click(screen.getByRole('button', { name: '生成所有状态' }));

    await waitFor(() => expect(onBusyChange).toHaveBeenCalledWith(true));
    resolveFirstRow(row('idle'));
    await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false));
  });

  it('ignores progress events from another run', async () => {
    let resolveFirstRow!: (value: ReturnType<typeof row>) => void;
    let firstCall = true;
    mockInvoke.mockImplementation((command: string, args: { state?: string }) => {
      if (command === 'generate_state_row' && firstCall) {
        firstCall = false;
        return new Promise((resolve) => { resolveFirstRow = resolve; });
      }
      if (command === 'generate_state_row') return Promise.resolve(row(args.state!));
      return Promise.resolve(assembled);
    });

    render(<StateGenerationStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: '生成所有状态' }));
    await waitFor(() => expect(progressHandler).toBeDefined());
    expect(screen.getByText('进度：0 / 4')).toBeTruthy();

    act(() => progressHandler!({
      payload: { runId: 'other-run', phase: 'state', state: 'sleeping', current: 4, total: 4 },
    }));
    expect(screen.queryByText('进度：4 / 4')).toBeNull();

    act(() => progressHandler!({
      payload: { runId: 'run-1', phase: 'state', state: 'idle', current: 1, total: 4 },
    }));
    expect(screen.getByText('进度：1 / 4')).toBeTruthy();

    resolveFirstRow(row('idle'));
    await waitFor(() => expect(mockInvoke).toHaveBeenLastCalledWith('assemble_run_preview', { runId: 'run-1' }));
  });

  it('marks a failed state and lets the user regenerate only that state via its per-state button', async () => {
    let sleepingAttempts = 0;
    mockInvoke.mockImplementation(async (command: string, args: { state?: string }) => {
      if (command === 'generate_state_row') {
        if (args.state === 'sleeping' && sleepingAttempts++ === 0) {
          throw new Error('sleeping failed');
        }
        return row(args.state!);
      }
      if (command === 'assemble_run_preview') return assembled;
      throw new Error(`unexpected command: ${command}`);
    });

    const onNext = vi.fn();
    render(<StateGenerationStep {...defaultProps} onNext={onNext} />);
    fireEvent.click(screen.getByRole('button', { name: '生成所有状态' }));

    expect(await screen.findByText('sleeping failed')).toBeTruthy();
    expect(screen.getByText('待机：已完成')).toBeTruthy();
    expect(screen.getByText('睡觉：失败')).toBeTruthy();

    // Only idle completed and sleeping failed. Per-state 🔄 button is available for both.
    expect(screen.getByRole('button', { name: '重新生成 待机' })).toBeTruthy();
    expect(screen.getByRole('button', { name: '重新生成 睡觉' })).toBeTruthy();

    // Regenerate just sleeping (second attempt succeeds); acting_cute/working stay pending.
    fireEvent.click(screen.getByRole('button', { name: '重新生成 睡觉' }));
    await screen.findByText('睡觉：已完成');
    expect(screen.queryByRole('button', { name: '下一步' })).toBeNull();

    // Fill the remaining two via the bulk button.
    fireEvent.click(screen.getByRole('button', { name: /生成剩余/ }));
    await screen.findByRole('button', { name: '下一步' });

    const states = mockInvoke.mock.calls
      .filter(([name]) => name === 'generate_state_row')
      .map(([, args]) => (args as { state: string }).state);
    expect(states).toEqual(['idle', 'sleeping', 'sleeping', 'acting_cute', 'working']);
    expect(onNext).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: '下一步' }));
    expect(onNext).toHaveBeenCalledOnce();
  });

  it('opens an animation preview when a completed state cell is clicked', async () => {
    mockInvoke.mockImplementation(async (command: string, args: { state?: string }) => {
      if (command === 'generate_state_row') return row(args.state!);
      if (command === 'assemble_run_preview') return assembled;
      throw new Error(`unexpected command: ${command}`);
    });

    render(<StateGenerationStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: '生成所有状态' }));
    await screen.findByRole('button', { name: '下一步' });

    fireEvent.click(screen.getByRole('button', { name: '查看 待机 动画预览' }));

    expect(await screen.findByText('待机 动画预览')).toBeTruthy();
    const animator = screen.getByTestId('sprite-animator');
    expect(animator.getAttribute('data-src')).toBe('data:image/png;base64,idle');

    fireEvent.click(screen.getByRole('button', { name: '关闭' }));
    await waitFor(() => expect(screen.queryByText('待机 动画预览')).toBeNull());
  });
});
