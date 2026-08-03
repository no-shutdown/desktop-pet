import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Pet } from '../../types/pet';

const mockInvoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

import { petCommands } from '../pet-commands';

const mockPet: Pet = {
  id: 'test-id',
  name: 'Test',
  frames: { idle: 'i.gif', walking: 'w.gif', waving: 'wv.gif', working: 'wk.gif' },
  createdAt: '2026-08-03T10:00:00Z',
  prompt: 'chibi',
};

describe('petCommands', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(undefined);
  });

  it('save calls invoke with save_pet and pet arg', async () => {
    await petCommands.save(mockPet);
    expect(mockInvoke).toHaveBeenCalledWith('save_pet', { pet: mockPet });
  });

  it('list calls invoke with list_pets', async () => {
    mockInvoke.mockResolvedValue([]);
    await petCommands.list();
    expect(mockInvoke).toHaveBeenCalledWith('list_pets');
  });

  it('delete calls invoke with delete_pet and petId arg', async () => {
    await petCommands.delete('test-id');
    expect(mockInvoke).toHaveBeenCalledWith('delete_pet', { petId: 'test-id' });
  });
});
