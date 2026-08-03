import { convertFileSrc } from '@tauri-apps/api/core';

interface GifDisplayProps {
  filePath: string;
  width?: number;
  height?: number;
}

export default function GifDisplay({ filePath, width = 200, height = 240 }: GifDisplayProps) {
  const src = convertFileSrc(filePath);
  return (
    <img
      src={src}
      width={width}
      height={height}
      alt="pet"
      draggable={false}
      style={{ display: 'block', imageRendering: 'pixelated', userSelect: 'none' }}
    />
  );
}
