import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { loadSettings } from '../../../lib/settings';

interface GenerateStepProps {
  prompt: string;
  onNext: (petId: string) => void;
  onBack: () => void;
}

type Status = 'idle' | 'generating' | 'done' | 'error';

const TOTAL_FRAMES = 18;

export default function GenerateStep({ prompt, onNext, onBack }: GenerateStepProps) {
  const [status, setStatus] = useState<Status>('idle');
  const [progress, setProgress] = useState({ current: 0, total: TOTAL_FRAMES });
  const [petId, setPetId] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  async function handleGenerate() {
    const id = crypto.randomUUID();
    setPetId(id);
    setStatus('generating');
    setProgress({ current: 0, total: TOTAL_FRAMES });

    const unlisten = await listen<{ current: number; total: number }>(
      'generation-progress',
      (event) => setProgress(event.payload)
    );

    try {
      const settings = loadSettings();
      await invoke('generate_and_assemble', {
        petId: id,
        basePrompt: prompt,
        imageProvider: settings.imageProvider,
        imageApiKey: settings.imageApiKey || null,
        localSdUrl: settings.localSdUrl || null,
      });
      setStatus('done');
    } catch (err) {
      setErrorMsg((err as Error).message ?? String(err));
      setStatus('error');
    } finally {
      unlisten();
    }
  }

  const pct = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24, alignItems: 'center', padding: '32px 0' }}>
      {status === 'idle' && (
        <>
          <div style={{ fontSize: 48 }}>✨</div>
          <p style={{ color: '#718096', textAlign: 'center', margin: 0 }}>
            Ready to generate 18 animation frames using your prompt.
            <br />
            <span style={{ fontSize: 13, color: '#a0aec0' }}>This may take 2–5 minutes.</span>
          </p>
        </>
      )}

      {status === 'generating' && (
        <>
          <div style={{ fontSize: 48 }}>⏳</div>
          <p style={{ color: '#4a5568', margin: 0 }}>
            Generating frame {progress.current} of {progress.total}…
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
          <p style={{ color: '#38a169', margin: 0 }}>All frames generated!</p>
        </>
      )}

      {status === 'error' && (
        <>
          <div style={{ fontSize: 48 }}>⚠️</div>
          <p style={{ color: '#e53e3e', margin: 0 }}>{errorMsg}</p>
        </>
      )}

      <div style={{ display: 'flex', gap: 12, marginTop: 8 }}>
        <button
          onClick={onBack}
          disabled={status === 'generating'}
          style={{
            padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0',
            background: '#fff', color: '#4a5568',
            cursor: status === 'generating' ? 'not-allowed' : 'pointer',
          }}
        >
          Back
        </button>

        {status === 'done' ? (
          <button
            onClick={() => petId && onNext(petId)}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer' }}
          >
            Next
          </button>
        ) : (
          <button
            onClick={handleGenerate}
            disabled={status === 'generating'}
            style={{
              padding: '8px 24px', borderRadius: 6, border: 'none',
              background: status === 'generating' ? '#e2e8f0' : '#4f8ef7',
              color: '#fff', cursor: status === 'generating' ? 'not-allowed' : 'pointer',
            }}
          >
            {status === 'error' ? 'Retry' : 'Generate'}
          </button>
        )}
      </div>
    </div>
  );
}
