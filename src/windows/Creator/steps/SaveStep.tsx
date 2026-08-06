import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Pet, PetState, SpriteStateInfo } from '../../../types/pet';

interface SaveStepProps {
  petId: string;
  runId: string;
  prompt: string;
  states: Record<PetState, SpriteStateInfo>;
  onComplete: (pet: Pet) => void;
  onBack: () => void;
}

export default function SaveStep({ petId, runId, prompt, states, onComplete, onBack }: SaveStepProps) {
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSave() {
    if (!name.trim()) return;
    setSaving(true);
    setError(null);
    try {
      const pet: Pet = {
        id: petId,
        name: name.trim(),
        states,
        createdAt: new Date().toISOString(),
        prompt,
      };
      await invoke('save_pet_from_run', { runId, pet });
      onComplete(pet);
    } catch (err) {
      setError((err as Error).message ?? String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24, alignItems: 'center', padding: '32px 0' }}>
      <div style={{ fontSize: 48 }}>💾</div>
      <p style={{ color: '#718096', margin: 0 }}>给你的宠物起个名字吧。</p>

      <input
        type="text"
        placeholder="宠物名称…"
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && handleSave()}
        maxLength={32}
        style={{
          padding: '10px 16px', borderRadius: 8, border: '1px solid #e2e8f0',
          fontSize: 16, width: 240, textAlign: 'center',
        }}
      />

      {error && <p style={{ color: '#e53e3e', fontSize: 13, margin: 0 }}>{error}</p>}

      <div style={{ display: 'flex', gap: 12 }}>
        <button
          onClick={onBack}
          disabled={saving}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: saving ? 'not-allowed' : 'pointer' }}
        >
          上一步
        </button>
        <button
          onClick={handleSave}
          disabled={!name.trim() || saving}
          style={{
            padding: '8px 24px', borderRadius: 6, border: 'none',
            background: name.trim() && !saving ? '#4f8ef7' : '#e2e8f0',
            color: '#fff', cursor: name.trim() && !saving ? 'pointer' : 'not-allowed',
          }}
        >
          {saving ? '保存中…' : '保存宠物'}
        </button>
      </div>
    </div>
  );
}
