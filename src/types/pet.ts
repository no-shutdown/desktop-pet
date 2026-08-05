export interface SpriteStateInfo {
  cols: number;
  rows: number;
  frameCount: number;
  frameW: number;
  frameH: number;
  delayMs: number;
}

export interface Pet {
  id: string;
  name: string;
  prompt: string;
  createdAt: string;
  states: Record<PetState, SpriteStateInfo>;
}

export type PetState = 'idle' | 'walking' | 'waving' | 'working';
export const PET_STATES: PetState[] = ['idle', 'walking', 'waving', 'working'];
