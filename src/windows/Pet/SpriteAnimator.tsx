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

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rawCtx = canvas.getContext('2d');
    if (!rawCtx) return;
    const ctx = rawCtx;

    frameRef.current = 0;
    lastTsRef.current = 0;
    cancelAnimationFrame(rafRef.current);

    const img = new Image();

    img.onload = () => {
      function tick(ts: number) {
        if (ts - lastTsRef.current >= meta.delayMs) {
          const col = frameRef.current % meta.cols;
          const row = Math.floor(frameRef.current / meta.cols);
          ctx.clearRect(0, 0, w, h);
          ctx.drawImage(img, col * meta.frameW, row * meta.frameH, meta.frameW, meta.frameH, 0, 0, w, h);
          frameRef.current = (frameRef.current + 1) % meta.frameCount;
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
  }, [sheetSrc, meta, w, h]);

  return (
    <canvas
      ref={canvasRef}
      width={w}
      height={h}
      style={{ imageRendering: 'pixelated', display: 'block' }}
    />
  );
}
