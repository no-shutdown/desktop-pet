import { useState } from 'react';
import {
  loadSettings, saveSettings,
  getVisionModels, defaultVisionModel, SILICONFLOW_MODELS,
  type AppSettings, type VisionProvider, type ImageProvider,
} from '../../lib/settings';

interface SettingsPanelProps {
  onBack: () => void;
}

const VISION_OPTIONS: { value: VisionProvider; label: string; desc: string }[] = [
  { value: 'skip',      label: '跳过（手动输入）',    desc: '自己描述角色特征' },
  { value: 'anthropic', label: 'Anthropic (Claude)', desc: '效果最佳，支持多个 Claude 模型' },
  { value: 'deepseek',  label: 'DeepSeek',           desc: 'deepseek-vl2 视觉模型' },
  { value: 'kimi',      label: 'Kimi（月之暗面）',    desc: 'moonshot 视觉模型' },
];

const IMAGE_OPTIONS: { value: ImageProvider; label: string; desc: string }[] = [
  { value: 'pollinations', label: 'Pollinations.ai（免费）', desc: '无需 API Key，Flux 模型' },
  { value: 'siliconflow',  label: '硅基流动 SiliconFlow',   desc: '有免费额度，siliconflow.cn' },
  { value: 'localsd',      label: '本地 Stable Diffusion',  desc: 'AUTOMATIC1111 WebUI' },
];

const fieldStyle: React.CSSProperties = {
  width: '100%', padding: '7px 10px', borderRadius: 6,
  border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box',
};

const selectStyle: React.CSSProperties = {
  ...fieldStyle, background: '#fff', cursor: 'pointer',
};

const labelStyle: React.CSSProperties = {
  fontSize: 12, color: '#4a5568', display: 'block', marginBottom: 4,
};

export default function SettingsPanel({ onBack }: SettingsPanelProps) {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  function update(patch: Partial<AppSettings>) {
    setSettings((prev) => ({ ...prev, ...patch }));
  }

  function handleVisionProviderChange(provider: VisionProvider) {
    update({ visionProvider: provider, visionModel: defaultVisionModel(provider) });
  }

  function handleSave() {
    saveSettings(settings);
    onBack();
  }

  const visionModels = getVisionModels(settings.visionProvider);
  const needsVisionKey = settings.visionProvider !== 'skip';
  const needsImageKey  = settings.imageProvider === 'siliconflow';
  const needsSdUrl     = settings.imageProvider === 'localsd';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 32 }}>
        <button
          onClick={onBack}
          style={{ background: 'none', border: 'none', cursor: 'pointer', color: '#718096', fontSize: 20, padding: '0 4px', lineHeight: 1 }}
          title="返回"
        >
          ←
        </button>
        <h2 style={{ margin: 0, fontSize: 20 }}>配置</h2>
        <span style={{ marginLeft: 'auto', fontSize: 12, color: '#a0aec0' }}>设置将作为各步骤的默认值（可随时在步骤中覆盖）</span>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 28 }}>

        {/* Vision Provider */}
        <section style={{ background: '#f7fafc', borderRadius: 10, padding: '20px 24px' }}>
          <h3 style={{ margin: '0 0 4px', fontSize: 13, color: '#4a5568', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            视觉分析服务
          </h3>
          <p style={{ fontSize: 12, color: '#a0aec0', margin: '0 0 16px' }}>
            步骤 2 中用于自动分析参考照片并生成描述。
          </p>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 16 }}>
            {VISION_OPTIONS.map(({ value, label, desc }) => (
              <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 10, cursor: 'pointer' }}>
                <input
                  type="radio"
                  name="visionProvider"
                  value={value}
                  checked={settings.visionProvider === value}
                  onChange={() => handleVisionProviderChange(value)}
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
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div>
                <label style={labelStyle}>API Key</label>
                <input
                  type="password"
                  value={settings.visionApiKey}
                  onChange={(e) => update({ visionApiKey: e.target.value })}
                  placeholder="粘贴 API Key…"
                  style={fieldStyle}
                />
              </div>
              {visionModels.length > 0 && (
                <div>
                  <label style={labelStyle}>模型</label>
                  <select
                    value={settings.visionModel}
                    onChange={(e) => update({ visionModel: e.target.value })}
                    style={selectStyle}
                  >
                    {visionModels.map(({ value, label }) => (
                      <option key={value} value={value}>{label}</option>
                    ))}
                  </select>
                </div>
              )}
            </div>
          )}
        </section>

        {/* Image Provider */}
        <section style={{ background: '#f7fafc', borderRadius: 10, padding: '20px 24px' }}>
          <h3 style={{ margin: '0 0 4px', fontSize: 13, color: '#4a5568', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            图像生成服务
          </h3>
          <p style={{ fontSize: 12, color: '#a0aec0', margin: '0 0 16px' }}>
            步骤 3 中用于生成全部 18 帧动画图像。
          </p>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 16 }}>
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
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div>
                <label style={labelStyle}>API Key</label>
                <input
                  type="password"
                  value={settings.imageApiKey}
                  onChange={(e) => update({ imageApiKey: e.target.value })}
                  placeholder="粘贴硅基流动 API Key…"
                  style={fieldStyle}
                />
              </div>
              <div>
                <label style={labelStyle}>模型</label>
                <select
                  value={settings.imageModel}
                  onChange={(e) => update({ imageModel: e.target.value })}
                  style={selectStyle}
                >
                  {SILICONFLOW_MODELS.map(({ value, label }) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {needsSdUrl && (
            <div>
              <label style={labelStyle}>WebUI 地址</label>
              <input
                type="text"
                value={settings.localSdUrl}
                onChange={(e) => update({ localSdUrl: e.target.value })}
                placeholder="http://localhost:7860"
                style={fieldStyle}
              />
            </div>
          )}
        </section>
      </div>

      {/* Footer */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12, marginTop: 24, paddingTop: 20, borderTop: '1px solid #e2e8f0' }}>
        <button
          onClick={onBack}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
        >
          取消
        </button>
        <button
          onClick={handleSave}
          style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer', fontWeight: 500 }}
        >
          保存
        </button>
      </div>
    </div>
  );
}
