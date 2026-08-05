import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PetState, SpriteStateInfo } from '../../../types/pet';

const STATES: PetState[] = ['idle', 'walking', 'waving', 'working'];
const STATE_LABELS: Record<PetState, string> = {
  idle: '待机', walking: '行走', waving: '招手', working: '工作',
};
const STATE_HINTS: Record<PetState, string> = {
  idle: '静止站立', walking: '移动 / 行走',
  waving: '打招呼 / 挥手', working: '专注 / 打字',
};

interface DirectUploadStepProps {
  onNext: (petId: string, states: Record<PetState, SpriteStateInfo>) => void;
  onBack: () => void;
}

export default function DirectUploadStep({ onNext, onBack }: DirectUploadStepProps) {
  const [files, setFiles] = useState<Partial<Record<PetState, string>>>({});
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const allFilled = STATES.every((s) => files[s]);

  function handleFile(state: PetState, file: File) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const dataUrl = (e.target as FileReader).result as string;
      setFiles((prev) => ({ ...prev, [state]: dataUrl }));
    };
    reader.readAsDataURL(file);
  }

  async function handleNext() {
    setSaving(true);
    setError(null);
    try {
      const petId = crypto.randomUUID();
      const states = await invoke<Record<PetState, SpriteStateInfo>>('save_custom_frames', {
        petId,
        idle: files.idle!,
        walking: files.walking!,
        waving: files.waving!,
        working: files.working!,
      });
      onNext(petId, states);
    } catch (err) {
      setError((err as Error).message ?? String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <p style={{ color: '#718096', margin: 0, fontSize: 14 }}>
        每个动画状态上传一张图片或 GIF。支持 .gif .png .jpg .webp
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
        {STATES.map((state) => (
          <label
            key={state}
            style={{
              display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
              border: `2px dashed ${files[state] ? '#4f8ef7' : '#e2e8f0'}`,
              borderRadius: 10, padding: 16, cursor: 'pointer',
              background: files[state] ? '#f0f5ff' : '#fafafa',
              transition: 'border-color 0.2s, background 0.2s',
            }}
          >
            <input
              type="file"
              accept=".gif,.png,.jpg,.jpeg,.webp"
              style={{ display: 'none' }}
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) handleFile(state, file);
              }}
            />
            {files[state] ? (
              <img
                src={files[state]}
                alt={state}
                style={{ width: 80, height: 80, objectFit: 'contain', imageRendering: 'pixelated' }}
              />
            ) : (
              <div style={{
                width: 80, height: 80, display: 'flex', alignItems: 'center',
                justifyContent: 'center', fontSize: 32, color: '#cbd5e0',
              }}>
                +
              </div>
            )}
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontWeight: 600, fontSize: 13, color: '#2d3748' }}>{STATE_LABELS[state]}</div>
              <div style={{ fontSize: 11, color: '#a0aec0' }}>{STATE_HINTS[state]}</div>
            </div>
          </label>
        ))}
      </div>

      {error && <p style={{ color: '#e53e3e', fontSize: 13, margin: 0 }}>{error}</p>}

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
        <button
          onClick={onBack}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
        >
          返回首页
        </button>
        <button
          onClick={handleNext}
          disabled={!allFilled || saving}
          style={{
            padding: '8px 24px', borderRadius: 6, border: 'none',
            background: allFilled && !saving ? '#4f8ef7' : '#e2e8f0',
            color: '#fff', cursor: allFilled && !saving ? 'pointer' : 'not-allowed',
          }}
        >
          {saving ? '保存中…' : '下一步'}
        </button>
      </div>
    </div>
  );
}
