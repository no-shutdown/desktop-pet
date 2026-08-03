import { create } from 'zustand';
import type { Pet, PetState } from '../types/pet';

interface PetStore {
  petState: PetState;
  activePet: Pet | null;
  setPetState: (state: PetState) => void;
  setActivePet: (pet: Pet | null) => void;
}

let resetTimer: ReturnType<typeof setTimeout> | null = null;

export const usePetStore = create<PetStore>((set) => ({
  petState: 'idle',
  activePet: null,
  setPetState: (state) => {
    if (resetTimer) clearTimeout(resetTimer);
    set({ petState: state });
    if (state !== 'idle') {
      resetTimer = setTimeout(() => set({ petState: 'idle' }), 3000);
    }
  },
  setActivePet: (pet) => set({ activePet: pet }),
}));
