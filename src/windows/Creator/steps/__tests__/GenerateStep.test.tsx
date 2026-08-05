import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

const { mockInvoke, mockListen } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mockListen }));
vi.mock('../../../../lib/settings', () => ({
  loadSettings: vi.fn().mockReturnValue({
    visionProvider: 'skip',
    visionApiKey: '',
    visionModel: '',
    imageProvider: 'pollinations',
    imageApiKey: '',
    imageModel: '',
    localSdUrl: '',
  }),
  saveSettings: vi.fn(),
  SILICONFLOW_MODELS: [],
}));

import GenerateStep from '../GenerateStep';

describe('GenerateStep', () => {
  const defaultProps = {
    prompt: 'anime chibi girl',
    onNext: vi.fn(),
    onBack: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
  });

  it('shows Generate button before starting', () => {
    render(<GenerateStep {...defaultProps} />);
    expect(screen.getByRole('button', { name: /开始生成/ })).toBeTruthy();
  });

  it('clicking Generate calls invoke with generate_and_assemble', async () => {
    render(<GenerateStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /开始生成/ }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'generate_and_assemble',
        expect.objectContaining({ basePrompt: 'anime chibi girl' })
      );
    });
  });

  it('shows progress text while generating', async () => {
    // Keep invoke pending so generating state persists
    let resolve: () => void;
    mockInvoke.mockReturnValue(new Promise<void>((res) => { resolve = res; }));

    render(<GenerateStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /开始生成/ }));

    await waitFor(() => {
      expect(screen.getByText(/正在生成/)).toBeTruthy();
    });

    resolve!();
  });

  it('shows Next button after generation completes', async () => {
    mockInvoke.mockResolvedValue(undefined);
    render(<GenerateStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /开始生成/ }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /下一步/ })).toBeTruthy();
    });
  });

  it('calls onBack when Back is clicked', () => {
    const onBack = vi.fn();
    render(<GenerateStep {...defaultProps} onBack={onBack} />);
    fireEvent.click(screen.getByRole('button', { name: /上一步/ }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
