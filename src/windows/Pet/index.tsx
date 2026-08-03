import { useEffect, useState } from 'react';
import { getCurrentWindow, Window, PhysicalPosition } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { exit } from '@tauri-apps/plugin-process';
import type { MouseEvent } from 'react';
import GifDisplay from './GifDisplay';
import ContextMenu from './ContextMenu';
import { useDrag } from './useDrag';
import { usePetStore } from '../../store/petStore';

const FALLBACK_GIF = '/placeholder.gif';

export default function PetWindow() {
  const { onMouseDown } = useDrag();
  const { petState, setPetState, activePet } = usePetStore();
  const [menu, setMenu] = useState<{ x: number; y: number; visible: boolean }>({
    x: 0, y: 0, visible: false,
  });

  useEffect(() => {
    async function init() {
      const savedPos = await invoke<{ x: number; y: number } | null>('load_window_position');
      const win = getCurrentWindow();
      if (savedPos) {
        await win.setPosition(new PhysicalPosition(savedPos.x, savedPos.y));
      }
      await win.show();
    }
    init();
  }, []);

  const handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY, visible: true });
  };

  const handleOpenCreator = async () => {
    setMenu((prev) => ({ ...prev, visible: false }));
    const creator = await Window.getByLabel('creator');
    await creator?.show();
    await creator?.setFocus();
  };

  const gifPath = activePet ? activePet.frames[petState] : FALLBACK_GIF;

  return (
    <div
      style={{ width: 200, height: 240, background: 'transparent', overflow: 'hidden', cursor: 'grab' }}
      onMouseDown={onMouseDown}
      onMouseEnter={() => setPetState('waving')}
      onContextMenu={handleContextMenu}
    >
      <GifDisplay filePath={gifPath} />
      <ContextMenu
        x={menu.x}
        y={menu.y}
        visible={menu.visible}
        onClose={() => setMenu((prev) => ({ ...prev, visible: false }))}
        onSwitchPet={() => {/* Plan 2: open pet picker */}}
        onOpenCreator={handleOpenCreator}
        onExit={() => exit(0)}
      />
    </div>
  );
}
