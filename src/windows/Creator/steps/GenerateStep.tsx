import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { loadSettings, saveSettings, SILICONFLOW_MODELS, type ImageProvider } from '../../../lib/settings';
import type { PetState, SpriteStateInfo } from '../../../types/pet';

interface GenerateStepProps {
  prompt: string;
  onNext: (petId: string, states: Record<PetState, SpriteStateInfo>) => void;
  onBack: () => void;
}

type Status = 'idle' | 'generating' | 'done' | 'error';

const TOTAL_STATES = 4;

const IMAGE_OPTIONS: { value: ImageProvider; label: string; desc: string }[] = [
  { value: 'pollinations', label: 'Pollinations.ai（免费）', desc: '无需 API Key，Flux 模型' },
  { value: 'siliconflow',  label: '硅基流动',               desc: '有免费额度，siliconflow.cn' },
  { value: 'localsd',      label: '本地 Stable Diffusion',  desc: 'AUTOMATIC1111 WebUI' },
];

export default function GenerateStep({ prompt, onNext, onBack }: GenerateStepProps) {
  const [status, setStatus] = useState<Status>('idle');
  const [progress, setProgress] = useState({ current: 0, total: TOTAL_STATES });
  const [petId, setPetId] = useState<string | null>(null);
  const [statesResult, setStatesResult] = useState<Record<PetState, SpriteStateInfo> | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [settings, setSettings] = useState(loadSettings);

  function updateSettings(patch: Partial<ReturnType<typeof loadSettings>>) {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      saveSettings(next);
      return next;
    });
  }

  async function handleGenerate() {
    const id = crypto.randomUUID();
    setPetId(id);
    setStatesResult(null);
    setStatus('generating');
    setProgress({ current: 0, total: TOTAL_STATES });

    const unlisten = await listen<{ current: number; total: number }>(
      'generation-progress',
      (event) => setProgress(event.payload)
    );

    try {
      const states = await invoke<Record<PetState, SpriteStateInfo>>('generate_and_assemble', {
        petId: id,
        basePrompt: prompt,
        imageProvider: settings.imageProvider,
        imageApiKey: settings.imageApiKey || null,
        imageModel: settings.imageModel || null,
        localSdUrl: settings.localSdUrl || null,
      });
      setStatesResult(states);
      setStatus('done');
    } catch (err) {
      setErrorMsg((err as Error).message ?? String(err));
      setStatus('error');
    } finally {
      unlisten();
    }
  }

  const pct = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;
  const generating = status === 'generating';
  const showConfig = status === 'idle' || status === 'error';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      {showConfig && (
        <div style={{ background: '#f7fafc', borderRadius: 10, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 10 }}>
          <p style={{ margin: 0, fontSize: 12, color: '#718096', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            图像生成服务
          </p>
          {IMAGE_OPTIONS.map(({ value, label, desc }) => (
            <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 8, cursor: 'pointer' }}>
              <input
                type="radio"
                name="imageProvider"
                value={value}
                checked={settings.imageProvider === value}
                onChange={() => updateSettings({ imageProvider: value })}
                style={{ marginTop: 3, accentColor: '#4f8ef7' }}
              />
              <div>
                <div style={{ fontSize: 13, fontWeight: 500, color: '#2d3748' }}>{label}</div>
                <div style={{ fontSize: 11, color: '#a0aec0' }}>{desc}</div>
              </div>
            </label>
          ))}

          {settings.imageProvider === 'siliconflow' && (
            <>
              <input
                type="password"
                value={settings.imageApiKey}
                onChange={(e) => updateSettings({ imageApiKey: e.target.value })}
                placeholder="粘贴硅基流动 API Key…"
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
              />
              <select
                value={settings.imageModel}
                onChange={(e) => updateSettings({ imageModel: e.target.value })}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {SILICONFLOW_MODELS.map(({ value, label }) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </>
          )}

          {settings.imageProvider === 'localsd' && (
            <input
              type="text"
              value={settings.localSdUrl}
              onChange={(e) => updateSettings({ localSdUrl: e.target.value })}
              placeholder="http://localhost:7860"
              style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
            />
          )}
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center', padding: '16px 0' }}>
        {status === 'idle' && (
          <>
            <div style={{ fontSize: 48 }}>✨</div>
            <p style={{ color: '#718096', textAlign: 'center', margin: 0 }}>
              将生成 4 个动画状态，每个状态整体生成一张精灵图再保存为 PNG。
              <br />
              <span style={{ fontSize: 13, color: '#a0aec0' }}>预计需要 1～3 分钟。</span>
            </p>
          </>
        )}

        {status === 'generating' && (
          <>
            <div style={{ fontSize: 48 }}>⏳</div>
            <p style={{ color: '#4a5568', margin: 0 }}>
              正在生成第 {progress.current} / {progress.total} 个动画状态…
            </p>
            <div style={{ width: '100%', maxWidth: 360, background: '#e2e8f0', borderRadius: 6, overflow: 'hidden', height: 8 }}>
              <div style={{ width: `${pct}%`, height: '100%', background: '#4f8ef7', transition: 'width 0.3s ease' }} />
            </div>
            <p style={{ color: '#a0aec0', fontSize: 13, margin: 0 }}>{pct}%</p>
          </>
        )}

        {status === 'done' && (
          <>
            <div style={{ fontSize: 48 }}>🎉</div>
            <p style={{ color: '#38a169', margin: 0 }}>全部动画已生成！</p>
          </>
        )}

        {status === 'error' && (
          <>
            <div style={{ fontSize: 48 }}>⚠️</div>
            <p style={{ color: '#e53e3e', margin: 0 }}>{errorMsg}</p>
          </>
        )}
      </div>

      <div style={{ display: 'flex', gap: 12, justifyContent: 'flex-end' }}>
        <button
          onClick={onBack}
          disabled={generating}
          style={{
            padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0',
            background: '#fff', color: '#4a5568',
            cursor: generating ? 'not-allowed' : 'pointer',
          }}
        >
          上一步
        </button>

        {status === 'done' ? (
          <button
            onClick={() => petId && statesResult && onNext(petId, statesResult)}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer' }}
          >
            下一步
          </button>
        ) : (
          <button
            onClick={handleGenerate}
            disabled={generating}
            style={{
              padding: '8px 24px', borderRadius: 6, border: 'none',
              background: generating ? '#e2e8f0' : '#4f8ef7',
              color: '#fff', cursor: generating ? 'not-allowed' : 'pointer',
            }}
          >
            {status === 'error' ? '重试' : '开始生成'}
          </button>
        )}
      </div>
    </div>
  );
}
