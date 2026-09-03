import { Menu, type MenuOptions } from '@tauri-apps/api/menu';
import type { PetState } from '../../types/pet';

interface PetContextMenuActions {
  onSwitchAction: (state: PetState) => void;
  onSwitchPet: () => void;
  onOpenCreator: () => void;
  onExit: () => void;
}

export function buildPetContextMenuItems(
  actions: PetContextMenuActions,
): NonNullable<MenuOptions['items']> {
  return [
    {
      id: 'switch-action',
      text: '切换动作',
      items: [
        { id: 'action-idle', text: '待机', action: () => actions.onSwitchAction('idle') },
        { id: 'action-sleeping', text: '睡觉', action: () => actions.onSwitchAction('sleeping') },
        { id: 'action-acting-cute', text: '撒娇', action: () => actions.onSwitchAction('acting_cute') },
        { id: 'action-working', text: '工作', action: () => actions.onSwitchAction('working') },
      ],
    },
    { item: 'Separator' },
    { id: 'switch-pet', text: '切换宠物', action: actions.onSwitchPet },
    { id: 'open-creator', text: '打开创建器', action: actions.onOpenCreator },
    { item: 'Separator' },
    { id: 'exit', text: '退出', action: actions.onExit },
  ];
}

export function createPetContextMenu(actions: PetContextMenuActions): Promise<Menu> {
  return Menu.new({
    id: 'pet-context-menu',
    items: buildPetContextMenuItems(actions),
  });
}
