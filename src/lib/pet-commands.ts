import { invoke } from '@tauri-apps/api/core';
import type { Pet } from '../types/pet';

export const petCommands = {
  save: (pet: Pet) => invoke<void>('save_pet', { pet }),
  list: () => invoke<Pet[]>('list_pets'),
  delete: (petId: string) => invoke<void>('delete_pet', { petId }),
};
