import { describe, expect, it, vi } from 'vitest';
import type { MenuItemOptions, SubmenuOptions } from '@tauri-apps/api/menu';
import { buildPetContextMenuItems } from '../ContextMenu';

function itemById(items: ReturnType<typeof buildPetContextMenuItems>, id: string) {
  return items.find((item) => 'id' in item && item.id === id) as MenuItemOptions | undefined;
}

describe('buildPetContextMenuItems', () => {
  function actions() {
    return {
      onSwitchAction: vi.fn(),
      onSwitchPet: vi.fn(),
      onOpenCreator: vi.fn(),
      onExit: vi.fn(),
    };
  }

  it('builds a complete Chinese native menu with all four pet actions', () => {
    const items = buildPetContextMenuItems(actions());
    const actionMenu = items[0] as SubmenuOptions;

    expect(actionMenu.text).toBe('切换动作');
    expect(actionMenu.items?.map((item) => 'text' in item ? item.text : null)).toEqual([
      '待机',
      '睡觉',
      '撒娇',
      '工作',
    ]);
    expect(itemById(items, 'switch-pet')?.text).toBe('切换宠物');
    expect(itemById(items, 'open-creator')?.text).toBe('打开创建器');
    expect(itemById(items, 'exit')?.text).toBe('退出');
  });

  it.each([
    ['action-idle', 'idle'],
    ['action-sleeping', 'sleeping'],
    ['action-acting-cute', 'acting_cute'],
    ['action-working', 'working'],
  ] as const)('maps %s to the %s pet state', (itemId, state) => {
    const callbacks = actions();
    const actionMenu = buildPetContextMenuItems(callbacks)[0] as SubmenuOptions;
    const actionItem = itemById(actionMenu.items ?? [], itemId);

    actionItem?.action?.(itemId);

    expect(callbacks.onSwitchAction).toHaveBeenCalledOnce();
    expect(callbacks.onSwitchAction).toHaveBeenCalledWith(state);
  });

  it('connects the existing pet, creator, and exit actions', () => {
    const callbacks = actions();
    const items = buildPetContextMenuItems(callbacks);

    itemById(items, 'switch-pet')?.action?.('switch-pet');
    itemById(items, 'open-creator')?.action?.('open-creator');
    itemById(items, 'exit')?.action?.('exit');

    expect(callbacks.onSwitchPet).toHaveBeenCalledOnce();
    expect(callbacks.onOpenCreator).toHaveBeenCalledOnce();
    expect(callbacks.onExit).toHaveBeenCalledOnce();
  });
});
