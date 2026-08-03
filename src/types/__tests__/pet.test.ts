import { describe, it, expect } from 'vitest';
import { PET_STATES } from '../pet';
import type { Pet } from '../pet';

describe('Pet types', () => {
  it('PET_STATES has all four states', () => {
    expect(PET_STATES).toContain('idle');
    expect(PET_STATES).toContain('walking');
    expect(PET_STATES).toContain('waving');
    expect(PET_STATES).toContain('working');
    expect(PET_STATES).toHaveLength(4);
  });

  it('Pet object matches expected shape', () => {
    const pet: Pet = {
      id: 'abc',
      name: 'Test',
      frames: { idle: 'i.gif', walking: 'w.gif', waving: 'wv.gif', working: 'wk.gif' },
      createdAt: '2026-08-03T10:00:00Z',
      prompt: 'chibi',
    };
    expect(pet.id).toBe('abc');
  });
});
