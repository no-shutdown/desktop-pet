import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

vi.mock('../../../../lib/claude-vision', () => ({
  analyzePhoto: vi.fn().mockResolvedValue('anime chibi girl, black hair'),
}));

import AnalyzeStep from '../AnalyzeStep';
import { analyzePhoto } from '../../../../lib/claude-vision';

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
    expect(screen.getByPlaceholderText(/anthropic api key/i)).toBeTruthy();
  });

  it('renders the prompt textarea', () => {
    render(<AnalyzeStep {...defaultProps} />);
    expect(screen.getByRole('textbox', { name: /character description/i })).toBeTruthy();
  });

  it('Analyze button calls analyzePhoto with correct args', async () => {
    render(<AnalyzeStep {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText(/anthropic api key/i), {
      target: { value: 'sk-ant-test' },
    });
    fireEvent.click(screen.getByRole('button', { name: /analyze/i }));
    await waitFor(() => {
      expect(analyzePhoto).toHaveBeenCalledWith('data:image/jpeg;base64,fake', 'sk-ant-test');
    });
  });

  it('fills prompt textarea after successful analysis', async () => {
    render(<AnalyzeStep {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText(/anthropic api key/i), {
      target: { value: 'sk-ant-test' },
    });
    fireEvent.click(screen.getByRole('button', { name: /analyze/i }));
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
    fireEvent.click(screen.getByRole('button', { name: /next/i }));
    expect(onNext).toHaveBeenCalledWith('my custom prompt');
  });

  it('Next button is disabled when prompt is empty', () => {
    render(<AnalyzeStep {...defaultProps} />);
    const btn = screen.getByRole('button', { name: /next/i }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('calls onBack when Back is clicked', () => {
    const onBack = vi.fn();
    render(<AnalyzeStep {...defaultProps} onBack={onBack} />);
    fireEvent.click(screen.getByRole('button', { name: /back/i }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
