import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act, cleanup } from '@testing-library/react';

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
  apiKeyForProvider: (settings: { imageApiKey: string; wanxiangApiKey: string }, provider: string) => (
    provider === 'wanxiang' ? settings.wanxiangApiKey : provider === 'localsd' ? '' : settings.imageApiKey
  ),
  baseModelForProvider: (settings: { imageBaseModel: string; wanxiangBaseModel: string }, provider: string) => (
    provider === 'wanxiang' ? settings.wanxiangBaseModel : provider === 'localsd' ? '' : settings.imageBaseModel
  ),
  rowModelForProvider: (settings: { imageReferenceModel: string; wanxiangEditModel: string }, provider: string) => (
    provider === 'wanxiang' ? settings.wanxiangEditModel : settings.imageReferenceModel
  ),
  SILICONFLOW_BASE_MODELS: [
    { value: 'base-model', label: 'Base model' },
    { value: 'other-base-model', label: 'Other base model' },
  ],
  SILICONFLOW_REFERENCE_MODELS: [
    { value: 'reference-model', label: 'Reference model' },
  ],
  SILICONFLOW_MODELS: [
    { value: 'base-model', label: 'Base model' },
  ],
}));

import GenerateStep from '../GenerateStep';

describe('GenerateStep', () => {
  const defaultProps = {
    prompt: 'anime chibi girl',
    referenceDataUrl: 'data:image/jpeg;base64,REF',
    sourceStyle: 'realistic' as const,
    styleReferenceDataUrl: null,
    onNext: vi.fn(),
    onBack: vi.fn(),
    onStyleReferenceChange: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockResolvedValue({
      runId: 'run-1',
      dataUrl: 'data:image/png;base64,BASE',
      chromaKey: '#FF00FF',
    });
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('calls generate_base_preview with the current backend payload', async () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: '生成基础图像' }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('generate_base_preview', {
        runId: 'run-1',
        basePrompt: 'anime chibi girl',
        referenceDataUrl: 'data:image/jpeg;base64,REF',
        imageProvider: 'siliconflow',
        imageApiKey: 'image-api-key',
        baseModel: 'base-model',
        referenceModel: 'reference-model',
        localSdUrl: 'http://localhost:7860',
        denoisingStrength: 0.55,
        sourceStyle: 'realistic',
        styleReferenceDataUrl: null,
      });
    });

    expect(mockInvoke.mock.calls.some(([name]) => name === 'generate_and_assemble')).toBe(false);
  });

  it('shows an optional style-reference upload control', () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);

    expect(screen.getByText('风格参考图（可选）')).toBeTruthy();
    expect(screen.getByTestId('style-reference-file-input')).toHaveAttribute(
      'accept', 'image/jpeg,image/png,image/webp',
    );
    expect(screen.getByText('只参考画风，不复制图片中的人物内容')).toBeTruthy();
  });

  it('opens the style-reference chooser from the keyboard-accessible upload control', () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const clickSpy = vi.spyOn(input, 'click');
    const uploadButton = screen.getByRole('button', { name: '上传一张参考画风的图片（可选）' });

    expect(input.style.display).not.toBe('none');
    fireEvent.keyDown(uploadButton, { key: 'Enter' });
    fireEvent.keyDown(uploadButton, { key: ' ' });

    expect(clickSpy).toHaveBeenCalledTimes(2);
  });

  it('disables base generation while a style-reference file is being read', () => {
    const readers: MockFileReader[] = [];
    class MockFileReader {
      onload: ((event: ProgressEvent) => void) | null = null;
      onerror: ((event: ProgressEvent) => void) | null = null;
      onabort: ((event: ProgressEvent) => void) | null = null;
      result: string | null = null;
      readAsDataURL = vi.fn();
      abort = vi.fn();
      constructor() { readers.push(this); }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const file = new File(['style'], 'style.png', { type: 'image/png' });
    Object.defineProperty(input, 'files', { configurable: true, value: [file] });

    act(() => {
      fireEvent.change(input);
    });
    const generateButton = screen.getByRole('button', { name: '生成基础图像' });
    expect(generateButton).toBeDisabled();
    expect(screen.getByText('正在读取风格参考图，请稍候…')).toBeTruthy();
    fireEvent.click(generateButton);
    expect(mockInvoke).not.toHaveBeenCalled();

    act(() => {
      readers[0].result = 'data:image/png;base64,STYLE';
      readers[0].onload?.({ target: readers[0] } as unknown as ProgressEvent);
    });
    expect(generateButton).not.toBeDisabled();
  });

  it('reads, previews, and removes a style reference image', () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const file = new File(['fake'], 'style.png', { type: 'image/png' });
    Object.defineProperty(input, 'files', {
      value: [file],
    });
    let reader: {
      onload: ((event: ProgressEvent) => void) | null;
      result: string;
      readAsDataURL: ReturnType<typeof vi.fn>;
    } | null = null;
    class MockFileReader {
      onload: ((event: ProgressEvent) => void) | null = null;
      result = 'data:image/png;base64,STYLE';
      readAsDataURL = vi.fn();
      constructor() { reader = this; }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    fireEvent.change(input);
    expect(reader).not.toBeNull();
    expect(reader!.readAsDataURL).toHaveBeenCalledWith(file);
    act(() => {
      reader?.onload?.({ target: reader } as unknown as ProgressEvent);
    });
    expect(screen.getByAltText('风格参考图预览')).toHaveAttribute(
      'src', 'data:image/png;base64,STYLE',
    );
    expect(defaultProps.onStyleReferenceChange).toHaveBeenLastCalledWith(
      'data:image/png;base64,STYLE',
    );

    fireEvent.click(screen.getByRole('button', { name: '移除风格参考图' }));
    expect(screen.queryByAltText('风格参考图预览')).toBeNull();
    expect(defaultProps.onStyleReferenceChange).toHaveBeenLastCalledWith(null);
  });

  it('aborts and ignores stale or unmounted style-reference readers', () => {
    const onStyleReferenceChange = vi.fn();
    const readers: MockFileReader[] = [];
    class MockFileReader {
      onload: ((event: ProgressEvent) => void) | null = null;
      onerror: ((event: ProgressEvent) => void) | null = null;
      onabort: ((event: ProgressEvent) => void) | null = null;
      result: string | null = null;
      readAsDataURL = vi.fn();
      abort = vi.fn();
      constructor() { readers.push(this); }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    const { unmount } = render(
      <GenerateStep {...defaultProps} onStyleReferenceChange={onStyleReferenceChange} runId="run-1" />,
    );
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const firstFile = new File(['first'], 'first.png', { type: 'image/png' });
    const secondFile = new File(['second'], 'second.png', { type: 'image/png' });

    Object.defineProperty(input, 'files', { configurable: true, value: [firstFile] });
    fireEvent.change(input);
    Object.defineProperty(input, 'files', { configurable: true, value: [secondFile] });
    fireEvent.change(input);

    expect(readers).toHaveLength(2);
    expect(readers[0].abort).toHaveBeenCalledTimes(1);

    act(() => {
      readers[0].result = 'data:image/png;base64,STALE';
      readers[0].onload?.({ target: readers[0] } as unknown as ProgressEvent);
      readers[0].onerror?.({ target: readers[0] } as unknown as ProgressEvent);
      readers[0].onabort?.({ target: readers[0] } as unknown as ProgressEvent);
    });
    expect(onStyleReferenceChange).not.toHaveBeenCalled();
    expect(screen.queryByAltText('风格参考图预览')).toBeNull();

    act(() => {
      unmount();
    });
    expect(readers[1].abort).toHaveBeenCalledTimes(1);
    act(() => {
      readers[1].result = 'data:image/png;base64,AFTER-UNMOUNT';
      readers[1].onload?.({ target: readers[1] } as unknown as ProgressEvent);
    });
    expect(onStyleReferenceChange).not.toHaveBeenCalled();
  });

  it('shows a retry message for style-reference reader errors and clears it', () => {
    const readers: MockFileReader[] = [];
    class MockFileReader {
      onload: ((event: ProgressEvent) => void) | null = null;
      onerror: ((event: ProgressEvent) => void) | null = null;
      onabort: ((event: ProgressEvent) => void) | null = null;
      result: string | null = null;
      readAsDataURL = vi.fn();
      abort = vi.fn();
      constructor() { readers.push(this); }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const firstFile = new File(['first'], 'first.png', { type: 'image/png' });
    const secondFile = new File(['second'], 'second.png', { type: 'image/png' });

    Object.defineProperty(input, 'files', { configurable: true, value: [firstFile] });
    fireEvent.change(input);
    act(() => {
      readers[0].onerror?.({ target: readers[0] } as unknown as ProgressEvent);
    });
    expect(screen.getByRole('alert')).toHaveTextContent('风格参考图读取失败，请重试。');

    Object.defineProperty(input, 'files', { configurable: true, value: [secondFile] });
    fireEvent.change(input);
    expect(screen.queryByRole('alert')).toBeNull();

    act(() => {
      readers[1].result = 'data:image/png;base64,STYLE';
      readers[1].onload?.({ target: readers[1] } as unknown as ProgressEvent);
    });
    fireEvent.click(screen.getByRole('button', { name: '移除风格参考图' }));
    expect(screen.queryByRole('alert')).toBeNull();
    act(() => {
      readers[1].onabort?.({ target: readers[1] } as unknown as ProgressEvent);
    });
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('clears a failed file input so the same style reference can be retried', () => {
    const readers: MockFileReader[] = [];
    class MockFileReader {
      onload: ((event: ProgressEvent) => void) | null = null;
      onerror: ((event: ProgressEvent) => void) | null = null;
      onabort: ((event: ProgressEvent) => void) | null = null;
      result: string | null = null;
      readAsDataURL = vi.fn();
      abort = vi.fn();
      constructor() { readers.push(this); }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const file = new File(['style'], 'style.png', { type: 'image/png' });
    const selectFile = () => {
      Object.defineProperty(input, 'files', { configurable: true, value: [file] });
      Object.defineProperty(input, 'value', {
        configurable: true,
        writable: true,
        value: 'C:\\fakepath\\style.png',
      });
      fireEvent.change(input);
    };

    selectFile();
    act(() => {
      readers[0].onerror?.({ target: readers[0] } as unknown as ProgressEvent);
    });
    expect(input.value).toBe('');
    expect(screen.getByRole('alert')).toHaveTextContent('风格参考图读取失败，请重试。');

    selectFile();
    expect(readers).toHaveLength(2);
    expect(readers[1].readAsDataURL).toHaveBeenCalledWith(file);
    act(() => {
      readers[1].result = 'data:image/png;base64,STYLE';
      readers[1].onload?.({ target: readers[1] } as unknown as ProgressEvent);
    });
  });

  it('rejects invalid style-reference files with a retryable Chinese error', () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const file = new File(['not an image'], 'style.txt', { type: 'text/plain' });
    Object.defineProperty(input, 'files', { configurable: true, value: [file] });
    Object.defineProperty(input, 'value', {
      configurable: true,
      writable: true,
      value: 'C:\\fakepath\\style.txt',
    });

    fireEvent.change(input);

    expect(input.value).toBe('');
    expect(screen.getByRole('alert')).toHaveTextContent('仅支持 JPG、PNG 或 WEBP 图片，请重试。');
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('passes a read style reference image in the base request', async () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);
    const input = screen.getByTestId('style-reference-file-input') as HTMLInputElement;
    const file = new File(['fake'], 'style.png', { type: 'image/png' });
    Object.defineProperty(input, 'files', {
      value: [file],
    });
    let reader: {
      onload: ((event: ProgressEvent) => void) | null;
      result: string;
      readAsDataURL: ReturnType<typeof vi.fn>;
    } | null = null;
    class MockFileReader {
      onload: ((event: ProgressEvent) => void) | null = null;
      result = 'data:image/png;base64,STYLE';
      readAsDataURL = vi.fn();
      constructor() { reader = this; }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    fireEvent.change(input);
    expect(reader).not.toBeNull();
    expect(reader!.readAsDataURL).toHaveBeenCalledWith(file);
    act(() => {
      reader?.onload?.({ target: reader } as unknown as ProgressEvent);
    });
    fireEvent.click(screen.getByRole('button', { name: '生成基础图像' }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('generate_base_preview', expect.objectContaining({
        styleReferenceDataUrl: 'data:image/png;base64,STYLE',
      }));
    });
  });

  it('reports Base generation busy until the async command settles', async () => {
    let resolveInvoke!: (value: unknown) => void;
    mockInvoke.mockReturnValue(new Promise((resolve) => { resolveInvoke = resolve; }));
    const onBusyChange = vi.fn();
    render(<GenerateStep {...defaultProps} onBusyChange={onBusyChange} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: '生成基础图像' }));

    await waitFor(() => expect(onBusyChange).toHaveBeenCalledWith(true));
    resolveInvoke({ runId: 'run-1', dataUrl: 'base', chromaKey: '#FF00FF' });
    await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false));
  });

  it('displays the Base preview and confirms only the Base result', async () => {
    const onNext = vi.fn();
    render(<GenerateStep {...defaultProps} onNext={onNext} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: '生成基础图像' }));
    expect(await screen.findByAltText('canonical base preview')).toHaveAttribute(
      'src',
      'data:image/png;base64,BASE',
    );

    fireEvent.click(screen.getByRole('button', { name: '确认基础图像' }));

    expect(onNext).toHaveBeenCalledWith({
      runId: 'run-1',
      dataUrl: 'data:image/png;base64,BASE',
    });
  });

  it('retries the same Base run after a provider failure', async () => {
    mockInvoke
      .mockRejectedValueOnce(new Error('provider failed'))
      .mockResolvedValueOnce({
        runId: 'run-1',
        dataUrl: 'data:image/png;base64,BASE2',
        chromaKey: '#FF00FF',
      });

    render(<GenerateStep {...defaultProps} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: '生成基础图像' }));
    expect(await screen.findByText('provider failed')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: '重新生成' }));

    await waitFor(() => expect(mockInvoke).toHaveBeenCalledTimes(2));
    expect(mockInvoke.mock.calls.map(([name]) => name)).toEqual([
      'generate_base_preview',
      'generate_base_preview',
    ]);
    expect(mockInvoke.mock.calls[0][1]).toEqual(mockInvoke.mock.calls[1][1]);
    expect(screen.getByAltText('canonical base preview')).toHaveAttribute(
      'src',
      'data:image/png;base64,BASE2',
    );
  });

  it('keeps the error visible and does not use a fallback command', async () => {
    mockInvoke.mockRejectedValue(new Error('no provider fallback'));
    render(<GenerateStep {...defaultProps} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: '生成基础图像' }));

    expect(await screen.findByText('no provider fallback')).toBeTruthy();
    expect(screen.getByRole('button', { name: '重新生成' })).toBeTruthy();
    expect(mockInvoke.mock.calls.every(([name]) => name === 'generate_base_preview')).toBe(true);
  });

  it('stores model changes using the canonical settings fields', () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);

    fireEvent.change(screen.getByLabelText('SiliconFlow 基础模型'), {
      target: { value: 'other-base-model' },
    });

    expect(mockSaveSettings).toHaveBeenLastCalledWith(expect.objectContaining({
      imageBaseModel: 'other-base-model',
      imageModel: 'other-base-model',
    }));
  });
});
