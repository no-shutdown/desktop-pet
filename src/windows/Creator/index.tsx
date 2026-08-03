import { useState } from 'react';

type Step = 'upload' | 'analyze' | 'generate' | 'preview' | 'save';
const STEPS: Step[] = ['upload', 'analyze', 'generate', 'preview', 'save'];
const LABELS: Record<Step, string> = {
  upload: 'Upload Photo',
  analyze: 'Analyze',
  generate: 'Generate',
  preview: 'Preview',
  save: 'Save',
};
const ICONS: Record<Step, string> = {
  upload: '📷',
  analyze: '🔍',
  generate: '✨',
  preview: '👀',
  save: '💾',
};

export default function CreatorWindow() {
  const [step, setStep] = useState<Step>('upload');
  const stepIndex = STEPS.indexOf(step);

  return (
    <div style={{ padding: 32, fontFamily: 'system-ui, sans-serif', maxWidth: 760, margin: '0 auto' }}>
      <h1 style={{ marginBottom: 8, fontSize: 24 }}>Create Your Pet</h1>
      <p style={{ color: '#718096', marginBottom: 32 }}>
        Turn a photo into an animated desktop companion.
      </p>

      {/* Step indicators */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 0, marginBottom: 40 }}>
        {STEPS.map((s, i) => (
          <div key={s} style={{ display: 'flex', alignItems: 'center' }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
              <div style={{
                width: 32, height: 32, borderRadius: '50%',
                background: i <= stepIndex ? '#4f8ef7' : '#e2e8f0',
                color: i <= stepIndex ? '#fff' : '#a0aec0',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontWeight: 600, fontSize: 14,
              }}>
                {i + 1}
              </div>
              <span style={{ fontSize: 12, color: i === stepIndex ? '#4f8ef7' : '#a0aec0' }}>
                {LABELS[s]}
              </span>
            </div>
            {i < STEPS.length - 1 && (
              <div style={{ width: 48, height: 2, background: i < stepIndex ? '#4f8ef7' : '#e2e8f0', marginBottom: 20 }} />
            )}
          </div>
        ))}
      </div>

      {/* Step content */}
      <div style={{
        border: '2px dashed #e2e8f0', borderRadius: 12, padding: 64,
        textAlign: 'center', color: '#a0aec0', minHeight: 240,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}>
        <div>
          <div style={{ fontSize: 48, marginBottom: 16 }}>
            {ICONS[step]}
          </div>
          <p style={{ margin: 0, fontSize: 16 }}>{LABELS[step]} — coming in Plan 2</p>
        </div>
      </div>

      {/* Navigation */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12, marginTop: 24 }}>
        <button
          onClick={() => setStep(STEPS[stepIndex - 1])}
          disabled={stepIndex === 0}
          style={{
            padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0',
            cursor: stepIndex === 0 ? 'not-allowed' : 'pointer', background: '#fff', color: '#4a5568',
          }}
        >
          Back
        </button>
        <button
          onClick={() => setStep(STEPS[stepIndex + 1])}
          disabled={stepIndex === STEPS.length - 1}
          style={{
            padding: '8px 20px', borderRadius: 6, border: 'none',
            background: stepIndex === STEPS.length - 1 ? '#e2e8f0' : '#4f8ef7',
            color: '#fff', cursor: stepIndex === STEPS.length - 1 ? 'not-allowed' : 'pointer',
          }}
        >
          Next
        </button>
      </div>
    </div>
  );
}
