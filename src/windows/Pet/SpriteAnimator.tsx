import { useEffect, useRef } from 'react';
import type { SpriteStateInfo } from '../../types/pet';

interface SpriteAnimatorProps {
  sheetSrc: string;
  meta: SpriteStateInfo;
  displayW?: number;
  displayH?: number;
}

export default function SpriteAnimator({ sheetSrc, meta, displayW, displayH }: SpriteAnimatorProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const frameRef = useRef(0);
  const lastTsRef = useRef(0);
  const rafRef = useRef(0);

  const w = displayW ?? meta.frameW;
  const h = displayH ?? meta.frameH;

  // Destructure to primitives so the effect only re-runs when values actually change,
  // not every time the parent renders with a new meta object reference.
  const { cols, frameCount, frameW, frameH, delayMs } = meta;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d')!;
    if (!ctx) return;

    frameRef.current = 0;
    lastTsRef.current = 0;
    cancelAnimationFrame(rafRef.current);

    const img = new Image();

    img.onload = () => {
      function tick(ts: number) {
        if (ts - lastTsRef.current >= delayMs) {
          const col = frameRef.current % cols;
          const row = Math.floor(frameRef.current / cols);
          ctx.clearRect(0, 0, w, h);
          ctx.drawImage(img, col * frameW, row * frameH, frameW, frameH, 0, 0, w, h);
          frameRef.current = (frameRef.current + 1) % frameCount;
          lastTsRef.current = ts;
        }
        rafRef.current = requestAnimationFrame(tick);
      }
      rafRef.current = requestAnimationFrame(tick);
    };

    img.src = sheetSrc;

    return () => {
      cancelAnimationFrame(rafRef.current);
      img.onload = null;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sheetSrc, cols, frameCount, frameW, frameH, delayMs, w, h]);

  return (
    <canvas
      ref={canvasRef}
      width={w}
      height={h}
      style={{ display: 'block', width: `${w}px`, height: `${h}px`, imageRendering: 'pixelated' }}
    />
  );
}
