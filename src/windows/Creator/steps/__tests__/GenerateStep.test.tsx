import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

const { mockInvoke, mockListen, mockSaveSettings } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn().mockResolvedValue(() => {}),
  mockSaveSettings: vi.fn(),
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
    imageBaseModel: 'Tongyi-MAI/Z-Image-Turbo',
    imageReferenceModel: 'Qwen/Qwen-Image-Edit-2509',
    localSdUrl: '',
    localSdDenoisingStrength: 0.55,
  }),
  saveSettings: mockSaveSettings,
  SILICONFLOW_MODELS: [
    { value: 'Tongyi-MAI/Z-Image-Turbo', label: 'Z-Image-Turbo' },
    { value: 'Tongyi-MAI/Z-Image', label: 'Z-Image' },
  ],
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

  it('saves a legacy model selection to both imageModel and imageBaseModel', () => {
    const { container } = render(<GenerateStep {...defaultProps} />);

    fireEvent.click(container.querySelector('input[value="siliconflow"]')!);
    fireEvent.change(container.querySelector('select')!, {
      target: { value: 'Tongyi-MAI/Z-Image' },
    });

    expect(mockSaveSettings).toHaveBeenLastCalledWith(expect.objectContaining({
      imageModel: 'Tongyi-MAI/Z-Image',
      imageBaseModel: 'Tongyi-MAI/Z-Image',
    }));
  });

  it('calls onBack when Back is clicked', () => {
    const onBack = vi.fn();
    render(<GenerateStep {...defaultProps} onBack={onBack} />);
    fireEvent.click(screen.getByRole('button', { name: /上一步/ }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
