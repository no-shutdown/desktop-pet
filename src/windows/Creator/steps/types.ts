export interface WizardData {
  photoDataUrl: string | null;
  prompt: string;
  apiKey: string;
  petId: string | null;
  petName: string;
}

export const INITIAL_WIZARD_DATA: WizardData = {
  photoDataUrl: null,
  prompt: '',
  apiKey: '',
  petId: null,
  petName: '',
};
