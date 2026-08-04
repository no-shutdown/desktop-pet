import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import UploadStep from '../UploadStep';

describe('UploadStep', () => {
  const onNext = vi.fn();

  it('renders a file input', () => {
    render(<UploadStep onNext={onNext} />);
    expect(screen.getByTestId('file-input')).toBeTruthy();
  });

  it('renders drop zone text', () => {
    render(<UploadStep onNext={onNext} />);
    expect(screen.getByText(/drag.*drop|upload/i)).toBeTruthy();
  });

  it('shows preview after file selected', () => {
    render(<UploadStep onNext={onNext} />);
    const input = screen.getByTestId('file-input') as HTMLInputElement;

    const file = new File(['fake'], 'photo.jpg', { type: 'image/jpeg' });
    Object.defineProperty(input, 'files', { value: [file] });

    // Mock FileReader
    const mockReadAsDataURL = vi.fn();
    const mockReader = {
      readAsDataURL: mockReadAsDataURL,
      onload: null as unknown as ((e: ProgressEvent) => void) | null,
      result: 'data:image/jpeg;base64,fake',
    };
    vi.spyOn(globalThis, 'FileReader').mockImplementation(() => mockReader as unknown as FileReader);

    fireEvent.change(input);

    // Trigger the onload
    mockReader.onload?.({ target: mockReader } as unknown as ProgressEvent);

    expect(screen.getByAltText('preview')).toBeTruthy();
  });

  it('Next button is disabled until a photo is selected', () => {
    render(<UploadStep onNext={onNext} />);
    const btn = screen.getByRole('button', { name: /next/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });
});
