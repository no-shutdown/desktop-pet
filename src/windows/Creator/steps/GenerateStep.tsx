import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  apiKeyForProvider,
  baseModelForProvider,
  loadSettings,
  saveSettings,
  SILICONFLOW_BASE_MODELS,
  WANXIANG_BASE_MODELS,
  type AppSettings,
  type ImageProvider,
} from '../../../lib/settings';
import type { GenerationProvider, SourceStyle } from './types';

interface BasePreviewResult {
  runId: string;
  dataUrl: string;
  chromaKey: string;
}

interface GenerationProgress {
  runId: string;
  phase: string;
  current: number;
  total: number;
}

interface GenerateStepProps {
  prompt: string;
  referenceDataUrl?: string | null;
  styleReferenceDataUrl?: string | null;
  sourceStyle: SourceStyle;
  runId?: string;
  onNext: (base: { runId: string; dataUrl: string }) => void;
  onBack: () => void;
  onBusyChange?: (busy: boolean) => void;
  onStyleReferenceChange?: (dataUrl: string | null) => void;
}

type Status = 'idle' | 'generating' | 'ready' | 'error';

const IMAGE_OPTIONS: { value: GenerationProvider; label: string; desc: string }[] = [
  { value: 'siliconflow', label: '硅基流动 SiliconFlow', desc: '云端图像生成' },
  { value: 'wanxiang', label: '阿里云万相', desc: 'DashScope wanx 系列' },
  { value: 'localsd', label: '本地 Stable Diffusion', desc: 'AUTOMATIC1111 WebUI' },
];

const STYLE_REFERENCE_ACCEPT = 'image/jpeg,image/png,image/webp';
const STYLE_REFERENCE_MIME_TYPES = new Set(['image/jpeg', 'image/png', 'image/webp']);
const STYLE_REFERENCE_ERROR_MESSAGE = '风格参考图读取失败，请重试。';

function makeRunId(): string {
  if (typeof globalThis.crypto?.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  return `generation-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function supportedProvider(provider: ImageProvider): GenerationProvider {
  if (provider === 'localsd') return 'localsd';
  if (provider === 'wanxiang') return 'wanxiang';
  return 'siliconflow';
}

function apiKeyPatchForProvider(
  provider: GenerationProvider,
  value: string,
): Partial<AppSettings> {
  return provider === 'wanxiang' ? { wanxiangApiKey: value } : { imageApiKey: value };
}

function baseModelPatchForProvider(
  provider: GenerationProvider,
  value: string,
): Partial<AppSettings> {
  if (provider === 'wanxiang') return { wanxiangBaseModel: value };
  return { imageBaseModel: value, imageModel: value };
}

function messageFromError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function GenerateStep({
  prompt,
  referenceDataUrl,
  styleReferenceDataUrl,
  sourceStyle,
  runId,
  onNext,
  onBack,
  onBusyChange,
  onStyleReferenceChange,
}: GenerateStepProps) {
  const [stableRunId] = useState(() => runId ?? makeRunId());
  const mountedRef = useRef(true);
  const busyRef = useRef(false);
  const styleReferenceInputRef = useRef<HTMLInputElement>(null);
  const styleReferenceReaderRef = useRef<FileReader | null>(null);
  const styleReferenceReadIdRef = useRef(0);
  const busyCallbackRef = useRef(onBusyChange);
  const [status, setStatus] = useState<Status>('idle');
  const [preview, setPreview] = useState<BasePreviewResult | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [progress, setProgress] = useState({ current: 0, total: 1 });
  const [settings, setSettings] = useState(loadSettings);
  const [styleReference, setStyleReference] = useState<string | null>(styleReferenceDataUrl ?? null);
  const [styleReferenceError, setStyleReferenceError] = useState<string | null>(null);

  busyCallbackRef.current = onBusyChange;

  function reportBusy(busy: boolean) {
    if (busyRef.current === busy) return;
    busyRef.current = busy;
    busyCallbackRef.current?.(busy);
  }

  function invalidateStyleReferenceRead() {
    styleReferenceReadIdRef.current += 1;
    const activeReader = styleReferenceReaderRef.current;
    styleReferenceReaderRef.current = null;
    activeReader?.abort();
  }

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      invalidateStyleReferenceRead();
      reportBusy(false);
    };
  }, []);

  useEffect(() => {
    setStyleReference(styleReferenceDataUrl ?? null);
  }, [styleReferenceDataUrl]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void listen<GenerationProgress>('generation-progress', (event) => {
      const payload = event.payload;
      if (!mountedRef.current || payload.runId !== stableRunId || payload.phase !== 'base') return;
      setProgress({ current: payload.current, total: payload.total });
    }).then((cleanup) => {
      if (active) {
        unlisten = cleanup;
      } else {
        cleanup();
      }
    }).catch(() => {
      // Progress is optional UI feedback; command results remain authoritative.
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [stableRunId]);

  function updateSettings(patch: Partial<ReturnType<typeof loadSettings>>) {
    setSettings((previous) => {
      const next = { ...previous, ...patch };
      saveSettings(next);
      return next;
    });
  }

  function handleStyleReferenceChange(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file || !STYLE_REFERENCE_MIME_TYPES.has(file.type)) return;

    setStyleReferenceError(null);
    invalidateStyleReferenceRead();
    const readId = styleReferenceReadIdRef.current;
    const reader = new FileReader();
    styleReferenceReaderRef.current = reader;
    reader.onload = (loadEvent) => {
      if (
        !mountedRef.current
        || styleReferenceReadIdRef.current !== readId
        || styleReferenceReaderRef.current !== reader
      ) return;

      const result = (loadEvent.target as FileReader).result;
      styleReferenceReaderRef.current = null;
      if (typeof result !== 'string') {
        setStyleReferenceError(STYLE_REFERENCE_ERROR_MESSAGE);
        return;
      }
      setStyleReference(result);
      onStyleReferenceChange?.(result);
    };
    reader.onerror = () => {
      if (
        !mountedRef.current
        || styleReferenceReadIdRef.current !== readId
        || styleReferenceReaderRef.current !== reader
      ) return;

      styleReferenceReaderRef.current = null;
      setStyleReferenceError(STYLE_REFERENCE_ERROR_MESSAGE);
    };
    reader.onabort = () => {
      if (
        !mountedRef.current
        || styleReferenceReadIdRef.current !== readId
        || styleReferenceReaderRef.current !== reader
      ) return;

      styleReferenceReaderRef.current = null;
    };

    try {
      reader.readAsDataURL(file);
    } catch {
      if (
        !mountedRef.current
        || styleReferenceReadIdRef.current !== readId
        || styleReferenceReaderRef.current !== reader
      ) return;

      styleReferenceReaderRef.current = null;
      setStyleReferenceError(STYLE_REFERENCE_ERROR_MESSAGE);
    }
  }

  function handleRemoveStyleReference() {
    invalidateStyleReferenceRead();
    setStyleReference(null);
    setStyleReferenceError(null);
    onStyleReferenceChange?.(null);
    if (styleReferenceInputRef.current) {
      styleReferenceInputRef.current.value = '';
    }
  }

  function openStyleReferencePicker() {
    styleReferenceInputRef.current?.click();
  }

  function handleStyleReferencePickerKeyDown(event: React.KeyboardEvent<HTMLButtonElement>) {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    openStyleReferencePicker();
  }

  async function handleGenerate() {
    if (status === 'generating') return;

    reportBusy(true);
    setStatus('generating');
    setPreview(null);
    setErrorMsg(null);
    setProgress({ current: 0, total: 1 });

    try {
      const activeProvider = supportedProvider(settings.imageProvider);
      const result = await invoke<BasePreviewResult>('generate_base_preview', {
        runId: stableRunId,
        basePrompt: prompt,
        referenceDataUrl: referenceDataUrl || null,
        styleReferenceDataUrl: styleReference || null,
        imageProvider: activeProvider,
        imageApiKey: apiKeyForProvider(settings, settings.imageProvider) || null,
        baseModel: baseModelForProvider(settings, settings.imageProvider) || null,
        referenceModel: settings.imageReferenceModel || null,
        localSdUrl: settings.localSdUrl || null,
        denoisingStrength: settings.localSdDenoisingStrength,
        sourceStyle,
      });
      if (!mountedRef.current) return;
      setPreview(result);
      setStatus('ready');
    } catch (error) {
      if (!mountedRef.current) return;
      setErrorMsg(messageFromError(error));
      setStatus('error');
    } finally {
      reportBusy(false);
    }
  }

  function handleConfirm() {
    if (!preview) return;
    onNext({ runId: preview.runId || stableRunId, dataUrl: preview.dataUrl });
  }

  const provider = supportedProvider(settings.imageProvider);
  const generating = status === 'generating';
  const progressPercent = progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div style={{ background: '#f7fafc', borderRadius: 10, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 10 }}>
        <p style={{ margin: 0, fontSize: 12, color: '#718096', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          生成基础图像
        </p>
        {IMAGE_OPTIONS.map(({ value, label, desc }) => (
          <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 8, cursor: 'pointer' }}>
            <input
              type="radio"
              name="imageProvider"
              value={value}
              checked={provider === value}
              onChange={() => updateSettings({ imageProvider: value })}
              style={{ marginTop: 3, accentColor: '#4f8ef7' }}
            />
            <div>
              <div style={{ fontSize: 13, fontWeight: 500, color: '#2d3748' }}>{label}</div>
              <div style={{ fontSize: 11, color: '#a0aec0' }}>{desc}</div>
            </div>
          </label>
        ))}

        {provider === 'siliconflow' && (
          <>
            <input
              type="password"
              aria-label="SiliconFlow API Key"
              value={settings.imageApiKey}
              onChange={(event) => updateSettings(apiKeyPatchForProvider(provider, event.target.value))}
              placeholder="SiliconFlow API Key"
              style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
            />
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12, color: '#718096' }}>
              基础模型
              <select
                aria-label="SiliconFlow 基础模型"
                value={settings.imageBaseModel}
                onChange={(event) => updateSettings(baseModelPatchForProvider(provider, event.target.value))}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {SILICONFLOW_BASE_MODELS.map(({ value, label }) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </label>
          </>
        )}

        {provider === 'wanxiang' && (
          <>
            <input
              type="password"
              aria-label="万相 API Key"
              value={settings.wanxiangApiKey}
              onChange={(event) => updateSettings(apiKeyPatchForProvider(provider, event.target.value))}
              placeholder="DashScope API Key（sk-…）"
              style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
            />
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12, color: '#718096' }}>
              基础模型
              <select
                aria-label="万相基础模型"
                value={settings.wanxiangBaseModel}
                onChange={(event) => updateSettings(baseModelPatchForProvider(provider, event.target.value))}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {WANXIANG_BASE_MODELS.map(({ value, label }) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </label>
          </>
        )}

        {provider === 'localsd' && (
          <input
            type="text"
            aria-label="本地 Stable Diffusion 地址"
            value={settings.localSdUrl}
            onChange={(event) => updateSettings({ localSdUrl: event.target.value })}
            placeholder="http://localhost:7860"
            style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
          />
        )}
      </div>

      <div style={{ background: '#f7fafc', borderRadius: 10, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 10 }}>
        <div style={{ fontSize: 13, color: '#2d3748', fontWeight: 600 }}>风格参考图（可选）</div>
        <input
          id="style-reference-file-input"
          data-testid="style-reference-file-input"
          ref={styleReferenceInputRef}
          type="file"
          accept={STYLE_REFERENCE_ACCEPT}
          aria-label="选择风格参考图"
          style={{
            position: 'absolute',
            width: 1,
            height: 1,
            padding: 0,
            margin: -1,
            overflow: 'hidden',
            clip: 'rect(0, 0, 0, 0)',
            whiteSpace: 'nowrap',
            border: 0,
          }}
          onChange={handleStyleReferenceChange}
        />
        {styleReference ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <img
              alt="风格参考图预览"
              src={styleReference}
              style={{ width: 96, height: 96, objectFit: 'cover', borderRadius: 8, border: '1px solid #e2e8f0' }}
            />
            <button
              type="button"
              aria-label="移除风格参考图"
              onClick={handleRemoveStyleReference}
              style={{ padding: '7px 12px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
            >
              移除风格参考图
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={openStyleReferencePicker}
            onKeyDown={handleStyleReferencePickerKeyDown}
            style={{ width: '100%', border: '1px dashed #cbd5e0', borderRadius: 8, padding: '14px 16px', color: '#4f8ef7', background: '#fff', cursor: 'pointer', textAlign: 'center' }}
          >
            上传一张参考画风的图片（可选）
          </button>
        )}
        <p style={{ margin: 0, fontSize: 12, color: '#a0aec0' }}>只参考画风，不复制图片中的人物内容</p>
        {styleReferenceError && (
          <p role="alert" style={{ margin: 0, fontSize: 12, color: '#e53e3e' }}>{styleReferenceError}</p>
        )}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 12, alignItems: 'center', padding: '16px 0' }}>
        {preview ? (
          <img
            alt="canonical base preview"
            src={preview.dataUrl}
            style={{ width: 128, height: 128, imageRendering: 'pixelated', objectFit: 'contain', background: '#f7fafc', borderRadius: 8 }}
          />
        ) : (
          <div style={{ fontSize: 48 }}>{status === 'error' ? '⚠️' : '🧱'}</div>
        )}

        {status === 'idle' && <p style={{ color: '#718096', margin: 0 }}>生成并确认基础图像后，再生成各动画状态。</p>}
        {status === 'generating' && (
          <>
            <p style={{ color: '#4a5568', margin: 0 }}>生成中…</p>
            <div style={{ width: '100%', maxWidth: 360, background: '#e2e8f0', borderRadius: 6, overflow: 'hidden', height: 8 }}>
              <div style={{ width: `${progressPercent}%`, height: '100%', background: '#4f8ef7', transition: 'width 0.3s ease' }} />
            </div>
          </>
        )}
        {status === 'ready' && preview && <p style={{ color: '#38a169', margin: 0 }}>基础图像已就绪 · 色键 {preview.chromaKey}</p>}
        {errorMsg && <p role="alert" style={{ color: '#e53e3e', margin: 0, whiteSpace: 'pre-wrap' }}>{errorMsg}</p>}
      </div>

      <div style={{ display: 'flex', gap: 12, justifyContent: 'flex-end' }}>
        <button
          onClick={onBack}
          disabled={generating}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: generating ? 'not-allowed' : 'pointer' }}
        >
          上一步
        </button>

        {status === 'ready' ? (
          <>
            <button
              onClick={handleGenerate}
              disabled={generating}
              style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #4f8ef7', background: '#fff', color: '#4f8ef7', cursor: 'pointer' }}
            >
              重新生成
            </button>
            <button
              onClick={handleConfirm}
              style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer' }}
            >
              确认基础图像
            </button>
          </>
        ) : (
          <button
            onClick={handleGenerate}
            disabled={generating}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: generating ? '#e2e8f0' : '#4f8ef7', color: '#fff', cursor: generating ? 'not-allowed' : 'pointer' }}
          >
            {status === 'error' ? '重新生成' : '生成基础图像'}
          </button>
        )}
      </div>
    </div>
  );
}
