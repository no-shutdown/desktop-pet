export type VisionProvider = 'anthropic' | 'deepseek' | 'kimi' | 'skip';

/**
 * `pollinations` is retained only so older callers can deserialize their
 * settings. `loadSettings` and the settings UI normalize it to SiliconFlow.
 */
export type ImageProvider = 'pollinations' | 'siliconflow' | 'localsd';

export const IMAGE_PROVIDERS = ['siliconflow', 'localsd'] as const;

export interface AppSettings {
  visionProvider: VisionProvider;
  visionApiKey: string;
  visionModel: string;
  imageProvider: ImageProvider;
  imageApiKey: string;
  /** Legacy compatibility field; kept in sync with imageBaseModel. */
  imageModel: string;
  imageBaseModel: string;
  imageReferenceModel: string;
  localSdUrl: string;
  localSdDenoisingStrength: number;
}

export const ANTHROPIC_MODELS = [
  { value: 'claude-opus-4-7', label: 'Claude Opus 4.7 (most capable)' },
  { value: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6 (balanced)' },
  { value: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5 (fast)' },
];

export const DEEPSEEK_MODELS = [
  { value: 'deepseek-vl2', label: 'DeepSeek-VL2' },
  { value: 'deepseek-vl2-tiny', label: 'DeepSeek-VL2 Tiny (fast)' },
];

export const KIMI_MODELS = [
  { value: 'moonshot-v1-8k-vision-preview', label: 'Moonshot 8K Vision' },
  { value: 'moonshot-v1-32k-vision-preview', label: 'Moonshot 32K Vision' },
];

export const SILICONFLOW_BASE_MODELS = [
  { value: 'Tongyi-MAI/Z-Image-Turbo', label: 'Z-Image-Turbo (Base)' },
  { value: 'Tongyi-MAI/Z-Image', label: 'Z-Image (Base)' },
  { value: 'baidu/ERNIE-Image-Turbo', label: 'ERNIE-Image-Turbo (Base)' },
] as const;

export const SILICONFLOW_REFERENCE_MODELS = [
  { value: 'Qwen/Qwen-Image-Edit-2509', label: 'Qwen-Image-Edit-2509 (img2img)' },
  { value: 'Kwai-Kolors/Kolors', label: 'Kolors (img2img)' },
] as const;

/**
 * Compatibility list for the pre-canonical Creator flow. Keep its historical
 * choices available while the settings panel uses separate model lists.
 */
export const SILICONFLOW_MODELS = [
  ...SILICONFLOW_BASE_MODELS,
  { value: 'Kwai-Kolors/Kolors', label: 'Kolors (img2img)' },
] as const;

export const LOCAL_SD_DENOISING_MIN = 0.35;
export const LOCAL_SD_DENOISING_MAX = 0.75;
export const DEFAULT_LOCAL_SD_DENOISING_STRENGTH = 0.55;

export function getVisionModels(provider: VisionProvider) {
  switch (provider) {
    case 'anthropic': return ANTHROPIC_MODELS;
    case 'deepseek': return DEEPSEEK_MODELS;
    case 'kimi': return KIMI_MODELS;
    default: return [];
  }
}

export function defaultVisionModel(provider: VisionProvider): string {
  return getVisionModels(provider)[0]?.value ?? '';
}

export function normalizeDenoisingStrength(value: unknown): number {
  if (value === null || value === undefined || (typeof value === 'string' && value.trim() === '')) {
    return DEFAULT_LOCAL_SD_DENOISING_STRENGTH;
  }

  const numericValue = typeof value === 'number'
    ? value
    : typeof value === 'string'
      ? Number(value.trim())
      : Number.NaN;
  if (!Number.isFinite(numericValue)) return DEFAULT_LOCAL_SD_DENOISING_STRENGTH;

  return Math.min(
    LOCAL_SD_DENOISING_MAX,
    Math.max(LOCAL_SD_DENOISING_MIN, numericValue),
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function nonEmptyString(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function isVisionProvider(value: unknown): value is VisionProvider {
  return value === 'anthropic' || value === 'deepseek' || value === 'kimi' || value === 'skip';
}

function normalizeImageProvider(value: unknown): ImageProvider {
  return value === 'localsd' ? 'localsd' : 'siliconflow';
}

const STORAGE_KEY = 'desktop-pet-settings';

export const DEFAULT_SETTINGS: AppSettings = {
  visionProvider: 'skip',
  visionApiKey: '',
  visionModel: 'claude-opus-4-7',
  imageProvider: 'siliconflow',
  imageApiKey: '',
  imageModel: 'Tongyi-MAI/Z-Image-Turbo',
  imageBaseModel: 'Tongyi-MAI/Z-Image-Turbo',
  imageReferenceModel: 'Qwen/Qwen-Image-Edit-2509',
  localSdUrl: 'http://localhost:7860',
  localSdDenoisingStrength: DEFAULT_LOCAL_SD_DENOISING_STRENGTH,
};

export function normalizeSettings(raw: unknown): AppSettings {
  const stored = isRecord(raw) ? raw : {};
  const legacyImageModel = nonEmptyString(stored.imageModel);
  const imageBaseModel = nonEmptyString(stored.imageBaseModel)
    ?? legacyImageModel
    ?? DEFAULT_SETTINGS.imageBaseModel;
  const imageReferenceModel = nonEmptyString(stored.imageReferenceModel)
    ?? DEFAULT_SETTINGS.imageReferenceModel;

  return {
    ...DEFAULT_SETTINGS,
    visionProvider: isVisionProvider(stored.visionProvider)
      ? stored.visionProvider
      : DEFAULT_SETTINGS.visionProvider,
    visionApiKey: nonEmptyString(stored.visionApiKey) ?? '',
    visionModel: nonEmptyString(stored.visionModel) ?? DEFAULT_SETTINGS.visionModel,
    imageProvider: normalizeImageProvider(stored.imageProvider),
    imageApiKey: nonEmptyString(stored.imageApiKey) ?? '',
    imageModel: imageBaseModel,
    imageBaseModel,
    imageReferenceModel,
    localSdUrl: nonEmptyString(stored.localSdUrl) ?? DEFAULT_SETTINGS.localSdUrl,
    localSdDenoisingStrength: normalizeDenoisingStrength(stored.localSdDenoisingStrength),
  };
}

export function loadSettings(): AppSettings {
  try {
    if (typeof localStorage === 'undefined') return { ...DEFAULT_SETTINGS };
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return normalizeSettings(JSON.parse(raw));
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function saveSettings(settings: AppSettings): void {
  try {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(normalizeSettings(settings)));
  } catch {
    // Settings are optional; a restricted storage implementation should not
    // prevent the Creator from opening.
  }
}

export const VISION_PROVIDER_LABELS: Record<VisionProvider, string> = {
  anthropic: 'Anthropic (Claude)',
  deepseek: 'DeepSeek',
  kimi: 'Kimi (Moonshot)',
  skip: 'Skip',
};
