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

export type PetState = 'idle' | 'sleeping' | 'waving' | 'working';
export const PET_STATES: PetState[] = ['idle', 'sleeping', 'waving', 'working'];

export interface PetStateDefinition {
  key: PetState;
  label: string;
  delayMs: number;
}

export const PET_STATE_LABELS: Record<PetState, string> = {
  idle: '待机',
  sleeping: '睡觉',
  waving: '挥手',
  working: '工作',
};

export const PET_STATE_CATALOG: readonly PetStateDefinition[] = [
  { key: 'idle', label: PET_STATE_LABELS.idle, delayMs: 150 },
  { key: 'sleeping', label: PET_STATE_LABELS.sleeping, delayMs: 200 },
  { key: 'waving', label: PET_STATE_LABELS.waving, delayMs: 110 },
  { key: 'working', label: PET_STATE_LABELS.working, delayMs: 120 },
];
