import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { PET_STATES, PET_STATE_CATALOG, PET_STATE_LABELS, type PetState } from '../../../types/pet';
import {
  apiKeyForProvider,
  loadSettings,
  rowModelForProvider,
  saveSettings,
  SILICONFLOW_REFERENCE_MODELS,
  WANXIANG_EDIT_MODELS,
  type AppSettings,
  type ImageProvider,
} from '../../../lib/settings';
import SpriteAnimator from '../../Pet/SpriteAnimator';
import type { GeneratedSpriteConfig, GenerationProvider } from './types';

interface StateRowResult {
  runId: string;
  state: string;
  dataUrl: string;
  frameW: number;
  frameH: number;
  frameCount: number;
}

interface StatePromptPreviewResult {
  runId: string;
  state: string;
  frameCount: number;
  prompts: string[];
}

interface StateProbeResult extends StateRowResult {
  validation: {
    passed: boolean;
    maxCenterDrift: number;
    maxBaselineDrift: number;
    minChangedPixels: number;
  };
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
  { value: 'wanxiang', label: '阿里云万相', desc: 'DashScope wanx 图像编辑' },
  { value: 'localsd', label: '本地 Stable Diffusion', desc: 'AUTOMATIC1111 WebUI' },
];

const STATE_DELAY_MS: Record<PetState, number> = PET_STATE_CATALOG.reduce(
  (accumulator, { key, delayMs }) => {
    accumulator[key] = delayMs;
    return accumulator;
  },
  {} as Record<PetState, number>,
);

const ANIMATED_STATES = PET_STATES.filter((state) => state !== 'idle');

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

function rowModelPatchForProvider(
  provider: GenerationProvider,
  value: string,
): Partial<AppSettings> {
  return provider === 'wanxiang'
    ? { wanxiangEditModel: value }
    : { imageReferenceModel: value };
}

function messageFromError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function buildGeneratedConfig(runId: string): GeneratedSpriteConfig {
  return {
    petId: runId,
    runId,
    frameW: 128,
    frameH: 128,
    rowGap: 0,
    layout: 'horizontalRows',
    idleFrames: 8,
    sleepingFrames: 8,
    actingCuteFrames: 8,
    workingFrames: 8,
  };
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
  const [failedStates, setFailedStates] = useState<Partial<Record<PetState, true>>>({});
  const [selectedProbeState, setSelectedProbeState] = useState<PetState>(
    ANIMATED_STATES[0] ?? 'sleeping',
  );
  const [promptPreview, setPromptPreview] = useState<StatePromptPreviewResult | null>(null);
  const [probeRows, setProbeRows] = useState<Partial<Record<PetState, StateProbeResult>>>({});
  const [approvedProbeStates, setApprovedProbeStates] = useState<Partial<Record<PetState, true>>>({});
  const [activeState, setActiveState] = useState<PetState | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [progress, setProgress] = useState({ state: null as PetState | null, current: 0, total: 4 });
  const [assembledPreview, setAssembledPreview] = useState<AssembleRunPreviewResult | null>(null);
  const [viewerState, setViewerState] = useState<PetState | 'combined' | null>(null);
  const busy = status === 'generating' || status === 'assembling';
  const hasApprovedProbe = ANIMATED_STATES.some((state) => (
    Boolean(probeRows[state] && approvedProbeStates[state])
  ));

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

  useEffect(() => {
    if (!viewerState) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') setViewerState(null);
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [viewerState]);

  function updateSettings(patch: Partial<ReturnType<typeof loadSettings>>) {
    setSettings((previous) => {
      const next = { ...previous, ...patch };
      saveSettings(next);
      return next;
    });
    setPromptPreview(null);
    setProbeRows({});
    setApprovedProbeStates({});
  }

  function generationArgs(state: PetState, reuseProbe = false) {
    const providerChoice = settings.rowImageProvider ?? settings.imageProvider;
    const args = {
      runId,
      state,
      imageProvider: supportedProvider(providerChoice),
      imageApiKey: apiKeyForProvider(settings, providerChoice) || null,
      referenceModel: rowModelForProvider(settings, providerChoice) || null,
      localSdUrl: settings.localSdUrl || null,
      denoisingStrength: settings.localSdDenoisingStrength,
    };
    return reuseProbe ? { ...args, reuseProbe: true } : args;
  }

  async function reassemblePreview() {
    const assembled = await invoke<AssembleRunPreviewResult>('assemble_run_preview', { runId });
    if (!mountedRef.current) return;
    setAssembledPreview(assembled);
  }

  async function handlePreviewPrompts() {
    if (busy) return;
    setErrorMsg(null);
    try {
      const result = await invoke<StatePromptPreviewResult>('preview_state_prompts', {
        runId,
        state: selectedProbeState,
      });
      if (!mountedRef.current) return;
      setPromptPreview(result);
    } catch (error) {
      if (!mountedRef.current) return;
      setErrorMsg(messageFromError(error));
    }
  }

  async function handleProbe() {
    if (busy) return;

    setProbeRows((previous) => {
      const next = { ...previous };
      delete next[selectedProbeState];
      return next;
    });
    setApprovedProbeStates((previous) => {
      const next = { ...previous };
      delete next[selectedProbeState];
      return next;
    });
    reportBusy(true);
    setStatus('generating');
    setErrorMsg(null);
    setActiveState(selectedProbeState);
    setProgress({ state: selectedProbeState, current: 0, total: 4 });

    try {
      const result = await invoke<StateProbeResult>(
        'generate_state_probe',
        generationArgs(selectedProbeState),
      );
      if (!mountedRef.current) return;
      setProbeRows((previous) => ({ ...previous, [selectedProbeState]: result }));
      setApprovedProbeStates((previous) => {
        const next = { ...previous };
        delete next[selectedProbeState];
        return next;
      });
      setActiveState(null);
      setStatus('ready');
    } catch (error) {
      if (!mountedRef.current) return;
      setActiveState(null);
      setErrorMsg(messageFromError(error));
      setStatus('error');
    } finally {
      reportBusy(false);
    }
  }

  function handleApproveProbe() {
    const probe = probeRows[selectedProbeState];
    if (!probe?.validation.passed) return;
    setApprovedProbeStates((previous) => ({
      ...previous,
      [selectedProbeState]: true,
    }));
  }

  async function handleGenerate() {
    if (status === 'generating' || status === 'assembling') return;

    if (!hasApprovedProbe) return;

    const pendingStates = PET_STATES.filter((state) => !completedRows[state]);
    if (pendingStates.length === 0) return;

    reportBusy(true);
    setStatus('generating');
    setErrorMsg(null);
    setFailedStates({});
    setProgress({ state: pendingStates[0] ?? null, current: 4 - pendingStates.length, total: 4 });

    let currentState: PetState | null = null;
    try {
      for (const state of pendingStates) {
        currentState = state;
        setActiveState(state);
        const result = await invoke<StateRowResult>(
          'generate_state_row',
          generationArgs(state, Boolean(probeRows[state] && approvedProbeStates[state])),
        );
        if (!mountedRef.current) return;
        setCompletedRows((previous) => ({ ...previous, [state]: result }));
        setProgress({ state, current: PET_STATES.indexOf(state) + 1, total: 4 });
      }

      if (!mountedRef.current) return;
      currentState = null;
      setActiveState(null);
      setStatus('assembling');
      await reassemblePreview();
      if (!mountedRef.current) return;
      setStatus('ready');
    } catch (error) {
      if (!mountedRef.current) return;
      setActiveState(null);
      if (currentState) setFailedStates((previous) => ({ ...previous, [currentState!]: true }));
      setErrorMsg(messageFromError(error));
      setStatus('error');
    } finally {
      reportBusy(false);
    }
  }

  async function handleRegenerateOne(state: PetState) {
    if (status === 'generating' || status === 'assembling') return;

    reportBusy(true);
    setStatus('generating');
    setErrorMsg(null);
    setFailedStates((previous) => {
      const next = { ...previous };
      delete next[state];
      return next;
    });
    setActiveState(state);
    setProgress({ state, current: PET_STATES.indexOf(state) + 1, total: 4 });

    try {
      const result = await invoke<StateRowResult>(
        'generate_state_row',
        generationArgs(state, Boolean(probeRows[state] && approvedProbeStates[state])),
      );
      if (!mountedRef.current) return;
      setCompletedRows((previous) => ({ ...previous, [state]: result }));
      setActiveState(null);

      const allComplete = PET_STATES.every((current) =>
        current === state ? true : Boolean(completedRows[current]),
      );
      if (allComplete) {
        setStatus('assembling');
        await reassemblePreview();
        if (!mountedRef.current) return;
      }
      setStatus('ready');
    } catch (error) {
      if (!mountedRef.current) return;
      setActiveState(null);
      setFailedStates((previous) => ({ ...previous, [state]: true }));
      setErrorMsg(messageFromError(error));
      setStatus('error');
    } finally {
      reportBusy(false);
    }
  }

  function handleConfirm() {
    if (!assembledPreview) return;
    onNext(assembledPreview.dataUrl, buildGeneratedConfig(runId));
  }

  const provider = supportedProvider(settings.rowImageProvider ?? settings.imageProvider);
  const completedCount = PET_STATES.filter((state) => completedRows[state]).length;
  const allComplete = completedCount === 4;
  const selectedProbe = probeRows[selectedProbeState];
  const canConfirm = Boolean(assembledPreview) && allComplete && !busy;

  const viewerRow = useMemo(() => {
    if (!viewerState || viewerState === 'combined') return null;
    return completedRows[viewerState] ?? null;
  }, [viewerState, completedRows]);

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
              onChange={() => updateSettings({ rowImageProvider: value })}
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
              参考模型
              <select
                aria-label="SiliconFlow 参考模型"
                value={settings.imageReferenceModel}
                onChange={(event) => updateSettings(rowModelPatchForProvider(provider, event.target.value))}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {SILICONFLOW_REFERENCE_MODELS.map(({ value, label }) => (
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
              图像编辑模型
              <select
                aria-label="万相图像编辑模型"
                value={settings.wanxiangEditModel}
                onChange={(event) => updateSettings(rowModelPatchForProvider(provider, event.target.value))}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {WANXIANG_EDIT_MODELS.map(({ value, label }) => (
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

      <div style={{ background: '#fffaf0', border: '1px solid #f6ad55', borderRadius: 10, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 10 }}>
        <p style={{ margin: 0, fontSize: 13, color: '#9c4221', fontWeight: 600 }}>
          先检测一个动作，再继续完整生成
        </p>
        <p style={{ margin: 0, fontSize: 12, color: '#975a16', lineHeight: 1.5 }}>
          预览提示词不会调用 API；动作检测只连续生成 4 帧，用来确认提示词、动作连贯性和画面是否漂移。
        </p>
        <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: '#744210' }}>
          检测动作
          <select
            aria-label="检测动作"
            value={selectedProbeState}
            onChange={(event) => {
              setSelectedProbeState(event.target.value as PetState);
              setPromptPreview(null);
              setErrorMsg(null);
            }}
            disabled={busy}
            style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #f6ad55', background: '#fff', cursor: busy ? 'not-allowed' : 'pointer' }}
          >
            {ANIMATED_STATES.map((state) => (
              <option key={state} value={state}>{PET_STATE_LABELS[state]}</option>
            ))}
          </select>
        </label>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          <button
            type="button"
            aria-label="查看检测动作提示词"
            onClick={handlePreviewPrompts}
            disabled={busy}
            style={{ padding: '6px 12px', borderRadius: 6, border: '1px solid #f6ad55', background: '#fff', color: '#975a16', cursor: busy ? 'not-allowed' : 'pointer' }}
          >
            查看提示词（0 次 API）
          </button>
          <button
            type="button"
            aria-label="生成 4 帧检测"
            onClick={handleProbe}
            disabled={busy}
            style={{ padding: '6px 12px', borderRadius: 6, border: 'none', background: busy ? '#fbd38d' : '#ed8936', color: '#fff', cursor: busy ? 'not-allowed' : 'pointer' }}
          >
            生成 4 帧检测
          </button>
        </div>

        {promptPreview?.state === selectedProbeState && (
          <details open style={{ background: '#fff', borderRadius: 6, padding: '8px 10px' }}>
            <summary style={{ cursor: 'pointer', color: '#744210', fontSize: 12 }}>
              {promptPreview.frameCount} 帧最终提示词
            </summary>
            <pre style={{ maxHeight: 220, overflow: 'auto', whiteSpace: 'pre-wrap', margin: '8px 0 0', fontSize: 11, lineHeight: 1.5, color: '#4a5568' }}>
              {promptPreview.prompts.map((prompt, index) => `第 ${index + 1} 帧\n${prompt}`).join('\n\n')}
            </pre>
          </details>
        )}

        {selectedProbe && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center', background: '#fff', borderRadius: 8, padding: 10 }}>
            <span style={{ fontSize: 12, color: '#276749', fontWeight: 600 }}>
              {selectedProbe.validation.passed ? '连续性预检通过' : '连续性预检未通过'}
            </span>
            <SpriteAnimator
              sheetSrc={selectedProbe.dataUrl}
              meta={{
                cols: selectedProbe.frameCount,
                rows: 1,
                frameCount: selectedProbe.frameCount,
                frameW: selectedProbe.frameW,
                frameH: selectedProbe.frameH,
                delayMs: STATE_DELAY_MS[selectedProbeState],
              }}
              displayW={selectedProbe.frameW * 2}
              displayH={selectedProbe.frameH * 2}
            />
            <img
              alt={`${selectedProbeState} 4-frame probe`}
              src={selectedProbe.dataUrl}
              style={{ maxWidth: '80vw', imageRendering: 'pixelated', border: '1px solid #e2e8f0', borderRadius: 6, background: '#f7fafc' }}
            />
            <span style={{ fontSize: 11, color: '#718096' }}>
              中心最大漂移 {selectedProbe.validation.maxCenterDrift}px；底线最大漂移 {selectedProbe.validation.maxBaselineDrift}px；相邻帧最少变化 {selectedProbe.validation.minChangedPixels} 像素
            </span>
            <button
              type="button"
              aria-label="确认检测通过，继续生成"
              onClick={handleApproveProbe}
              disabled={busy || !selectedProbe.validation.passed || Boolean(approvedProbeStates[selectedProbeState])}
              style={{ padding: '6px 14px', borderRadius: 6, border: 'none', background: approvedProbeStates[selectedProbeState] ? '#68d391' : '#38a169', color: '#fff', cursor: busy || Boolean(approvedProbeStates[selectedProbeState]) ? 'default' : 'pointer' }}
            >
              {approvedProbeStates[selectedProbeState] ? '检测已确认' : '确认检测通过，继续生成'}
            </button>
          </div>
        )}

        {hasApprovedProbe && (
          <p style={{ margin: 0, fontSize: 12, color: '#276749' }}>
            已确认一个动作的提示词和连续性；现在可以手动启动完整状态生成。
          </p>
        )}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12 }}>
        {PET_STATES.map((state) => {
          const row = completedRows[state];
          const failed = Boolean(failedStates[state]);
          const isActive = activeState === state;
          const canRegenerate = Boolean(row || failed)
            && !busy
            && (state === 'idle' || hasApprovedProbe);
          const canOpenViewer = Boolean(row) && !busy;
          return (
            <div key={state} style={{ display: 'flex', flexDirection: 'column', gap: 6, alignItems: 'center' }}>
              <button
                type="button"
                onClick={() => canOpenViewer && setViewerState(state)}
                disabled={!canOpenViewer}
                aria-label={`查看 ${PET_STATE_LABELS[state]} 动画预览`}
                title={canOpenViewer ? '点击查看大图与动画预览' : undefined}
                style={{
                  width: '100%',
                  minHeight: 96,
                  border: '1px solid #e2e8f0',
                  borderRadius: 8,
                  background: '#f7fafc',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: 4,
                  cursor: canOpenViewer ? 'zoom-in' : 'default',
                }}
              >
                {isActive ? (
                  <span style={{ color: '#4a5568', fontSize: 12 }}>生成中…</span>
                ) : row ? (
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
              </button>
              <span style={{ fontSize: 12, color: failed ? '#e53e3e' : '#4a5568' }}>
                {PET_STATE_LABELS[state]}：{isActive ? '生成中' : row ? '已完成' : failed ? '失败' : '待生成'}
              </span>
              <button
                type="button"
                onClick={() => handleRegenerateOne(state)}
                disabled={!canRegenerate}
                aria-label={`重新生成 ${PET_STATE_LABELS[state]}`}
                style={{
                  padding: '4px 10px',
                  fontSize: 11,
                  borderRadius: 4,
                  border: '1px solid #e2e8f0',
                  background: canRegenerate ? '#fff' : '#f7fafc',
                  color: canRegenerate ? '#4a5568' : '#cbd5e0',
                  cursor: canRegenerate ? 'pointer' : 'not-allowed',
                }}
              >
                🔄 重新生成
              </button>
            </div>
          );
        })}
      </div>

      {assembledPreview && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, alignItems: 'center' }}>
          <p style={{ margin: 0, fontSize: 12, color: '#718096' }}>组合预览（点击查看大图）</p>
          <button
            type="button"
            onClick={() => !busy && setViewerState('combined')}
            disabled={busy}
            style={{ padding: 4, border: '1px solid #e2e8f0', borderRadius: 8, background: '#f7fafc', cursor: busy ? 'default' : 'zoom-in' }}
          >
            <img
              alt="combined preview"
              src={assembledPreview.dataUrl}
              style={{ maxWidth: '100%', maxHeight: 240, imageRendering: 'pixelated', display: 'block' }}
            />
          </button>
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center' }}>
        {busy && <p style={{ color: '#4a5568', margin: 0 }}>{activeState ? `正在生成 ${PET_STATE_LABELS[activeState]}…` : '正在组合预览图…'}</p>}
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

        {!allComplete && (
          <button
            onClick={handleGenerate}
            disabled={busy || !hasApprovedProbe}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: busy || !hasApprovedProbe ? '#e2e8f0' : '#4f8ef7', color: '#fff', cursor: busy || !hasApprovedProbe ? 'not-allowed' : 'pointer' }}
          >
            {completedCount === 0 ? '生成所有状态' : `生成剩余 ${4 - completedCount} 个`}
          </button>
        )}

        {allComplete && (
          <button
            onClick={handleConfirm}
            disabled={!canConfirm}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: canConfirm ? '#38a169' : '#e2e8f0', color: '#fff', cursor: canConfirm ? 'pointer' : 'not-allowed' }}
          >
            下一步
          </button>
        )}
      </div>

      {viewerState && (
        <div
          onClick={() => setViewerState(null)}
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(15, 23, 42, 0.75)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 1000,
            padding: 24,
          }}
        >
          <div
            onClick={(event) => event.stopPropagation()}
            style={{
              background: '#fff',
              borderRadius: 12,
              padding: 24,
              maxWidth: '90vw',
              maxHeight: '90vh',
              overflow: 'auto',
              display: 'flex',
              flexDirection: 'column',
              gap: 16,
              alignItems: 'center',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', width: '100%', gap: 16 }}>
              <h3 style={{ margin: 0, fontSize: 16, color: '#1a202c' }}>
                {viewerState === 'combined'
                  ? '组合预览'
                  : `${PET_STATE_LABELS[viewerState]} 动画预览`}
              </h3>
              <button
                type="button"
                onClick={() => setViewerState(null)}
                style={{ padding: '4px 12px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', cursor: 'pointer', color: '#4a5568', fontSize: 13 }}
              >
                关闭
              </button>
            </div>

            {viewerState !== 'combined' && viewerRow && (
              <>
                <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
                  <span style={{ fontSize: 12, color: '#718096' }}>动画播放</span>
                  <div style={{ padding: 12, border: '1px solid #e2e8f0', borderRadius: 8, background: '#f7fafc' }}>
                    <SpriteAnimator
                      sheetSrc={viewerRow.dataUrl}
                      meta={{
                        cols: viewerRow.frameCount,
                        rows: 1,
                        frameCount: viewerRow.frameCount,
                        frameW: viewerRow.frameW,
                        frameH: viewerRow.frameH,
                        delayMs: STATE_DELAY_MS[viewerState],
                      }}
                      displayW={viewerRow.frameW * 2}
                      displayH={viewerRow.frameH * 2}
                    />
                  </div>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
                  <span style={{ fontSize: 12, color: '#718096' }}>逐帧序列</span>
                  <img
                    alt={`${viewerState} full row`}
                    src={viewerRow.dataUrl}
                    style={{ maxWidth: '80vw', imageRendering: 'pixelated', border: '1px solid #e2e8f0', borderRadius: 8, background: '#f7fafc' }}
                  />
                </div>
              </>
            )}

            {viewerState === 'combined' && assembledPreview && (
              <img
                alt="combined preview large"
                src={assembledPreview.dataUrl}
                style={{ maxWidth: '80vw', maxHeight: '70vh', imageRendering: 'pixelated', border: '1px solid #e2e8f0', borderRadius: 8, background: '#f7fafc' }}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}
