import { useState } from 'react';
import { analyzePhoto } from '../../../lib/claude-vision';

interface AnalyzeStepProps {
  photoDataUrl: string;
  initialPrompt: string;
  onNext: (prompt: string) => void;
  onBack: () => void;
}

export default function AnalyzeStep({ photoDataUrl, initialPrompt, onNext, onBack }: AnalyzeStepProps) {
  const [apiKey, setApiKey] = useState('');
  const [prompt, setPrompt] = useState(initialPrompt);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleAnalyze() {
    setAnalyzing(true);
    setError(null);
    try {
      const description = await analyzePhoto(photoDataUrl, apiKey);
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
          style={{ width: 140, height: 140, objectFit: 'cover', borderRadius: 8, flexShrink: 0 }}
        />

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 12 }}>
          <div>
            <label style={{ fontSize: 13, color: '#4a5568', display: 'block', marginBottom: 4 }}>
              Anthropic API Key <span style={{ color: '#a0aec0' }}>(optional)</span>
            </label>
            <div style={{ display: 'flex', gap: 8 }}>
              <input
                type="password"
                placeholder="Anthropic API key"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                style={{
                  flex: 1, padding: '6px 10px', borderRadius: 6,
                  border: '1px solid #e2e8f0', fontSize: 13,
                }}
              />
              <button
                onClick={handleAnalyze}
                disabled={!apiKey || analyzing}
                style={{
                  padding: '6px 16px', borderRadius: 6, border: 'none',
                  background: apiKey && !analyzing ? '#4f8ef7' : '#e2e8f0',
                  color: '#fff', cursor: apiKey && !analyzing ? 'pointer' : 'not-allowed',
                  fontSize: 13, whiteSpace: 'nowrap',
                }}
              >
                {analyzing ? 'Analyzing…' : 'Analyze with AI'}
              </button>
            </div>
          </div>

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
          No AI key? Type the character description manually above.
        </p>
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
        <button
          onClick={onBack}
          style={{
            padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0',
            background: '#fff', color: '#4a5568', cursor: 'pointer',
          }}
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
