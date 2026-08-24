import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import type { PetState, SpriteStateInfo } from '../../../../types/pet';

const { mockInvoke } = vi.hoisted(() => ({ mockInvoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));
vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn().mockResolvedValue('C:\\AppData\\Roaming\\desktop-pet'),
  join: vi.fn((...parts: string[]) => Promise.resolve(parts.join('/'))),
}));

import SaveStep from '../SaveStep';

const STATES: Record<PetState, SpriteStateInfo> = {
  idle:    { cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200 },
  sleeping: { cols: 2, rows: 3, frameCount: 6, frameW: 128, frameH: 128, delayMs: 150 },
  acting_cute: { cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 150 },
  working: { cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200 },
};

describe('SaveStep', () => {
  const defaultProps = {
    petId: 'abc-123',
    runId: 'run-1',
    prompt: 'anime chibi girl',
    states: STATES,
    onComplete: vi.fn(),
    onBack: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
  });

  it('renders pet name input', () => {
    render(<SaveStep {...defaultProps} />);
    expect(screen.getByPlaceholderText(/宠物名称/)).toBeTruthy();
  });

  it('Save button is disabled when name is empty', () => {
    render(<SaveStep {...defaultProps} />);
    const btn = screen.getByRole('button', { name: /保存宠物/ }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('Save button enables after typing a name', () => {
    render(<SaveStep {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText(/宠物名称/), { target: { value: 'Chibi' } });
    const btn = screen.getByRole('button', { name: /保存宠物/ }) as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
  });

  it('clicking Save calls invoke save_pet_from_run with the run and correct data', async () => {
    render(<SaveStep {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText(/宠物名称/), { target: { value: 'Chibi' } });
    fireEvent.click(screen.getByRole('button', { name: /保存宠物/ }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('save_pet_from_run', expect.objectContaining({
        runId: 'run-1',
        pet: expect.objectContaining({ id: 'abc-123', name: 'Chibi', prompt: 'anime chibi girl' }),
      }));
    });
  });

  it('calls onBack when Back is clicked', () => {
    const onBack = vi.fn();
    render(<SaveStep {...defaultProps} onBack={onBack} />);
    fireEvent.click(screen.getByRole('button', { name: /上一步/ }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
