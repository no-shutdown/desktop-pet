import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Pet, SpriteStateInfo } from '../../types/pet';

const mockInvoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

import { petCommands } from '../pet-commands';

const META: SpriteStateInfo = { cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200 };

const mockPet: Pet = {
  id: 'test-id',
  name: 'Test',
  states: { idle: META, sleeping: META, waving: META, working: META },
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
