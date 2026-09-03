import { useState } from 'react';
import {
  loadSettings, saveSettings,
  getVisionModels, defaultVisionModel,
  SILICONFLOW_BASE_MODELS, SILICONFLOW_REFERENCE_MODELS,
  WANXIANG_BASE_MODELS, WANXIANG_EDIT_MODELS,
  LOCAL_SD_DENOISING_MIN, LOCAL_SD_DENOISING_MAX,
  normalizeDenoisingStrength,
  type AppSettings, type VisionProvider, type ImageProvider,
} from '../../lib/settings';

interface SettingsPanelProps {
  onBack: () => void;
}

const VISION_OPTIONS: { value: VisionProvider; label: string; desc: string }[] = [
  { value: 'skip',      label: '跳过（手动输入）',    desc: '自己描述角色特征' },
  { value: 'anthropic', label: 'Anthropic (Claude)', desc: '效果最佳，支持多个 Claude 模型' },
  { value: 'deepseek',  label: 'DeepSeek',           desc: 'DeepSeek V4 Flash Vision 视觉模型' },
  { value: 'kimi',      label: 'Kimi（月之暗面）',    desc: 'Kimi K2.6 / K3 视觉模型' },
];

const IMAGE_OPTIONS: { value: Exclude<ImageProvider, 'pollinations'>; label: string; desc: string }[] = [
  { value: 'siliconflow',  label: '硅基流动 SiliconFlow',   desc: '有免费额度，siliconflow.cn' },
  { value: 'wanxiang',     label: '阿里云万相',              desc: 'DashScope wanx 系列（异步任务）' },
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
  const activeProviders = new Set<ImageProvider>([
    settings.imageProvider,
    settings.rowImageProvider,
  ]);
  const needsSiliconflow = activeProviders.has('siliconflow');
  const needsWanxiang = activeProviders.has('wanxiang');
  const needsSdUrl = activeProviders.has('localsd');

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
            步骤 3 生成 1 张基础图，步骤 4 生成 4 行动画（可用不同 provider）。
          </p>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 12 }}>
            <label style={{ ...labelStyle, fontWeight: 600 }}>基础图 provider（步骤 3）</label>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {IMAGE_OPTIONS.map(({ value, label }) => (
                <label key={value} style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
                  <input
                    type="radio"
                    name="baseImageProvider"
                    value={value}
                    checked={settings.imageProvider === value}
                    onChange={() => update({ imageProvider: value })}
                    style={{ accentColor: '#4f8ef7' }}
                  />
                  <span style={{ fontSize: 13, color: '#2d3748' }}>{label}</span>
                </label>
              ))}
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginBottom: 20 }}>
            <label style={{ ...labelStyle, fontWeight: 600 }}>动画行 provider（步骤 4）</label>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {IMAGE_OPTIONS.map(({ value, label }) => (
                <label key={value} style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
                  <input
                    type="radio"
                    name="rowImageProvider"
                    value={value}
                    checked={settings.rowImageProvider === value}
                    onChange={() => update({ rowImageProvider: value })}
                    style={{ accentColor: '#4f8ef7' }}
                  />
                  <span style={{ fontSize: 13, color: '#2d3748' }}>{label}</span>
                </label>
              ))}
            </div>
          </div>

          {needsSiliconflow && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12, padding: '12px 14px', background: '#fff', borderRadius: 8, marginBottom: 12, border: '1px solid #e2e8f0' }}>
              <label style={{ ...labelStyle, fontWeight: 600, margin: 0 }}>SiliconFlow</label>
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
                <label style={labelStyle} htmlFor="siliconflow-base-model">Base 模型</label>
                <select
                  id="siliconflow-base-model"
                  aria-label="Base model"
                  value={settings.imageBaseModel}
                  onChange={(e) => update({ imageBaseModel: e.target.value, imageModel: e.target.value })}
                  style={selectStyle}
                >
                  {SILICONFLOW_BASE_MODELS.map(({ value, label }) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>
              </div>
              <div>
                <label style={labelStyle} htmlFor="siliconflow-reference-model">Reference / img2img 模型</label>
                <select
                  id="siliconflow-reference-model"
                  aria-label="Reference model"
                  value={settings.imageReferenceModel}
                  onChange={(e) => update({ imageReferenceModel: e.target.value })}
                  style={selectStyle}
                >
                  {SILICONFLOW_REFERENCE_MODELS.map(({ value, label }) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {needsWanxiang && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12, padding: '12px 14px', background: '#fff', borderRadius: 8, marginBottom: 12, border: '1px solid #e2e8f0' }}>
              <label style={{ ...labelStyle, fontWeight: 600, margin: 0 }}>阿里云万相 / DashScope</label>
              <div>
                <label style={labelStyle}>API Key</label>
                <input
                  type="password"
                  value={settings.wanxiangApiKey}
                  onChange={(e) => update({ wanxiangApiKey: e.target.value })}
                  placeholder="DashScope API Key（sk-…）"
                  style={fieldStyle}
                />
              </div>
              <div>
                <label style={labelStyle} htmlFor="wanxiang-base-model">Base 模型（文生图）</label>
                <select
                  id="wanxiang-base-model"
                  aria-label="Wanxiang base model"
                  value={settings.wanxiangBaseModel}
                  onChange={(e) => update({ wanxiangBaseModel: e.target.value })}
                  style={selectStyle}
                >
                  {WANXIANG_BASE_MODELS.map(({ value, label }) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>
              </div>
              <div>
                <label style={labelStyle} htmlFor="wanxiang-edit-model">图像编辑模型</label>
                <select
                  id="wanxiang-edit-model"
                  aria-label="Wanxiang edit model"
                  value={settings.wanxiangEditModel}
                  onChange={(e) => update({ wanxiangEditModel: e.target.value })}
                  style={selectStyle}
                >
                  {WANXIANG_EDIT_MODELS.map(({ value, label }) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {needsSdUrl && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12, padding: '12px 14px', background: '#fff', borderRadius: 8, border: '1px solid #e2e8f0' }}>
              <label style={{ ...labelStyle, fontWeight: 600, margin: 0 }}>Local Stable Diffusion</label>
              <div>
                <label style={labelStyle} htmlFor="local-sd-url">WebUI 地址</label>
                <input
                  id="local-sd-url"
                  type="text"
                  value={settings.localSdUrl}
                  onChange={(e) => update({ localSdUrl: e.target.value })}
                  placeholder="http://localhost:7860"
                  style={fieldStyle}
                />
              </div>
              <div>
                <label style={labelStyle} htmlFor="local-sd-denoising">
                  img2img 去噪强度：{settings.localSdDenoisingStrength.toFixed(2)}
                </label>
                <input
                  id="local-sd-denoising"
                  name="localSdDenoisingStrength"
                  type="range"
                  min={LOCAL_SD_DENOISING_MIN}
                  max={LOCAL_SD_DENOISING_MAX}
                  step="0.05"
                  value={settings.localSdDenoisingStrength}
                  onChange={(e) => update({ localSdDenoisingStrength: normalizeDenoisingStrength(e.target.value) })}
                  style={{ width: '100%', accentColor: '#4f8ef7', cursor: 'pointer' }}
                />
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: '#a0aec0' }}>
                  <span>{LOCAL_SD_DENOISING_MIN.toFixed(2)}</span>
                  <span>{LOCAL_SD_DENOISING_MAX.toFixed(2)}</span>
                </div>
              </div>
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
