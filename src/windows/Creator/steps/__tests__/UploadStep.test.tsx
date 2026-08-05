import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import UploadStep from '../UploadStep';

describe('UploadStep', () => {
  const onNext = vi.fn();

  it('renders a file input', () => {
    render(<UploadStep onNext={onNext} />);
    expect(screen.getByTestId('file-input')).toBeTruthy();
  });

  it('renders drop zone text', () => {
    render(<UploadStep onNext={onNext} />);
    expect(screen.getByText(/拖拽|上传/)).toBeTruthy();
  });

  it('shows preview after file selected', () => {
    render(<UploadStep onNext={onNext} />);
    const input = screen.getByTestId('file-input') as HTMLInputElement;

    const file = new File(['fake'], 'photo.jpg', { type: 'image/jpeg' });
    Object.defineProperty(input, 'files', { value: [file] });

    // vi.stubGlobal with a class so `new FileReader()` works as a constructor
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let capturedReader: any = null;
    class MockFileReader {
      onload: ((e: ProgressEvent) => void) | null = null;
      result = 'data:image/jpeg;base64,fake';
      readAsDataURL = vi.fn();
      constructor() { capturedReader = this; }
    }
    vi.stubGlobal('FileReader', MockFileReader);

    fireEvent.change(input);
    act(() => {
      capturedReader.onload?.({ target: capturedReader } as unknown as ProgressEvent);
    });

    expect(screen.getByAltText('preview')).toBeTruthy();
    vi.unstubAllGlobals();
  });

  it('Next button is disabled until a photo is selected', () => {
    render(<UploadStep onNext={onNext} />);
    const btn = screen.getByRole('button', { name: /下一步/ });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });
});
