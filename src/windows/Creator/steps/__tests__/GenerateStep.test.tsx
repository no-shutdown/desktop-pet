import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

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
    onNext: vi.fn(),
    onBack: vi.fn(),
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

  it('calls generate_base_preview with the current backend payload', async () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: 'Generate Base' }));

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
      });
    });

    expect(mockInvoke.mock.calls.some(([name]) => name === 'generate_and_assemble')).toBe(false);
  });

  it('displays the Base preview and confirms only the Base result', async () => {
    const onNext = vi.fn();
    render(<GenerateStep {...defaultProps} onNext={onNext} runId="run-1" />);

    fireEvent.click(screen.getByRole('button', { name: 'Generate Base' }));
    expect(await screen.findByAltText('canonical base preview')).toHaveAttribute(
      'src',
      'data:image/png;base64,BASE',
    );

    fireEvent.click(screen.getByRole('button', { name: 'Confirm Base' }));

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

    fireEvent.click(screen.getByRole('button', { name: 'Generate Base' }));
    expect(await screen.findByText('provider failed')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Retry Base' }));

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

    fireEvent.click(screen.getByRole('button', { name: 'Generate Base' }));

    expect(await screen.findByText('no provider fallback')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Retry Base' })).toBeTruthy();
    expect(mockInvoke.mock.calls.every(([name]) => name === 'generate_base_preview')).toBe(true);
  });

  it('stores model changes using the canonical settings fields', () => {
    render(<GenerateStep {...defaultProps} runId="run-1" />);

    fireEvent.change(screen.getByLabelText('Base model'), {
      target: { value: 'other-base-model' },
    });

    expect(mockSaveSettings).toHaveBeenLastCalledWith(expect.objectContaining({
      imageBaseModel: 'other-base-model',
      imageModel: 'other-base-model',
    }));
  });
});
