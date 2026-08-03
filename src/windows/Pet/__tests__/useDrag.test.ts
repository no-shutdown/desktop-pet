import { describe, it, expect, vi } from 'vitest';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn().mockResolvedValue(undefined),
    outerPosition: vi.fn().mockResolvedValue({ x: 100, y: 200 }),
  }),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { renderHook } from '@testing-library/react';
import { useDrag } from '../useDrag';

describe('useDrag', () => {
  it('returns an onMouseDown handler', () => {
    const { result } = renderHook(() => useDrag());
    expect(typeof result.current.onMouseDown).toBe('function');
  });
});
