import { useEffect, useState, useCallback, useRef } from 'react';
import { getCurrentWindow, PhysicalPosition } from '@tauri-apps/api/window';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { exit } from '@tauri-apps/plugin-process';
import type { MouseEvent } from 'react';
import ContextMenu from './ContextMenu';
import SpeechBubble from './SpeechBubble';
import PetPicker from './PetPicker';
import { useDrag } from './useDrag';
import { usePetStore } from '../../store/petStore';
import { PluginSandbox } from '../../lib/plugin-sandbox';
import { SCHEDULE_REMINDER_CODE, CLAUDE_CODE_PROGRESS_CODE } from '../../lib/bundled-plugins';
import { appDataDir, join } from '@tauri-apps/api/path';
import SpriteAnimator from './SpriteAnimator';
import { type Pet, type PetState } from '../../types/pet';

export default function PetWindow() {
  const { onMouseDown } = useDrag();
  const { petState, setPetState, activePet, setActivePet } = usePetStore();
  const [menu, setMenu] = useState<{ x: number; y: number; visible: boolean }>({
    x: 0, y: 0, visible: false,
  });
  const [bubbleText, setBubbleText] = useState<string | null>(null);
  const [showPicker, setShowPicker] = useState(false);
  const [allPets, setAllPets] = useState<Pet[]>([]);
  const [petsDir, setPetsDir] = useState<string | null>(null);
  const isDragging = useRef(false);

  const handleHideBubble = useCallback(() => setBubbleText(null), []);

  useEffect(() => {
    let cleanupFn: (() => void) | undefined;

    async function init() {
      const win = getCurrentWindow();
      const savedPos = await invoke<{ x: number; y: number } | null>('load_window_position');
      if (savedPos) {
        await win.setPosition(new PhysicalPosition(savedPos.x, savedPos.y));
      }

      async function loadPets() {
        try {
          const pets = await invoke<Pet[]>('list_pets');
          const sorted = [...pets].sort((a, b) =>
            new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
          );
          setAllPets(sorted);
          if (sorted.length === 0) return;

          const currentId = usePetStore.getState().activePet?.id;
          const keep = sorted.find((p) => p.id === currentId) ?? sorted[0];
          if (keep.id !== currentId) {
            usePetStore.getState().setActivePet(keep);
          } else if (!currentId) {
            usePetStore.getState().setActivePet(keep);
          }
        } catch (err) {
          console.error('[pet] Failed to load pets:', err);
        }
      }

      // Load pets before showing — avoids the blank transparent window flash
      await loadPets();
      const appDir = await appDataDir();
      setPetsDir(await join(appDir, 'pets'));
      if (usePetStore.getState().activePet !== null) {
        await win.show();
      }

      // Reload when tray "Show Pet" is clicked, and show if a pet is available
      const unlistenShow = await listen('pet-window-show', async () => {
        await loadPets();
        if (usePetStore.getState().activePet !== null) {
          await win.show();
        }
      });

      const sandbox = new PluginSandbox(
        (state: PetState) => setPetState(state),
        (text: string) => setBubbleText(text),
      );
      sandbox.loadPlugin(SCHEDULE_REMINDER_CODE);
      sandbox.loadPlugin(CLAUDE_CODE_PROGRESS_CODE);

      try {
        const pluginNames = await invoke<string[]>('scan_plugins');
        for (const name of pluginNames) {
          const code = await invoke<string>('read_plugin_file', { name });
          sandbox.loadPlugin(code);
        }
      } catch (err) {
        console.error('[plugins] Failed to load user plugins:', err);
      }

      const unlisten = await listen<string>('plugin-event', (event) => {
        sandbox.dispatch(event.payload);
      });

      const unlistenPetSaved = await listen<Pet>('pet-saved', async (event) => {
        usePetStore.getState().setActivePet(event.payload);
        setAllPets((prev) => {
          const without = prev.filter((p) => p.id !== event.payload.id);
          return [event.payload, ...without];
        });
        await win.show();
      });

      return () => {
        sandbox.destroy();
        unlisten();
        unlistenPetSaved();
        unlistenShow();
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
    await invoke('open_creator');
  };

  async function handleSwitchPet() {
    setMenu((prev) => ({ ...prev, visible: false }));
    try {
      const pets = await invoke<Pet[]>('list_pets');
      const sorted = [...pets].sort((a, b) =>
        new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
      );
      setAllPets(sorted);
    } catch {}
    setShowPicker(true);
  }

  async function handleMouseDown(e: MouseEvent) {
    if (showPicker) return;
    isDragging.current = true;
    await onMouseDown(e);
    isDragging.current = false;
  }

  return (
    <div
      style={{ width: 200, height: 240, background: 'transparent', overflow: 'visible', cursor: 'grab', position: 'relative' }}
      onMouseDown={handleMouseDown}
      onMouseEnter={() => { if (!showPicker && !isDragging.current) setPetState('waving'); }}
      onContextMenu={handleContextMenu}
    >
      <SpeechBubble text={bubbleText} onHide={handleHideBubble} />
      {activePet && petsDir && (
        <SpriteAnimator
          sheetSrc={convertFileSrc(`${petsDir}/${activePet.id}/${petState}.png`)}
          meta={activePet.states[petState]}
          displayW={200}
          displayH={200}
        />
      )}

      {showPicker && (
        <PetPicker
          pets={allPets}
          activePetId={activePet?.id ?? null}
          onSelect={(pet) => { setActivePet(pet); setShowPicker(false); }}
          onClose={() => setShowPicker(false)}
        />
      )}

      <ContextMenu
        x={menu.x}
        y={menu.y}
        visible={menu.visible}
        onClose={() => setMenu((prev) => ({ ...prev, visible: false }))}
        onSwitchPet={handleSwitchPet}
        onOpenCreator={handleOpenCreator}
        onExit={() => exit(0)}
      />
    </div>
  );
}
