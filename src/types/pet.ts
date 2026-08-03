export interface PetFrames {
  idle: string;
  walking: string;
  waving: string;
  working: string;
}

export interface Pet {
  id: string;
  name: string;
  frames: PetFrames;
  createdAt: string;
  prompt: string;
}

export type PetState = 'idle' | 'walking' | 'waving' | 'working';

export const PET_STATES: PetState[] = ['idle', 'walking', 'waving', 'working'];
