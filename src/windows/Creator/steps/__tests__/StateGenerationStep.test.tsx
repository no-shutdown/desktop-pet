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
    imageApiKey: 'image-api-key',
    imageModel: 'legacy-model',
    imageBaseModel: 'base-model',
    imageReferenceModel: 'reference-model',
    localSdUrl: 'http://localhost:7860',
    localSdDenoisingStrength: 0.55,
  }),
  saveSettings: mockSaveSettings,
  SILICONFLOW_REFERENCE_MODELS: [
    { value: 'reference-model', label: 'Reference model' },
  ],
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

  beforeEach(() => {
    vi.clearAllMocks();
    progressHandler = undefined;
    mockListen.mockImplementation((_event: string, handler: ProgressHandler) => {
      progressHandler = handler;
      return Promise.resolve(() => {});
    });
    mockInvoke.mockResolvedValue(row('idle'));
  });

  it('generates idle, walking, waving, and working sequentially, then assembles', async () => {
    mockInvoke.mockImplementation(async (command: string, args: { state?: string }) => {
      if (command === 'generate_state_row') return row(args.state!);
      if (command === 'assemble_run_preview') {
        return {
          runId: 'run-1',
          dataUrl: 'data:image/png;base64,combined',
          frameW: 128,
          frameH: 128,
          frameCount: 8,
          rowGap: 0,
        };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    const onNext = vi.fn();
    render(<StateGenerationStep {...defaultProps} onNext={onNext} />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate all states' }));

    await waitFor(() => expect(onNext).toHaveBeenCalledOnce());

    const rowCalls = mockInvoke.mock.calls
      .filter(([name]) => name === 'generate_state_row')
      .map(([, args]) => args as { state: string; runId: string });
    expect(rowCalls.map(({ state }) => state)).toEqual(['idle', 'walking', 'waving', 'working']);
    expect(rowCalls.every(({ runId }) => runId === 'run-1')).toBe(true);
    expect(mockInvoke.mock.calls.filter(([name]) => name === 'generate_state_row').every(([, args]) => (
      args.imageProvider === 'siliconflow'
      && args.imageApiKey === 'image-api-key'
      && args.referenceModel === 'reference-model'
      && args.localSdUrl === 'http://localhost:7860'
      && args.denoisingStrength === 0.55
    ))).toBe(true);
    expect(mockInvoke).toHaveBeenLastCalledWith('assemble_run_preview', { runId: 'run-1' });
    expect(onNext).toHaveBeenCalledWith('data:image/png;base64,combined', {
      petId: 'run-1',
      runId: 'run-1',
      frameW: 128,
      frameH: 128,
      rowGap: 0,
      layout: 'horizontalRows',
      idleFrames: 8,
      walkingFrames: 8,
      wavingFrames: 8,
      workingFrames: 8,
    });
    expect(mockInvoke.mock.calls.every(([name]) => (
      name === 'generate_state_row' || name === 'assemble_run_preview'
    ))).toBe(true);
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
      return Promise.resolve({
        runId: 'run-1',
        dataUrl: 'data:image/png;base64,combined',
        frameW: 128,
        frameH: 128,
        frameCount: 8,
        rowGap: 0,
      });
    });

    render(<StateGenerationStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate all states' }));
    await waitFor(() => expect(progressHandler).toBeDefined());
    expect(screen.getByText('Progress: 0 / 4')).toBeTruthy();

    act(() => progressHandler!({
      payload: { runId: 'other-run', phase: 'state', state: 'walking', current: 4, total: 4 },
    }));
    expect(screen.queryByText('Progress: 4 / 4')).toBeNull();

    act(() => progressHandler!({
      payload: { runId: 'run-1', phase: 'state', state: 'idle', current: 1, total: 4 },
    }));
    expect(screen.getByText('Progress: 1 / 4')).toBeTruthy();

    resolveFirstRow(row('idle'));
    await waitFor(() => expect(mockInvoke).toHaveBeenLastCalledWith('assemble_run_preview', { runId: 'run-1' }));
  });

  it('retries only the failed state and preserves completed rows', async () => {
    let walkingAttempts = 0;
    mockInvoke.mockImplementation(async (command: string, args: { state?: string }) => {
      if (command === 'generate_state_row') {
        if (args.state === 'walking' && walkingAttempts++ === 0) {
          throw new Error('walking failed');
        }
        return row(args.state!);
      }
      if (command === 'assemble_run_preview') {
        return {
          runId: 'run-1',
          dataUrl: 'data:image/png;base64,combined',
          frameW: 128,
          frameH: 128,
          frameCount: 8,
          rowGap: 0,
        };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    const onNext = vi.fn();
    render(<StateGenerationStep {...defaultProps} onNext={onNext} />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate all states' }));

    expect(await screen.findByText('walking failed')).toBeTruthy();
    expect(screen.getByText('idle: complete')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Retry walking' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Retry idle' })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Retry walking' }));
    await waitFor(() => expect(onNext).toHaveBeenCalledOnce());

    const states = mockInvoke.mock.calls
      .filter(([name]) => name === 'generate_state_row')
      .map(([, args]) => (args as { state: string }).state);
    expect(states).toEqual(['idle', 'walking', 'walking', 'waving', 'working']);
    expect(mockInvoke.mock.calls.some(([name]) => name === 'generate_and_assemble')).toBe(false);
    expect(mockInvoke.mock.calls.some(([name]) => name === 'pollinations')).toBe(false);
  });
});
