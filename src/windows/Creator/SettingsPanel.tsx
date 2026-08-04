import { useState } from 'react';
import {
  loadSettings, saveSettings,
  type AppSettings, type VisionProvider, type ImageProvider,
} from '../../lib/settings';

interface SettingsPanelProps {
  onClose: () => void;
}

const VISION_OPTIONS: { value: VisionProvider; label: string; desc: string }[] = [
  { value: 'skip',      label: 'Skip (manual input)',   desc: 'Type the character description yourself' },
  { value: 'anthropic', label: 'Anthropic (Claude)',    desc: 'Best quality vision — claude-opus-4-7' },
  { value: 'deepseek',  label: 'DeepSeek',             desc: 'deepseek-vl2 vision model' },
  { value: 'kimi',      label: 'Kimi (Moonshot)',       desc: 'moonshot-v1-8k-vision-preview' },
];

const IMAGE_OPTIONS: { value: ImageProvider; label: string; desc: string }[] = [
  { value: 'pollinations', label: 'Pollinations.ai (Flux)', desc: 'Free, no API key needed' },
  { value: 'siliconflow',  label: '硅基流动 SiliconFlow',   desc: 'Free quota, Flux / SD models — siliconflow.cn' },
  { value: 'localsd',      label: 'Local Stable Diffusion', desc: 'AUTOMATIC1111 WebUI running locally' },
];

export default function SettingsPanel({ onClose }: SettingsPanelProps) {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  function update(patch: Partial<AppSettings>) {
    setSettings((prev) => ({ ...prev, ...patch }));
  }

  function handleSave() {
    saveSettings(settings);
    onClose();
  }

  const needsVisionKey = settings.visionProvider !== 'skip';
  const needsImageKey  = settings.imageProvider === 'siliconflow';
  const needsSdUrl     = settings.imageProvider === 'localsd';

  return (
    <div
      style={{
        position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.4)',
        display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100,
      }}
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={{
        background: '#fff', borderRadius: 12, padding: 32, width: 520,
        maxHeight: '85vh', overflowY: 'auto',
        boxShadow: '0 20px 60px rgba(0,0,0,0.25)',
      }}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 28 }}>
          <h2 style={{ margin: 0, fontSize: 18 }}>Settings</h2>
          <button
            onClick={onClose}
            style={{ background: 'none', border: 'none', fontSize: 22, cursor: 'pointer', color: '#a0aec0', lineHeight: 1 }}
          >
            ×
          </button>
        </div>

        {/* Vision Provider */}
        <section style={{ marginBottom: 28 }}>
          <h3 style={{ fontSize: 13, color: '#4a5568', marginBottom: 4, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            Vision Provider
          </h3>
          <p style={{ fontSize: 12, color: '#a0aec0', margin: '0 0 14px' }}>
            Used in Step 2 to auto-analyze your reference photo.
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {VISION_OPTIONS.map(({ value, label, desc }) => (
              <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 10, cursor: 'pointer' }}>
                <input
                  type="radio"
                  name="visionProvider"
                  value={value}
                  checked={settings.visionProvider === value}
                  onChange={() => update({ visionProvider: value })}
                  style={{ marginTop: 3, accentColor: '#4f8ef7' }}
                />
                <div>
                  <div style={{ fontSize: 13, fontWeight: 500, color: '#2d3748' }}>{label}</div>
                  <div style={{ fontSize: 11, color: '#a0aec0' }}>{desc}</div>
                </div>
              </label>
            ))}
          </div>
          {needsVisionKey && (
            <div style={{ marginTop: 14 }}>
              <label style={{ fontSize: 12, color: '#4a5568', display: 'block', marginBottom: 4 }}>API Key</label>
              <input
                type="password"
                value={settings.visionApiKey}
                onChange={(e) => update({ visionApiKey: e.target.value })}
                placeholder="Paste API key here…"
                style={{ width: '100%', padding: '8px 12px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
              />
            </div>
          )}
        </section>

        <hr style={{ border: 'none', borderTop: '1px solid #e2e8f0', margin: '0 0 24px' }} />

        {/* Image Provider */}
        <section style={{ marginBottom: 28 }}>
          <h3 style={{ fontSize: 13, color: '#4a5568', marginBottom: 4, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            Image Generation Provider
          </h3>
          <p style={{ fontSize: 12, color: '#a0aec0', margin: '0 0 14px' }}>
            Used in Step 3 to generate animation frames from your description.
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {IMAGE_OPTIONS.map(({ value, label, desc }) => (
              <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 10, cursor: 'pointer' }}>
                <input
                  type="radio"
                  name="imageProvider"
                  value={value}
                  checked={settings.imageProvider === value}
                  onChange={() => update({ imageProvider: value })}
                  style={{ marginTop: 3, accentColor: '#4f8ef7' }}
                />
                <div>
                  <div style={{ fontSize: 13, fontWeight: 500, color: '#2d3748' }}>{label}</div>
                  <div style={{ fontSize: 11, color: '#a0aec0' }}>{desc}</div>
                </div>
              </label>
            ))}
          </div>
          {needsImageKey && (
            <div style={{ marginTop: 14 }}>
              <label style={{ fontSize: 12, color: '#4a5568', display: 'block', marginBottom: 4 }}>API Key</label>
              <input
                type="password"
                value={settings.imageApiKey}
                onChange={(e) => update({ imageApiKey: e.target.value })}
                placeholder="Paste SiliconFlow API key here…"
                style={{ width: '100%', padding: '8px 12px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
              />
            </div>
          )}
          {needsSdUrl && (
            <div style={{ marginTop: 14 }}>
              <label style={{ fontSize: 12, color: '#4a5568', display: 'block', marginBottom: 4 }}>WebUI URL</label>
              <input
                type="text"
                value={settings.localSdUrl}
                onChange={(e) => update({ localSdUrl: e.target.value })}
                placeholder="http://localhost:7860"
                style={{ width: '100%', padding: '8px 12px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
              />
            </div>
          )}
        </section>

        {/* Footer */}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
          <button
            onClick={onClose}
            style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer', fontWeight: 500 }}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
