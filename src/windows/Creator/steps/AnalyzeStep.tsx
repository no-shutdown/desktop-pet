import { useState } from 'react';
import { loadSettings, VISION_PROVIDER_LABELS } from '../../../lib/settings';
import { analyzePhotoWithSettings } from '../../../lib/vision';

interface AnalyzeStepProps {
  photoDataUrl: string;
  initialPrompt: string;
  onNext: (prompt: string) => void;
  onBack: () => void;
}

export default function AnalyzeStep({ photoDataUrl, initialPrompt, onNext, onBack }: AnalyzeStepProps) {
  const [prompt, setPrompt] = useState(initialPrompt);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const settings = loadSettings();
  const canAnalyze = settings.visionProvider !== 'skip' && settings.visionApiKey.length > 0;

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

  const providerLabel = VISION_PROVIDER_LABELS[settings.visionProvider];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', gap: 24, alignItems: 'flex-start' }}>
        <img
          alt="reference"
          src={photoDataUrl}
          style={{ width: 140, height: 140, objectFit: 'cover', borderRadius: 8, flexShrink: 0 }}
        />

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 12 }}>
          {settings.visionProvider === 'skip' ? (
            <p style={{ margin: 0, fontSize: 13, color: '#718096' }}>
              Vision analysis is disabled. Type your character description below, or change the provider in{' '}
              <strong>Settings</strong>.
            </p>
          ) : (
            <div>
              <p style={{ margin: '0 0 8px', fontSize: 13, color: '#4a5568' }}>
                Provider: <strong>{providerLabel}</strong>
                {!canAnalyze && (
                  <span style={{ color: '#e53e3e', marginLeft: 8 }}>— API key not set (open Settings)</span>
                )}
              </p>
              <button
                onClick={handleAnalyze}
                disabled={!canAnalyze || analyzing}
                style={{
                  padding: '6px 16px', borderRadius: 6, border: 'none',
                  background: canAnalyze && !analyzing ? '#4f8ef7' : '#e2e8f0',
                  color: '#fff', cursor: canAnalyze && !analyzing ? 'pointer' : 'not-allowed',
                  fontSize: 13, whiteSpace: 'nowrap',
                }}
              >
                {analyzing ? 'Analyzing…' : 'Analyze with AI'}
              </button>
            </div>
          )}

          {error && (
            <p style={{ color: '#e53e3e', fontSize: 13, margin: 0 }}>{error}</p>
          )}
        </div>
      </div>

      <div>
        <label
          htmlFor="prompt-textarea"
          style={{ fontSize: 13, color: '#4a5568', display: 'block', marginBottom: 4 }}
        >
          Character Description
        </label>
        <textarea
          id="prompt-textarea"
          aria-label="character description"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={5}
          placeholder="e.g. anime chibi girl, black twin-tail hair, red sailor uniform, blue eyes, cute expression..."
          style={{
            width: '100%', padding: '8px 12px', borderRadius: 6,
            border: '1px solid #e2e8f0', fontSize: 13, resize: 'vertical',
            fontFamily: 'inherit', boxSizing: 'border-box',
          }}
        />
        <p style={{ color: '#a0aec0', fontSize: 12, marginTop: 4 }}>
          {canAnalyze ? 'AI-generated above — edit as needed.' : 'Type the description manually above.'}
        </p>
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
        <button
          onClick={onBack}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
        >
          Back
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
          Next
        </button>
      </div>
    </div>
  );
}
