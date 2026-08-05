import type { PetState, SpriteStateInfo } from '../../../types/pet';

export interface WizardData {
  photoDataUrl: string | null;
  prompt: string;
  apiKey: string;
  petId: string | null;
  petName: string;
  petStates: Record<PetState, SpriteStateInfo> | null;
}

export const INITIAL_WIZARD_DATA: WizardData = {
  photoDataUrl: null,
  prompt: '',
  apiKey: '',
  petId: null,
  petName: '',
  petStates: null,
};
