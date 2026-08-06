import type { PetState, SpriteStateInfo } from '../../../types/pet';

export type GenerationProvider = 'siliconflow' | 'localsd';
export type GenerationPhase = 'base' | 'state' | 'assemble';
export type GenerationStatus = 'pending' | 'generating' | 'complete' | 'failed';

export interface GenerationRunPreview {
  runId: string;
  dataUrl: string;
  frameW: number;
  frameH: number;
  frameCount: number;
  rowGap: number;
}

export interface GeneratedSpriteConfig {
  petId: string;
  /** Optional while the legacy all-in-one generator remains in the Creator flow. */
  runId?: string;
  frameW: number;
  frameH: number;
  /** Optional while legacy grid imports remain supported. */
  rowGap?: number;
  /** Optional while legacy grid imports remain supported. */
  layout?: 'horizontalRows' | 'grid';
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
  generationRunId: string | null;
  baseDataUrl: string | null;
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
  generationRunId: null,
  baseDataUrl: null,
};
