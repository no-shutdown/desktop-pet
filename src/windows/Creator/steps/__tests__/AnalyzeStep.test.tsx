import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

vi.mock('../../../../lib/vision', () => ({
  analyzePhotoWithSettings: vi.fn().mockResolvedValue('anime chibi girl, black hair'),
}));
vi.mock('../../../../lib/settings', () => ({
  loadSettings: vi.fn().mockReturnValue({
    visionProvider: 'anthropic',
    visionApiKey: 'sk-ant-test',
    visionModel: 'claude-opus-4-5',
    imageProvider: 'pollinations',
    imageApiKey: '',
    imageModel: '',
    localSdUrl: '',
  }),
  saveSettings: vi.fn(),
  getVisionModels: vi.fn().mockReturnValue([]),
  defaultVisionModel: vi.fn().mockReturnValue('claude-opus-4-5'),
}));

import AnalyzeStep from '../AnalyzeStep';
import { analyzePhotoWithSettings } from '../../../../lib/vision';

describe('AnalyzeStep', () => {
  const defaultProps = {
    photoDataUrl: 'data:image/jpeg;base64,fake',
    initialPrompt: '',
    onNext: vi.fn(),
    onBack: vi.fn(),
  };

  beforeEach(() => vi.clearAllMocks());

  it('shows the uploaded photo preview', () => {
    render(<AnalyzeStep {...defaultProps} />);
    expect(screen.getByAltText('reference')).toBeTruthy();
  });

  it('renders the API key input', () => {
    render(<AnalyzeStep {...defaultProps} />);
    expect(screen.getByPlaceholderText(/API Key/)).toBeTruthy();
  });

  it('renders the prompt textarea', () => {
    render(<AnalyzeStep {...defaultProps} />);
    expect(screen.getByRole('textbox', { name: /character description/i })).toBeTruthy();
  });

  it('Analyze button calls analyzePhoto with correct args', async () => {
    render(<AnalyzeStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /AI 分析照片/ }));
    await waitFor(() => {
      expect(analyzePhotoWithSettings).toHaveBeenCalled();
    });
  });

  it('fills prompt textarea after successful analysis', async () => {
    render(<AnalyzeStep {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /AI 分析照片/ }));
    await waitFor(() => {
      const textarea = screen.getByRole('textbox', { name: /character description/i }) as HTMLTextAreaElement;
      expect(textarea.value).toBe('anime chibi girl, black hair');
    });
  });

  it('Next button passes prompt to onNext', async () => {
    const onNext = vi.fn();
    render(<AnalyzeStep {...defaultProps} onNext={onNext} />);
    const textarea = screen.getByRole('textbox', { name: /character description/i });
    fireEvent.change(textarea, { target: { value: 'my custom prompt' } });
    fireEvent.click(screen.getByRole('button', { name: /下一步/ }));
    expect(onNext).toHaveBeenCalledWith('my custom prompt');
  });

  it('Next button is disabled when prompt is empty', () => {
    render(<AnalyzeStep {...defaultProps} />);
    const btn = screen.getByRole('button', { name: /下一步/ }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('calls onBack when Back is clicked', () => {
    const onBack = vi.fn();
    render(<AnalyzeStep {...defaultProps} onBack={onBack} />);
    fireEvent.click(screen.getByRole('button', { name: /上一步/ }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
