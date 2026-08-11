export type VisionProvider = 'anthropic' | 'deepseek' | 'kimi' | 'skip';

/**
 * `pollinations` is retained only so older callers can deserialize their
 * settings. `loadSettings` and the settings UI normalize it to SiliconFlow.
 */
export type ImageProvider = 'pollinations' | 'siliconflow' | 'wanxiang' | 'localsd';

export const IMAGE_PROVIDERS = ['siliconflow', 'wanxiang', 'localsd'] as const;

export interface AppSettings {
  visionProvider: VisionProvider;
  visionApiKey: string;
  visionModel: string;
  /** Provider used to generate the canonical base image (step 3). */
  imageProvider: ImageProvider;
  /** Provider used to generate each state row (step 4). Falls back to imageProvider if unset. */
  rowImageProvider: ImageProvider;
  /** SiliconFlow API key. Historical field name; also referred to as the "image" API key. */
  imageApiKey: string;
  /** Legacy compatibility field; kept in sync with imageBaseModel. */
  imageModel: string;
  /** SiliconFlow base model (text-to-image). */
  imageBaseModel: string;
  /** SiliconFlow reference / image-edit model. */
  imageReferenceModel: string;
  /** DashScope (Aliyun Wanxiang) API key. */
  wanxiangApiKey: string;
  /** Wanxiang text-to-image model. */
  wanxiangBaseModel: string;
  /** Wanxiang image-edit model (reference-based row generation). */
  wanxiangEditModel: string;
  localSdUrl: string;
  localSdDenoisingStrength: number;
}

export const ANTHROPIC_MODELS = [
  { value: 'claude-opus-4-7', label: 'Claude Opus 4.7（最强）' },
  { value: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6（均衡）' },
  { value: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5（快速）' },
];

export const DEEPSEEK_MODELS = [
  { value: 'deepseek-vl2', label: 'DeepSeek-VL2' },
  { value: 'deepseek-vl2-tiny', label: 'DeepSeek-VL2 Tiny（快速）' },
];

export const KIMI_MODELS = [
  { value: 'moonshot-v1-8k-vision-preview', label: 'Moonshot 8K 视觉' },
  { value: 'moonshot-v1-32k-vision-preview', label: 'Moonshot 32K 视觉' },
];

export const SILICONFLOW_BASE_MODELS = [
  { value: 'Tongyi-MAI/Z-Image-Turbo', label: 'Z-Image-Turbo（基础）' },
  { value: 'Tongyi-MAI/Z-Image', label: 'Z-Image（基础）' },
  { value: 'baidu/ERNIE-Image-Turbo', label: 'ERNIE-Image-Turbo（基础）' },
] as const;

export const SILICONFLOW_REFERENCE_MODELS = [
  { value: 'Qwen/Qwen-Image-Edit-2509', label: 'Qwen-Image-Edit-2509（图生图）' },
  { value: 'Kwai-Kolors/Kolors', label: 'Kolors（图生图）' },
] as const;

/**
 * Wanxiang text-to-image models available on DashScope.
 * `wan2.*` models use the newer /image-generation/generation endpoint with a
 * chat-style messages body; `wanx*` models use the legacy /text2image endpoint.
 */
export const WANXIANG_BASE_MODELS = [
  { value: 'wan2.7-image', label: 'wan2.7-image（新版，通用）' },
  { value: 'wan2.7-image-pro', label: 'wan2.7-image-pro（新版专业，支持 4K）' },
  { value: 'wan2.6-image', label: 'wan2.6-image（新版）' },
  { value: 'wanx2.1-t2i-turbo', label: 'wanx2.1-t2i-turbo（旧版快速）' },
  { value: 'wanx2.1-t2i-plus', label: 'wanx2.1-t2i-plus（旧版高质量）' },
  { value: 'wanx-v1', label: 'wanx-v1（旧版）' },
] as const;

/**
 * Wanxiang image-edit models. `wan2.*` models handle both generation and
 * editing via the same endpoint; `wanx2.1-imageedit` is the legacy edit model.
 */
export const WANXIANG_EDIT_MODELS = [
  { value: 'wan2.7-image', label: 'wan2.7-image（新版编辑）' },
  { value: 'wan2.7-image-pro', label: 'wan2.7-image-pro（新版专业编辑）' },
  { value: 'wan2.6-image', label: 'wan2.6-image（新版编辑）' },
  { value: 'wanx2.1-imageedit', label: 'wanx2.1-imageedit（旧版）' },
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
  if (value === 'localsd' || value === 'wanxiang') return value;
  return 'siliconflow';
}

const STORAGE_KEY = 'desktop-pet-settings';

export const DEFAULT_SETTINGS: AppSettings = {
  visionProvider: 'skip',
  visionApiKey: '',
  visionModel: 'claude-opus-4-7',
  imageProvider: 'siliconflow',
  rowImageProvider: 'siliconflow',
  imageApiKey: '',
  imageModel: 'Tongyi-MAI/Z-Image-Turbo',
  imageBaseModel: 'Tongyi-MAI/Z-Image-Turbo',
  imageReferenceModel: 'Qwen/Qwen-Image-Edit-2509',
  wanxiangApiKey: '',
  wanxiangBaseModel: 'wan2.7-image',
  wanxiangEditModel: 'wan2.7-image',
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
  const imageProvider = normalizeImageProvider(stored.imageProvider);
  const rowImageProvider = stored.rowImageProvider !== undefined
    ? normalizeImageProvider(stored.rowImageProvider)
    : imageProvider;

  return {
    ...DEFAULT_SETTINGS,
    visionProvider: isVisionProvider(stored.visionProvider)
      ? stored.visionProvider
      : DEFAULT_SETTINGS.visionProvider,
    visionApiKey: nonEmptyString(stored.visionApiKey) ?? '',
    visionModel: nonEmptyString(stored.visionModel) ?? DEFAULT_SETTINGS.visionModel,
    imageProvider,
    rowImageProvider,
    imageApiKey: nonEmptyString(stored.imageApiKey) ?? '',
    imageModel: imageBaseModel,
    imageBaseModel,
    imageReferenceModel,
    wanxiangApiKey: nonEmptyString(stored.wanxiangApiKey) ?? '',
    wanxiangBaseModel: nonEmptyString(stored.wanxiangBaseModel) ?? DEFAULT_SETTINGS.wanxiangBaseModel,
    wanxiangEditModel: nonEmptyString(stored.wanxiangEditModel) ?? DEFAULT_SETTINGS.wanxiangEditModel,
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
  kimi: 'Kimi（月之暗面）',
  skip: '跳过',
};

/**
 * Given a provider and settings, return the API key that provider needs.
 * LocalSD uses a URL instead of a key and returns an empty string.
 */
export function apiKeyForProvider(settings: AppSettings, provider: ImageProvider): string {
  switch (provider) {
    case 'wanxiang': return settings.wanxiangApiKey;
    case 'localsd': return '';
    case 'pollinations':
    case 'siliconflow':
    default: return settings.imageApiKey;
  }
}

/**
 * Given a provider and settings, return that provider's base (text-to-image) model.
 */
export function baseModelForProvider(settings: AppSettings, provider: ImageProvider): string {
  switch (provider) {
    case 'wanxiang': return settings.wanxiangBaseModel;
    case 'localsd': return '';
    case 'pollinations':
    case 'siliconflow':
    default: return settings.imageBaseModel;
  }
}

/**
 * Given a provider and settings, return that provider's row / reference-edit model.
 */
export function rowModelForProvider(settings: AppSettings, provider: ImageProvider): string {
  switch (provider) {
    case 'wanxiang': return settings.wanxiangEditModel;
    case 'localsd': return '';
    case 'pollinations':
    case 'siliconflow':
    default: return settings.imageReferenceModel;
  }
}
