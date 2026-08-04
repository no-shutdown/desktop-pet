export type VisionProvider = 'anthropic' | 'deepseek' | 'kimi' | 'skip';
export type ImageProvider = 'pollinations' | 'siliconflow' | 'localsd';

export interface AppSettings {
  visionProvider: VisionProvider;
  visionApiKey: string;
  imageProvider: ImageProvider;
  imageApiKey: string;
  localSdUrl: string;
}

const STORAGE_KEY = 'desktop-pet-settings';

export const DEFAULT_SETTINGS: AppSettings = {
  visionProvider: 'skip',
  visionApiKey: '',
  imageProvider: 'pollinations',
  imageApiKey: '',
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
  deepseek: 'DeepSeek',
  kimi: 'Kimi (Moonshot)',
  skip: 'Skip',
};
