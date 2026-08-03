import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import type { MouseEvent } from 'react';

export function useDrag() {
  const onMouseDown = async (event: MouseEvent) => {
    if (event.button !== 0) return;
    event.preventDefault();
    const win = getCurrentWindow();
    await win.startDragging();
    const pos = await win.outerPosition();
    await invoke('save_window_position', { position: { x: pos.x, y: pos.y } });
  };

  return { onMouseDown };
}
