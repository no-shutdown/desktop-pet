import type { PetState, SpriteStateInfo } from '../../../types/pet';

export type GenerationProvider = 'siliconflow' | 'wanxiang' | 'localsd';
export type GenerationPhase = 'base' | 'state' | 'assemble';
export type GenerationStatus = 'pending' | 'generating' | 'complete' | 'failed';
export type SourceStyle = 'realistic' | 'stylized';

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
  runId: string;
  frameW: number;
  frameH: number;
  rowGap: number;
  layout: 'horizontalRows' | 'grid';
  idleFrames: number;
  sleepingFrames: number;
  actingCuteFrames: number;
  workingFrames: number;
}

export interface WizardData {
  photoDataUrl: string | null;
  styleReferenceDataUrl: string | null;
  prompt: string;
  sourceStyle: SourceStyle;
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
  styleReferenceDataUrl: null,
  prompt: '',
  sourceStyle: 'realistic',
  apiKey: '',
  petId: null,
  petName: '',
  petStates: null,
  generatedDataUrl: null,
  generatedConfig: null,
  generationRunId: null,
  baseDataUrl: null,
};
