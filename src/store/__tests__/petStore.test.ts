import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { usePetStore } from '../petStore';
import type { Pet, SpriteStateInfo } from '../../types/pet';

const META: SpriteStateInfo = { cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200 };

const makePet = (): Pet => ({
  id: 'test',
  name: 'Test',
  states: {
    idle:    META,
    sleeping: { cols: 2, rows: 3, frameCount: 6, frameW: 128, frameH: 128, delayMs: 150 },
    waving:  { ...META, delayMs: 150 },
    working: META,
  },
  createdAt: '2026-08-03T10:00:00Z',
  prompt: 'chibi',
});

describe('petStore', () => {
  beforeEach(() => {
    usePetStore.setState({ petState: 'idle', activePet: null });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts with idle state and no active pet', () => {
    const { petState, activePet } = usePetStore.getState();
    expect(petState).toBe('idle');
    expect(activePet).toBeNull();
  });

  it('setPetState changes state immediately', () => {
    usePetStore.getState().setPetState('working');
    expect(usePetStore.getState().petState).toBe('working');
  });

  it('non-idle state auto-resets to idle after 3s', () => {
    vi.useFakeTimers();
    usePetStore.getState().setPetState('waving');
    expect(usePetStore.getState().petState).toBe('waving');
    vi.advanceTimersByTime(3001);
    expect(usePetStore.getState().petState).toBe('idle');
  });

  it('idle state does NOT auto-reset', () => {
    vi.useFakeTimers();
    usePetStore.getState().setPetState('idle');
    vi.advanceTimersByTime(5000);
    expect(usePetStore.getState().petState).toBe('idle');
  });

  it('setActivePet updates activePet', () => {
    const pet = makePet();
    usePetStore.getState().setActivePet(pet);
    expect(usePetStore.getState().activePet?.id).toBe('test');
  });
});
