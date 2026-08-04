import { useEffect, useState, useCallback } from 'react';
import { getCurrentWindow, Window, PhysicalPosition } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { exit } from '@tauri-apps/plugin-process';
import type { MouseEvent } from 'react';
import GifDisplay from './GifDisplay';
import ContextMenu from './ContextMenu';
import SpeechBubble from './SpeechBubble';
import { useDrag } from './useDrag';
import { usePetStore } from '../../store/petStore';
import { PluginSandbox } from '../../lib/plugin-sandbox';
import { SCHEDULE_REMINDER_CODE, CLAUDE_CODE_PROGRESS_CODE } from '../../lib/bundled-plugins';
import type { PetState } from '../../types/pet';

const FALLBACK_GIF = '/placeholder.gif';

export default function PetWindow() {
  const { onMouseDown } = useDrag();
  const { petState, setPetState, activePet } = usePetStore();
  const [menu, setMenu] = useState<{ x: number; y: number; visible: boolean }>({
    x: 0, y: 0, visible: false,
  });
  const [bubbleText, setBubbleText] = useState<string | null>(null);

  const handleHideBubble = useCallback(() => setBubbleText(null), []);

  useEffect(() => {
    let cleanupFn: (() => void) | undefined;

    async function init() {
      // Restore window position
      const savedPos = await invoke<{ x: number; y: number } | null>('load_window_position');
      const win = getCurrentWindow();
      if (savedPos) {
        await win.setPosition(new PhysicalPosition(savedPos.x, savedPos.y));
      }
      await win.show();

      // Initialize plugin sandbox
      const sandbox = new PluginSandbox(
        (state: PetState) => setPetState(state),
        (text: string) => setBubbleText(text),
      );

      // Load bundled plugins
      sandbox.loadPlugin(SCHEDULE_REMINDER_CODE);
      sandbox.loadPlugin(CLAUDE_CODE_PROGRESS_CODE);

      // Load user-installed plugins from AppData/plugins/
      try {
        const pluginNames = await invoke<string[]>('scan_plugins');
        for (const name of pluginNames) {
          const code = await invoke<string>('read_plugin_file', { name });
          sandbox.loadPlugin(code);
        }
      } catch (err) {
        console.error('[plugins] Failed to load user plugins:', err);
      }

      // Listen for events from the HTTP receiver (Claude Code hooks, etc.)
      const unlisten = await listen<string>('plugin-event', (event) => {
        sandbox.dispatch(event.payload);
      });

      return () => {
        sandbox.destroy();
        unlisten();
      };
    }

    init().then((fn) => { cleanupFn = fn; });

    return () => { cleanupFn?.(); };
  }, [setPetState]);

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
      style={{ width: 200, height: 240, background: 'transparent', overflow: 'visible', cursor: 'grab', position: 'relative' }}
      onMouseDown={onMouseDown}
      onMouseEnter={() => setPetState('waving')}
      onContextMenu={handleContextMenu}
    >
      <SpeechBubble text={bubbleText} onHide={handleHideBubble} />
      <GifDisplay filePath={gifPath} />
      <ContextMenu
        x={menu.x}
        y={menu.y}
        visible={menu.visible}
        onClose={() => setMenu((prev) => ({ ...prev, visible: false }))}
        onSwitchPet={() => {/* future: open pet picker */}}
        onOpenCreator={handleOpenCreator}
        onExit={() => exit(0)}
      />
    </div>
  );
}
