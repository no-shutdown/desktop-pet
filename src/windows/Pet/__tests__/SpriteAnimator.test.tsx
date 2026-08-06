import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, render, waitFor } from '@testing-library/react';
import SpriteAnimator from '../SpriteAnimator';
import type { SpriteStateInfo } from '../../../types/pet';

const mockCtx = {
  clearRect: vi.fn(),
  drawImage: vi.fn(),
};

const rafQueue: FrameRequestCallback[] = [];
const mockRequestAnimationFrame = vi.fn((callback: FrameRequestCallback) => {
  rafQueue.push(callback);
  return rafQueue.length;
});
const mockCancelAnimationFrame = vi.fn();

class MockImage {
  onload: (() => void) | null = null;

  set src(_value: string) {
    queueMicrotask(() => this.onload?.());
  }
}

beforeEach(() => {
  rafQueue.length = 0;
  mockCtx.clearRect.mockReset();
  mockCtx.drawImage.mockReset();
  mockRequestAnimationFrame.mockClear();
  mockCancelAnimationFrame.mockClear();
  vi.stubGlobal('Image', MockImage);
  vi.stubGlobal('requestAnimationFrame', mockRequestAnimationFrame);
  vi.stubGlobal('cancelAnimationFrame', mockCancelAnimationFrame);
  Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
    configurable: true,
    value: vi.fn().mockReturnValue(mockCtx),
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

const META: SpriteStateInfo = {
  cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200,
};

async function drawAllFrames(meta: SpriteStateInfo) {
  render(<SpriteAnimator sheetSrc="/test.png" meta={meta} />);
  await waitFor(() => expect(rafQueue.length).toBeGreaterThan(0));

  for (let frame = 0; frame < meta.frameCount; frame += 1) {
    const callback = rafQueue.shift()!;
    act(() => callback((frame + 1) * meta.delayMs));
  }
}

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

  it('draws frame zero as soon as the sheet loads', async () => {
    render(<SpriteAnimator sheetSrc="/test.png" meta={META} />);

    await waitFor(() => expect(mockCtx.drawImage).toHaveBeenCalledTimes(1));

    expect(mockCtx.drawImage).toHaveBeenCalledWith(
      expect.any(MockImage),
      0,
      0,
      META.frameW,
      META.frameH,
      0,
      0,
      META.frameW,
      META.frameH,
    );
  });

  it.each([
    ['legacy two-row sheet', { cols: 4, rows: 2, frameCount: 8 }],
    ['canonical horizontal sheet', { cols: 8, rows: 1, frameCount: 8 }],
  ])('draws every frame within the %s bounds', async (_name, layout) => {
    const meta: SpriteStateInfo = {
      ...layout,
      frameW: 128,
      frameH: 128,
      delayMs: 10,
    };

    await drawAllFrames(meta);

    expect(mockCtx.drawImage).toHaveBeenCalledTimes(meta.frameCount + 1);
    for (const [image, sourceX, sourceY, sourceW, sourceH] of mockCtx.drawImage.mock.calls) {
      expect(image).toBeInstanceOf(MockImage);
      expect(sourceW).toBe(meta.frameW);
      expect(sourceH).toBe(meta.frameH);
      expect(sourceX).toBeGreaterThanOrEqual(0);
      expect(sourceX).toBeLessThan(meta.cols * meta.frameW);
      expect(sourceY).toBeGreaterThanOrEqual(0);
      expect(sourceY).toBeLessThan(meta.rows * meta.frameH);
    }
  });
});
