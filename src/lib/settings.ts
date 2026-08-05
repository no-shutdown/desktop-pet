export type VisionProvider = 'anthropic' | 'deepseek' | 'kimi' | 'skip';
export type ImageProvider = 'pollinations' | 'siliconflow' | 'localsd';

export interface AppSettings {
  visionProvider: VisionProvider;
  visionApiKey: string;
  visionModel: string;
  imageProvider: ImageProvider;
  imageApiKey: string;
  imageModel: string;
  localSdUrl: string;
}

export const ANTHROPIC_MODELS = [
  { value: 'claude-opus-4-7',          label: 'Claude Opus 4.7（最强）' },
  { value: 'claude-sonnet-4-6',        label: 'Claude Sonnet 4.6（均衡）' },
  { value: 'claude-haiku-4-5-20251001',label: 'Claude Haiku 4.5（快速）' },
];

export const DEEPSEEK_MODELS = [
  { value: 'deepseek-vl2',      label: 'DeepSeek-VL2' },
  { value: 'deepseek-vl2-tiny', label: 'DeepSeek-VL2 Tiny（快速）' },
];

export const KIMI_MODELS = [
  { value: 'moonshot-v1-8k-vision-preview',  label: 'Moonshot 8K Vision' },
  { value: 'moonshot-v1-32k-vision-preview', label: 'Moonshot 32K Vision' },
];

export const SILICONFLOW_MODELS = [
  { value: 'Tongyi-MAI/Z-Image-Turbo', label: 'Z-Image-Turbo（通义）' },
  { value: 'Tongyi-MAI/Z-Image',       label: 'Z-Image（通义）' },
  { value: 'baidu/ERNIE-Image-Turbo',  label: 'ERNIE-Image-Turbo（文心）' },
  { value: 'Kwai-Kolors/Kolors',       label: 'Kolors（快手）' },
];

export function getVisionModels(provider: VisionProvider) {
  switch (provider) {
    case 'anthropic': return ANTHROPIC_MODELS;
    case 'deepseek':  return DEEPSEEK_MODELS;
    case 'kimi':      return KIMI_MODELS;
    default:          return [];
  }
}

export function defaultVisionModel(provider: VisionProvider): string {
  return getVisionModels(provider)[0]?.value ?? '';
}

const STORAGE_KEY = 'desktop-pet-settings';

export const DEFAULT_SETTINGS: AppSettings = {
  visionProvider: 'skip',
  visionApiKey: '',
  visionModel: 'claude-opus-4-7',
  imageProvider: 'pollinations',
  imageApiKey: '',
  imageModel: 'Tongyi-MAI/Z-Image-Turbo',
  localSdUrl: 'http://localhost:7860',
};

export function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function saveSettings(settings: AppSettings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

export const VISION_PROVIDER_LABELS: Record<VisionProvider, string> = {
  anthropic: 'Anthropic (Claude)',
  deepseek:  'DeepSeek',
  kimi:      'Kimi (Moonshot)',
  skip:      'Skip',
};
