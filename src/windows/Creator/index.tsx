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
  upload: '上传照片', analyze: '分析角色', generate: '生成动画',
  'direct-upload': '上传动画', preview: '预览', save: '保存',
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

  if (showSettings) {
    return (
      <div style={{ padding: 32, fontFamily: 'system-ui, sans-serif', maxWidth: 760, margin: '0 auto', height: '100vh', boxSizing: 'border-box', display: 'flex', flexDirection: 'column' }}>
        <SettingsPanel onBack={() => setShowSettings(false)} />
      </div>
    );
  }

  if (mode === 'choose') {
    return (
      <div style={{ padding: 32, fontFamily: 'system-ui, sans-serif', maxWidth: 760, margin: '0 auto' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
          <h1 style={{ margin: 0, fontSize: 24 }}>创建你的桌面宠物</h1>
          <button
            onClick={() => setShowSettings(true)}
            title="配置"
            style={{ background: 'none', border: '1px solid #e2e8f0', borderRadius: 8, padding: '6px 10px', cursor: 'pointer', color: '#718096', fontSize: 18, lineHeight: 1 }}
          >
            ⚙️
          </button>
        </div>
        <p style={{ color: '#718096', marginBottom: 48 }}>
          把一张照片变成在桌面上活动的动态伴侣。
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
              AI 自动生成
            </div>
            <div style={{ fontSize: 13, color: '#718096', lineHeight: 1.5 }}>
              上传一张参考照片，AI 自动分析角色并生成全部动画帧。
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
              上传我的动画
            </div>
            <div style={{ fontSize: 13, color: '#718096', lineHeight: 1.5 }}>
              已有动画素材？每个状态上传一张图片或 GIF 即可直接使用。
            </div>
          </button>
        </div>
      </div>
    );
  }

  return (
    <div style={{ padding: 32, fontFamily: 'system-ui, sans-serif', maxWidth: 760, margin: '0 auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
        <h1 style={{ margin: 0, fontSize: 24 }}>创建你的桌面宠物</h1>
        <button
          onClick={() => setShowSettings(true)}
          title="配置"
          style={{ background: 'none', border: '1px solid #e2e8f0', borderRadius: 8, padding: '6px 10px', cursor: 'pointer', color: '#718096', fontSize: 18, lineHeight: 1 }}
        >
          ⚙️
        </button>
      </div>
      <p style={{ color: '#718096', marginBottom: 32 }}>
        把一张照片变成在桌面上活动的动态伴侣。
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
          <h2 style={{ marginBottom: 8 }}>{savedPet.name} 已就绪！</h2>
          <p style={{ color: '#718096' }}>宠物已保存，可在系统托盘中查看。</p>
          <button
            onClick={reset}
            style={{ marginTop: 16, padding: '10px 24px', borderRadius: 8, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer', fontSize: 15 }}
          >
            再创建一个
          </button>
        </div>
      ) : (
        <>
          {step === 'upload' && (
            <UploadStep
              onNext={(photoDataUrl) => { updateData({ photoDataUrl }); setStep('analyze'); }}
              onBack={() => { setMode('choose'); }}
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
              onNext={(petId, states) => { updateData({ petId, petStates: states }); setStep('preview'); }}
              onBack={() => setStep('analyze')}
            />
          )}
          {step === 'direct-upload' && (
            <DirectUploadStep
              onNext={(petId, states) => { updateData({ petId, petStates: states }); setStep('preview'); }}
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
          {step === 'save' && data.petId && data.petStates && (
            <SaveStep
              petId={data.petId}
              prompt={data.prompt}
              states={data.petStates}
              onComplete={(pet) => setSavedPet(pet)}
              onBack={() => setStep('preview')}
            />
          )}
        </>
      )}
    </div>
  );
}
