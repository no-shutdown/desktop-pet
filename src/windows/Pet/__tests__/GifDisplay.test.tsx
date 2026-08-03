import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import GifDisplay from '../GifDisplay';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost${path}`,
}));

describe('GifDisplay', () => {
  it('renders an img element', () => {
    render(<GifDisplay filePath="/pets/abc/idle.gif" />);
    expect(screen.getByRole('img')).toBeTruthy();
  });

  it('converts filePath to asset:// URL', () => {
    render(<GifDisplay filePath="/pets/abc/idle.gif" />);
    const img = screen.getByRole('img') as HTMLImageElement;
    expect(img.src).toContain('asset://');
  });

  it('disables native drag on the img', () => {
    render(<GifDisplay filePath="/pets/abc/idle.gif" />);
    const img = screen.getByRole('img') as HTMLImageElement;
    expect(img.draggable).toBe(false);
  });
});
