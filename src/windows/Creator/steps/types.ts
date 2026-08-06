import type { PetState, SpriteStateInfo } from '../../../types/pet';

export interface GeneratedSpriteConfig {
  petId: string;
  frameW: number;
  frameH: number;
  idleFrames: number;
  walkingFrames: number;
  wavingFrames: number;
  workingFrames: number;
}

export interface WizardData {
  photoDataUrl: string | null;
  prompt: string;
  apiKey: string;
  petId: string | null;
  petName: string;
  petStates: Record<PetState, SpriteStateInfo> | null;
  generatedDataUrl: string | null;
  generatedConfig: GeneratedSpriteConfig | null;
}

export const INITIAL_WIZARD_DATA: WizardData = {
  photoDataUrl: null,
  prompt: '',
  apiKey: '',
  petId: null,
  petName: '',
  petStates: null,
  generatedDataUrl: null,
  generatedConfig: null,
};
