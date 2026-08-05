import { useState } from 'react';
import { loadSettings, saveSettings, type VisionProvider } from '../../../lib/settings';
import { analyzePhotoWithSettings } from '../../../lib/vision';

interface AnalyzeStepProps {
  photoDataUrl: string;
  initialPrompt: string;
  onNext: (prompt: string) => void;
  onBack: () => void;
}

const VISION_OPTIONS: { value: VisionProvider; label: string; desc: string }[] = [
  { value: 'skip',      label: '跳过（手动输入）', desc: '自己描述角色特征' },
  { value: 'anthropic', label: 'Anthropic Claude', desc: '效果最佳，claude-opus-4-7' },
  { value: 'deepseek',  label: 'DeepSeek',         desc: 'deepseek-vl2 视觉模型' },
  { value: 'kimi',      label: 'Kimi（月之暗面）',  desc: 'moonshot-v1-8k-vision-preview' },
];

export default function AnalyzeStep({ photoDataUrl, initialPrompt, onNext, onBack }: AnalyzeStepProps) {
  const [prompt, setPrompt] = useState(initialPrompt);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState(loadSettings);

  function updateSettings(patch: Partial<ReturnType<typeof loadSettings>>) {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      saveSettings(next);
      return next;
    });
  }

  const canAnalyze = settings.visionProvider !== 'skip' && settings.visionApiKey.trim().length > 0;

  async function handleAnalyze() {
    setAnalyzing(true);
    setError(null);
    try {
      const description = await analyzePhotoWithSettings(photoDataUrl, settings);
      setPrompt(description);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setAnalyzing(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', gap: 24, alignItems: 'flex-start' }}>
        <img
          alt="reference"
          src={photoDataUrl}
          style={{ width: 120, height: 120, objectFit: 'cover', borderRadius: 8, flexShrink: 0 }}
        />

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 10 }}>
          <p style={{ margin: 0, fontSize: 12, color: '#718096', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            视觉分析服务
          </p>
          {VISION_OPTIONS.map(({ value, label, desc }) => (
            <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 8, cursor: 'pointer' }}>
              <input
                type="radio"
                name="visionProvider"
                value={value}
                checked={settings.visionProvider === value}
                onChange={() => updateSettings({ visionProvider: value })}
                style={{ marginTop: 3, accentColor: '#4f8ef7' }}
              />
              <div>
                <div style={{ fontSize: 13, fontWeight: 500, color: '#2d3748' }}>{label}</div>
                <div style={{ fontSize: 11, color: '#a0aec0' }}>{desc}</div>
              </div>
            </label>
          ))}
          {settings.visionProvider !== 'skip' && (
            <input
              type="password"
              value={settings.visionApiKey}
              onChange={(e) => updateSettings({ visionApiKey: e.target.value })}
              placeholder="粘贴 API Key…"
              style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, width: '100%', boxSizing: 'border-box' }}
            />
          )}
          {settings.visionProvider !== 'skip' && (
            <button
              onClick={handleAnalyze}
              disabled={!canAnalyze || analyzing}
              style={{
                padding: '6px 16px', borderRadius: 6, border: 'none',
                background: canAnalyze && !analyzing ? '#4f8ef7' : '#e2e8f0',
                color: '#fff', cursor: canAnalyze && !analyzing ? 'pointer' : 'not-allowed',
                fontSize: 13, alignSelf: 'flex-start',
              }}
            >
              {analyzing ? '分析中…' : 'AI 分析照片'}
            </button>
          )}
          {error && <p style={{ color: '#e53e3e', fontSize: 13, margin: 0 }}>{error}</p>}
        </div>
      </div>

      <div>
        <label
          htmlFor="prompt-textarea"
          style={{ fontSize: 13, color: '#4a5568', display: 'block', marginBottom: 4 }}
        >
          角色描述
        </label>
        <textarea
          id="prompt-textarea"
          aria-label="character description"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={5}
          placeholder="例：anime chibi girl, black twin-tail hair, red sailor uniform, blue eyes, cute expression..."
          style={{
            width: '100%', padding: '8px 12px', borderRadius: 6,
            border: '1px solid #e2e8f0', fontSize: 13, resize: 'vertical',
            fontFamily: 'inherit', boxSizing: 'border-box',
          }}
        />
        <p style={{ color: '#a0aec0', fontSize: 12, marginTop: 4 }}>
          {canAnalyze ? 'AI 已生成描述，可手动编辑。' : '请在上方手动输入角色描述。'}
        </p>
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
        <button
          onClick={onBack}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
        >
          上一步
        </button>
        <button
          onClick={() => onNext(prompt)}
          disabled={!prompt.trim()}
          style={{
            padding: '8px 24px', borderRadius: 6, border: 'none',
            background: prompt.trim() ? '#4f8ef7' : '#e2e8f0',
            color: '#fff', cursor: prompt.trim() ? 'pointer' : 'not-allowed',
          }}
        >
          下一步
        </button>
      </div>
    </div>
  );
}
