import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

vi.mock('../../../../lib/vision', () => ({
  analyzePhotoWithSettings: vi.fn().mockResolvedValue('年轻女性，圆脸，黑色长直发，气质温柔'),
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
    expect(screen.getByPlaceholderText(/API 密钥/)).toBeTruthy();
  });

  it('renders the prompt textarea', () => {
    render(<AnalyzeStep {...defaultProps} />);
    expect(screen.getByRole('textbox', { name: /角色描述/ })).toBeTruthy();
  });

  it('uses Chinese text for the prompt input', () => {
    render(<AnalyzeStep {...defaultProps} />);

    expect(screen.getByRole('textbox', { name: /角色描述/ })).toHaveAttribute(
      'placeholder',
      '例：年轻女性，圆脸，黑色长直发，五官柔和，气质温柔，穿米色刺绣上衣……'
    );
  });

  it('shows Chinese labels for the source style choices', () => {
    render(<AnalyzeStep {...defaultProps} />);

    const realisticRadio = screen.getByRole('radio', { name: /真实人物照片/ }) as HTMLInputElement;
    expect(realisticRadio).toBeTruthy();
    expect(screen.getByRole('radio', { name: /卡通 \/ 插画作品/ })).toBeTruthy();
    expect(screen.getByText('参考图风格')).toBeTruthy();
    expect(screen.getByText('转换为可爱的 2D Q 版形象')).toBeTruthy();
    expect(screen.getByText('保留原始画风')).toBeTruthy();
    expect(realisticRadio.checked).toBe(true);
  });

  it('groups source style choices under an accessible fieldset', () => {
    render(<AnalyzeStep {...defaultProps} />);

    expect(screen.getByRole('group', { name: /参考图风格/ })).toBeTruthy();
  });

  it('initializes the source style from the optional initial value', () => {
    render(<AnalyzeStep {...defaultProps} initialSourceStyle="stylized" />);

    expect((screen.getByRole('radio', { name: /卡通 \/ 插画作品/ }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole('radio', { name: /真实人物照片/ }) as HTMLInputElement).checked).toBe(false);
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
      const textarea = screen.getByRole('textbox', { name: /角色描述/ }) as HTMLTextAreaElement;
      expect(textarea.value).toBe('年轻女性，圆脸，黑色长直发，气质温柔');
    });
  });

  it('Next button passes prompt to onNext', async () => {
    const onNext = vi.fn();
    render(<AnalyzeStep {...defaultProps} onNext={onNext} />);
    const textarea = screen.getByRole('textbox', { name: /角色描述/ });
    fireEvent.change(textarea, { target: { value: 'my custom prompt' } });
    fireEvent.click(screen.getByRole('button', { name: /下一步/ }));
    expect(onNext).toHaveBeenCalledWith('my custom prompt', 'realistic');
  });

  it('passes the selected source style with the description', () => {
    const onNext = vi.fn();
    render(<AnalyzeStep {...defaultProps} onNext={onNext} />);
    fireEvent.change(screen.getByRole('textbox', { name: /角色描述/ }), {
      target: { value: 'pink cartoon character' },
    });
    fireEvent.click(screen.getByRole('radio', { name: /卡通 \/ 插画作品/ }));
    fireEvent.click(screen.getByRole('button', { name: /下一步/ }));

    expect(onNext).toHaveBeenCalledWith('pink cartoon character', 'stylized');
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

  it('passes the current source style to onBack before leaving', () => {
    const onBack = vi.fn();
    render(<AnalyzeStep {...defaultProps} onBack={onBack} />);

    fireEvent.click(screen.getByRole('radio', { name: /卡通 \/ 插画作品/ }));
    fireEvent.click(screen.getByRole('button', { name: /上一步/ }));

    expect(onBack).toHaveBeenCalledWith('stylized');
  });
});
