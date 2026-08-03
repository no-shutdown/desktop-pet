import { describe, it, expect, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { petCommands } from '../pet-commands';

describe('petCommands', () => {
  it('exports save, list, delete', () => {
    expect(typeof petCommands.save).toBe('function');
    expect(typeof petCommands.list).toBe('function');
    expect(typeof petCommands.delete).toBe('function');
  });
});
