import { describe, it, expect } from 'vitest';
import { PET_STATES } from '../pet';
import type { Pet, SpriteStateInfo } from '../pet';

const META: SpriteStateInfo = {
  cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200,
};

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
      states: { idle: META, walking: META, waving: META, working: META },
      createdAt: '2026-08-03T10:00:00Z',
      prompt: 'chibi',
    };
    expect(pet.id).toBe('abc');
    expect(pet.states.idle.frameCount).toBe(4);
    expect(pet.states.walking.delayMs).toBe(200);
  });
});
