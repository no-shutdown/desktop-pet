import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

const { mockInvoke } = vi.hoisted(() => ({ mockInvoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));
vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn().mockResolvedValue('C:\\AppData\\Roaming\\desktop-pet'),
  join: vi.fn((...parts: string[]) => Promise.resolve(parts.join('/'))),
}));

import SaveStep from '../SaveStep';

describe('SaveStep', () => {
  const defaultProps = {
    petId: 'abc-123',
    prompt: 'anime chibi girl',
    onComplete: vi.fn(),
    onBack: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
  });

  it('renders pet name input', () => {
    render(<SaveStep {...defaultProps} />);
    expect(screen.getByPlaceholderText(/name/i)).toBeTruthy();
  });

  it('Save button is disabled when name is empty', () => {
    render(<SaveStep {...defaultProps} />);
    const btn = screen.getByRole('button', { name: /save/i }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('Save button enables after typing a name', () => {
    render(<SaveStep {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText(/name/i), { target: { value: 'Chibi' } });
    const btn = screen.getByRole('button', { name: /save/i }) as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
  });

  it('clicking Save calls invoke save_pet with correct data', async () => {
    render(<SaveStep {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText(/name/i), { target: { value: 'Chibi' } });
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('save_pet', expect.objectContaining({
        pet: expect.objectContaining({ id: 'abc-123', name: 'Chibi', prompt: 'anime chibi girl' }),
      }));
    });
  });

  it('calls onBack when Back is clicked', () => {
    const onBack = vi.fn();
    render(<SaveStep {...defaultProps} onBack={onBack} />);
    fireEvent.click(screen.getByRole('button', { name: /back/i }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
