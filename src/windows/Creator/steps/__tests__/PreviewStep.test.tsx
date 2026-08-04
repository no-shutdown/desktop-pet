import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn().mockResolvedValue('C:\\AppData\\Roaming\\desktop-pet'),
  join: vi.fn((...parts: string[]) => Promise.resolve(parts.join('/'))),
}));
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path: string) => `asset://localhost/${path}`),
}));

import PreviewStep from '../PreviewStep';

describe('PreviewStep', () => {
  const defaultProps = {
    petId: 'abc-123',
    onNext: vi.fn(),
    onBack: vi.fn(),
  };

  it('renders four state labels', async () => {
    const { findByText } = render(<PreviewStep {...defaultProps} />);
    expect(await findByText(/idle/i)).toBeTruthy();
    expect(await findByText(/walking/i)).toBeTruthy();
    expect(await findByText(/waving/i)).toBeTruthy();
    expect(await findByText(/working/i)).toBeTruthy();
  });

  it('renders four GIF preview images', async () => {
    const { findAllByRole } = render(<PreviewStep {...defaultProps} />);
    const images = await findAllByRole('img');
    expect(images.length).toBe(4);
  });

  it('renders Next and Back buttons', () => {
    render(<PreviewStep {...defaultProps} />);
    expect(screen.getByRole('button', { name: /next/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /back/i })).toBeTruthy();
  });
});
