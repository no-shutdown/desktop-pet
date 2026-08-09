import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { PET_STATES, type PetState } from '../../../types/pet';
import {
  loadSettings,
  saveSettings,
  SILICONFLOW_REFERENCE_MODELS,
  type ImageProvider,
} from '../../../lib/settings';
import type { GeneratedSpriteConfig, GenerationProvider } from './types';

interface StateRowResult {
  runId: string;
  state: string;
  dataUrl: string;
  frameW: number;
  frameH: number;
  frameCount: number;
}

interface AssembleRunPreviewResult {
  runId: string;
  dataUrl: string;
  frameW: number;
  frameH: number;
  frameCount: number;
  rowGap: number;
}

interface GenerationProgress {
  runId: string;
  phase: string;
  state?: string;
  current: number;
  total: number;
}

interface StateGenerationStepProps {
  runId: string;
  baseDataUrl?: string | null;
  onNext: (dataUrl: string, config: GeneratedSpriteConfig) => void;
  onBack: () => void;
  onBusyChange?: (busy: boolean) => void;
}

type Status = 'idle' | 'generating' | 'assembling' | 'error' | 'ready';

const IMAGE_OPTIONS: { value: GenerationProvider; label: string; desc: string }[] = [
  { value: 'siliconflow', label: '硅基流动 SiliconFlow', desc: '云端图像生成' },
  { value: 'localsd', label: '本地 Stable Diffusion', desc: 'AUTOMATIC1111 WebUI' },
];

function supportedProvider(provider: ImageProvider): GenerationProvider {
  return provider === 'localsd' ? 'localsd' : 'siliconflow';
}

function messageFromError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function StateGenerationStep({
  runId,
  baseDataUrl,
  onNext,
  onBack,
  onBusyChange,
}: StateGenerationStepProps) {
  const mountedRef = useRef(true);
  const busyRef = useRef(false);
  const busyCallbackRef = useRef(onBusyChange);
  const [status, setStatus] = useState<Status>('idle');
  const [settings, setSettings] = useState(loadSettings);
  const [completedRows, setCompletedRows] = useState<Partial<Record<PetState, StateRowResult>>>({});
  const [failedState, setFailedState] = useState<PetState | null>(null);
  const [activeState, setActiveState] = useState<PetState | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [progress, setProgress] = useState({ state: null as PetState | null, current: 0, total: 4 });

  busyCallbackRef.current = onBusyChange;

  function reportBusy(busy: boolean) {
    if (busyRef.current === busy) return;
    busyRef.current = busy;
    busyCallbackRef.current?.(busy);
  }

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      reportBusy(false);
    };
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void listen<GenerationProgress>('generation-progress', (event) => {
      const payload = event.payload;
      if (!mountedRef.current || payload.runId !== runId) return;

      const state = PET_STATES.includes(payload.state as PetState)
        ? payload.state as PetState
        : null;
      const stateIndex = state ? PET_STATES.indexOf(state) : -1;
      const current = payload.total === 1 && stateIndex >= 0
        ? stateIndex + 1
        : payload.current;
      setProgress({ state, current, total: 4 });
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
  }, [runId]);

  function updateSettings(patch: Partial<ReturnType<typeof loadSettings>>) {
    setSettings((previous) => {
      const next = { ...previous, ...patch };
      saveSettings(next);
      return next;
    });
  }

  async function handleGenerate() {
    if (status === 'generating' || status === 'assembling') return;

    const pendingStates = PET_STATES.filter((state) => !completedRows[state]);
    reportBusy(true);
    setStatus('generating');
    setErrorMsg(null);
    setFailedState(null);
    setProgress({ state: pendingStates[0] ?? null, current: 4 - pendingStates.length, total: 4 });

    let currentState: PetState | null = null;
    try {
      for (const state of pendingStates) {
        currentState = state;
        setActiveState(state);
        const result = await invoke<StateRowResult>('generate_state_row', {
          runId,
          state,
          imageProvider: supportedProvider(settings.imageProvider),
          imageApiKey: settings.imageApiKey || null,
          referenceModel: settings.imageReferenceModel || null,
          localSdUrl: settings.localSdUrl || null,
          denoisingStrength: settings.localSdDenoisingStrength,
        });
        if (!mountedRef.current) return;
        setCompletedRows((previous) => ({ ...previous, [state]: result }));
        setProgress({ state, current: PET_STATES.indexOf(state) + 1, total: 4 });
      }

      if (!mountedRef.current) return;
      currentState = null;
      setActiveState(null);
      setStatus('assembling');
      const assembled = await invoke<AssembleRunPreviewResult>('assemble_run_preview', { runId });
      if (!mountedRef.current) return;
      const config: GeneratedSpriteConfig = {
        petId: runId,
        runId,
        frameW: 128,
        frameH: 128,
        rowGap: 0,
        layout: 'horizontalRows',
        idleFrames: 8,
        walkingFrames: 8,
        wavingFrames: 8,
        workingFrames: 8,
      };
      setStatus('ready');
      reportBusy(false);
      onNext(assembled.dataUrl, config);
    } catch (error) {
      if (!mountedRef.current) return;
      setActiveState(null);
      setFailedState(currentState);
      setErrorMsg(messageFromError(error));
      setStatus('error');
    } finally {
      reportBusy(false);
    }
  }

  const provider = supportedProvider(settings.imageProvider);
  const busy = status === 'generating' || status === 'assembling';
  const completedCount = PET_STATES.filter((state) => completedRows[state]).length;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div style={{ background: '#f7fafc', borderRadius: 10, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 10 }}>
        <p style={{ margin: 0, fontSize: 12, color: '#718096', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          生成动画状态行
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
              aria-label="图像 API Key"
              value={settings.imageApiKey}
              onChange={(event) => updateSettings({ imageApiKey: event.target.value })}
              placeholder="图像 API Key"
              style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
            />
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12, color: '#718096' }}>
              参考模型
              <select
                aria-label="参考模型"
                value={settings.imageReferenceModel}
                onChange={(event) => updateSettings({ imageReferenceModel: event.target.value })}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {SILICONFLOW_REFERENCE_MODELS.map(({ value, label }) => (
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

      {baseDataUrl && (
        <img
          alt="canonical base"
          src={baseDataUrl}
          style={{ alignSelf: 'center', width: 96, height: 96, imageRendering: 'pixelated', objectFit: 'contain', background: '#f7fafc', borderRadius: 8 }}
        />
      )}

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12 }}>
        {PET_STATES.map((state) => {
          const row = completedRows[state];
          const failed = failedState === state;
          return (
            <div key={state} style={{ display: 'flex', flexDirection: 'column', gap: 6, alignItems: 'center' }}>
              <div style={{ width: '100%', minHeight: 96, border: '1px solid #e2e8f0', borderRadius: 8, background: '#f7fafc', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {row ? (
                  <img
                    alt={`${state} state row`}
                    src={row.dataUrl}
                    style={{ maxWidth: '100%', maxHeight: 96, imageRendering: 'pixelated' }}
                  />
                ) : (
                  <span style={{ color: failed ? '#e53e3e' : '#a0aec0', fontSize: 12 }}>
                    {failed ? '失败' : '待生成'}
                  </span>
                )}
              </div>
              <span style={{ fontSize: 12, color: failed ? '#e53e3e' : '#4a5568' }}>
                {state}：{row ? '已完成' : failed ? '失败' : '待生成'}
              </span>
            </div>
          );
        })}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center' }}>
        {busy && <p style={{ color: '#4a5568', margin: 0 }}>{activeState ? `正在生成 ${activeState}…` : '正在组合预览图…'}</p>}
        <p style={{ color: '#718096', margin: 0 }}>进度：{progress.current} / {progress.total}</p>
        {completedCount > 0 && !busy && <p style={{ color: '#38a169', margin: 0 }}>{completedCount} / 4 状态行已完成。</p>}
        {errorMsg && <p role="alert" style={{ color: '#e53e3e', margin: 0, whiteSpace: 'pre-wrap' }}>{errorMsg}</p>}
      </div>

      <div style={{ display: 'flex', gap: 12, justifyContent: 'flex-end' }}>
        <button
          onClick={onBack}
          disabled={busy}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: busy ? 'not-allowed' : 'pointer' }}
        >
          返回基础图像
        </button>

        {failedState ? (
          <button
            onClick={handleGenerate}
            disabled={busy}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: busy ? '#e2e8f0' : '#4f8ef7', color: '#fff', cursor: busy ? 'not-allowed' : 'pointer' }}
          >
            重试 {failedState}
          </button>
        ) : (
          <button
            onClick={handleGenerate}
            disabled={busy || status === 'ready'}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: busy || status === 'ready' ? '#e2e8f0' : '#4f8ef7', color: '#fff', cursor: busy || status === 'ready' ? 'not-allowed' : 'pointer' }}
          >
            {status === 'error' ? '重新生成' : '生成所有状态'}
          </button>
        )}
      </div>
    </div>
  );
}
