import { describe, it, expect, vi, beforeAll, afterEach } from 'vitest';
import { render } from '@testing-library/react';
import SpriteAnimator from '../SpriteAnimator';
import type { SpriteStateInfo } from '../../../types/pet';

const mockCtx = {
  clearRect: vi.fn(),
  drawImage: vi.fn(),
};

beforeAll(() => {
  HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue(mockCtx);
  vi.stubGlobal('requestAnimationFrame', vi.fn());
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
});

afterEach(() => {
  vi.clearAllMocks();
});

const META: SpriteStateInfo = {
  cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200,
};

describe('SpriteAnimator', () => {
  it('renders a canvas element', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} />
    );
    expect(container.querySelector('canvas')).not.toBeNull();
  });

  it('uses displayW/displayH for canvas dimensions', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} displayW={200} displayH={200} />
    );
    const canvas = container.querySelector('canvas')!;
    expect(canvas.width).toBe(200);
    expect(canvas.height).toBe(200);
  });

  it('defaults canvas size to frameW/frameH when display props omitted', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} />
    );
    const canvas = container.querySelector('canvas')!;
    expect(canvas.width).toBe(128);
    expect(canvas.height).toBe(128);
  });

  it('applies pixelated image rendering', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} />
    );
    const canvas = container.querySelector('canvas')!;
    expect(canvas.style.imageRendering).toBe('pixelated');
  });
});
