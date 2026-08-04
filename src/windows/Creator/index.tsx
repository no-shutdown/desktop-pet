import { useState } from 'react';
import type { Pet } from '../../types/pet';
import { INITIAL_WIZARD_DATA, type WizardData } from './steps/types';
import UploadStep from './steps/UploadStep';
import AnalyzeStep from './steps/AnalyzeStep';
import GenerateStep from './steps/GenerateStep';
import DirectUploadStep from './steps/DirectUploadStep';
import PreviewStep from './steps/PreviewStep';
import SaveStep from './steps/SaveStep';
import SettingsPanel from './SettingsPanel';

type Mode = 'choose' | 'ai' | 'manual';
type Step = 'upload' | 'analyze' | 'generate' | 'direct-upload' | 'preview' | 'save';

const AI_STEPS: Step[] = ['upload', 'analyze', 'generate', 'preview', 'save'];
const MANUAL_STEPS: Step[] = ['direct-upload', 'preview', 'save'];
const STEP_LABELS: Record<Step, string> = {
  upload: 'Upload', analyze: 'Analyze', generate: 'Generate',
  'direct-upload': 'Upload GIFs', preview: 'Preview', save: 'Save',
};

export default function CreatorWindow() {
  const [mode, setMode] = useState<Mode>('choose');
  const [step, setStep] = useState<Step>('upload');
  const [data, setData] = useState<WizardData>(INITIAL_WIZARD_DATA);
  const [savedPet, setSavedPet] = useState<Pet | null>(null);
  const [showSettings, setShowSettings] = useState(false);

  function updateData(patch: Partial<WizardData>) {
    setData((prev) => ({ ...prev, ...patch }));
  }

  function reset() {
    setData(INITIAL_WIZARD_DATA);
    setSavedPet(null);
    setMode('choose');
  }

  const currentSteps = mode === 'ai' ? AI_STEPS : MANUAL_STEPS;
  const stepIndex = currentSteps.indexOf(step);

  if (mode === 'choose') {
    return (
      <div style={{ padding: 32, fontFamily: 'system-ui, sans-serif', maxWidth: 760, margin: '0 auto' }}>
        {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
          <h1 style={{ margin: 0, fontSize: 24 }}>Create Your Pet</h1>
          <button
            onClick={() => setShowSettings(true)}
            title="Settings"
            style={{ background: 'none', border: 'none', fontSize: 20, cursor: 'pointer', color: '#a0aec0', padding: 4 }}
          >
            ⚙
          </button>
        </div>
        <p style={{ color: '#718096', marginBottom: 48 }}>
          Turn a photo into an animated desktop companion.
        </p>

        <div style={{ display: 'flex', gap: 24 }}>
          <button
            onClick={() => { setMode('ai'); setStep('upload'); }}
            style={{
              flex: 1, padding: '32px 24px', borderRadius: 12, border: '2px solid #e2e8f0',
              background: '#fff', cursor: 'pointer', textAlign: 'left',
              transition: 'border-color 0.2s, box-shadow 0.2s',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLButtonElement).style.borderColor = '#4f8ef7';
              (e.currentTarget as HTMLButtonElement).style.boxShadow = '0 4px 12px rgba(79,142,247,0.15)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLButtonElement).style.borderColor = '#e2e8f0';
              (e.currentTarget as HTMLButtonElement).style.boxShadow = 'none';
            }}
          >
            <div style={{ fontSize: 40, marginBottom: 12 }}>🤖</div>
            <div style={{ fontWeight: 700, fontSize: 16, marginBottom: 6, color: '#1a202c' }}>
              Generate with AI
            </div>
            <div style={{ fontSize: 13, color: '#718096', lineHeight: 1.5 }}>
              Upload a reference photo and let AI generate all animation frames automatically.
            </div>
          </button>

          <button
            onClick={() => { setMode('manual'); setStep('direct-upload'); }}
            style={{
              flex: 1, padding: '32px 24px', borderRadius: 12, border: '2px solid #e2e8f0',
              background: '#fff', cursor: 'pointer', textAlign: 'left',
              transition: 'border-color 0.2s, box-shadow 0.2s',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLButtonElement).style.borderColor = '#4f8ef7';
              (e.currentTarget as HTMLButtonElement).style.boxShadow = '0 4px 12px rgba(79,142,247,0.15)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLButtonElement).style.borderColor = '#e2e8f0';
              (e.currentTarget as HTMLButtonElement).style.boxShadow = 'none';
            }}
          >
            <div style={{ fontSize: 40, marginBottom: 12 }}>🎨</div>
            <div style={{ fontWeight: 700, fontSize: 16, marginBottom: 6, color: '#1a202c' }}>
              Upload My Own GIFs
            </div>
            <div style={{ fontSize: 13, color: '#718096', lineHeight: 1.5 }}>
              Already have animations? Upload one image or GIF per state and use them directly.
            </div>
          </button>
        </div>
      </div>
    );
  }

  return (
    <div style={{ padding: 32, fontFamily: 'system-ui, sans-serif', maxWidth: 760, margin: '0 auto' }}>
      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
        <h1 style={{ margin: 0, fontSize: 24 }}>Create Your Pet</h1>
        <button
          onClick={() => setShowSettings(true)}
          title="Settings"
          style={{ background: 'none', border: 'none', fontSize: 20, cursor: 'pointer', color: '#a0aec0', padding: 4 }}
        >
          ⚙
        </button>
      </div>
      <p style={{ color: '#718096', marginBottom: 32 }}>
        Turn a photo into an animated desktop companion.
      </p>

      {/* Step indicators */}
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 40 }}>
        {currentSteps.map((s, i) => (
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
                {STEP_LABELS[s]}
              </span>
            </div>
            {i < currentSteps.length - 1 && (
              <div style={{ width: 48, height: 2, background: i < stepIndex ? '#4f8ef7' : '#e2e8f0', marginBottom: 20 }} />
            )}
          </div>
        ))}
      </div>

      {/* Step content */}
      {savedPet ? (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <div style={{ fontSize: 56, marginBottom: 16 }}>🎉</div>
          <h2 style={{ marginBottom: 8 }}>{savedPet.name} is ready!</h2>
          <p style={{ color: '#718096' }}>Your pet is saved and available in the system tray.</p>
          <button
            onClick={reset}
            style={{ marginTop: 16, padding: '10px 24px', borderRadius: 8, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer', fontSize: 15 }}
          >
            Create Another
          </button>
        </div>
      ) : (
        <>
          {step === 'upload' && (
            <UploadStep
              onNext={(photoDataUrl) => { updateData({ photoDataUrl }); setStep('analyze'); }}
            />
          )}
          {step === 'analyze' && data.photoDataUrl && (
            <AnalyzeStep
              photoDataUrl={data.photoDataUrl}
              initialPrompt={data.prompt}
              onNext={(prompt) => { updateData({ prompt }); setStep('generate'); }}
              onBack={() => setStep('upload')}
            />
          )}
          {step === 'generate' && (
            <GenerateStep
              prompt={data.prompt}
              onNext={(petId) => { updateData({ petId }); setStep('preview'); }}
              onBack={() => setStep('analyze')}
            />
          )}
          {step === 'direct-upload' && (
            <DirectUploadStep
              onNext={(petId) => { updateData({ petId }); setStep('preview'); }}
              onBack={() => setMode('choose')}
            />
          )}
          {step === 'preview' && data.petId && (
            <PreviewStep
              petId={data.petId}
              onNext={() => setStep('save')}
              onBack={() => mode === 'ai' ? setStep('generate') : setStep('direct-upload')}
            />
          )}
          {step === 'save' && data.petId && (
            <SaveStep
              petId={data.petId}
              prompt={data.prompt}
              onComplete={(pet) => setSavedPet(pet)}
              onBack={() => setStep('preview')}
            />
          )}
        </>
      )}
    </div>
  );
}
