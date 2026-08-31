import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Pet } from '../../types/pet';
import { INITIAL_WIZARD_DATA, type GeneratedSpriteConfig, type WizardData } from './steps/types';
import AnalyzeStep from './steps/AnalyzeStep';
import GenerateStep from './steps/GenerateStep';
import ManualFramePickerStep from './steps/ManualFramePickerStep';
import PreviewStep from './steps/PreviewStep';
import SaveStep from './steps/SaveStep';
import StateGenerationStep from './steps/StateGenerationStep';
import UploadStep from './steps/UploadStep';
import SettingsPanel from './SettingsPanel';

type Mode = 'choose' | 'ai' | 'sprite';
type Step = 'upload' | 'analyze' | 'base-generate' | 'state-generate' | 'sprite-import' | 'preview' | 'save';

const AI_STEPS: Step[] = ['upload', 'analyze', 'base-generate', 'state-generate', 'sprite-import', 'preview', 'save'];
const SPRITE_STEPS: Step[] = ['sprite-import', 'preview', 'save'];
const STEP_LABELS: Record<Step, string> = {
  upload: '上传',
  analyze: '分析',
  'base-generate': '基础',
  'state-generate': '状态',
  'sprite-import': '配置',
  preview: '预览',
  save: '保存',
};

function makeGenerationRunId(): string {
  if (typeof globalThis.crypto?.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  return `generation-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export default function CreatorWindow() {
  const [mode, setMode] = useState<Mode>('choose');
  const [step, setStep] = useState<Step>('upload');
  const [data, setData] = useState<WizardData>(INITIAL_WIZARD_DATA);
  const [savedPet, setSavedPet] = useState<Pet | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [generationBusy, setGenerationBusy] = useState(false);

  function updateData(patch: Partial<WizardData>) {
    setData((previous) => ({ ...previous, ...patch }));
  }

  function discardGenerationRun(runId: string | null) {
    if (!runId) return;
    void invoke('discard_generation_run', { runId }).catch((error: unknown) => {
      if (import.meta.env.DEV) {
        console.warn('discard generation run failed', error);
      }
    });
  }

  function abandonGeneration() {
    discardGenerationRun(data.generationRunId);
    updateData({
      generationRunId: null,
      baseDataUrl: null,
      generatedDataUrl: null,
      generatedConfig: null,
    });
  }

  function reset() {
    discardGenerationRun(data.generationRunId);
    setData(INITIAL_WIZARD_DATA);
    setSavedPet(null);
    setMode('choose');
  }

  const currentSteps = mode === 'ai' ? AI_STEPS : SPRITE_STEPS;
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
            disabled={generationBusy}
            title="设置"
            style={{ background: 'none', border: '1px solid #e2e8f0', borderRadius: 8, padding: '6px 10px', cursor: generationBusy ? 'not-allowed' : 'pointer', color: '#718096', fontSize: 18, lineHeight: 1, opacity: generationBusy ? 0.5 : 1 }}
          >
            ⚙️
          </button>
        </div>
        <p style={{ color: '#718096', marginBottom: 48 }}>将参考图片变成活生生的桌面伴侣。</p>

        <div style={{ display: 'flex', gap: 24 }}>
          <button
            onClick={() => { setData(INITIAL_WIZARD_DATA); setMode('ai'); setStep('upload'); }}
            style={{ flex: 1, padding: '32px 24px', borderRadius: 12, border: '2px solid #e2e8f0', background: '#fff', cursor: 'pointer', textAlign: 'left' }}
          >
            <div style={{ fontSize: 40, marginBottom: 12 }}>🐾</div>
            <div style={{ fontWeight: 700, fontSize: 16, marginBottom: 6, color: '#1a202c' }}>AI 生成</div>
            <div style={{ fontSize: 13, color: '#718096', lineHeight: 1.5 }}>上传参考图，分析角色，生成基础图像，再生成各动画状态。</div>
          </button>
          <button
            onClick={() => { setData(INITIAL_WIZARD_DATA); setMode('sprite'); setStep('sprite-import'); }}
            style={{ flex: 1, padding: '32px 24px', borderRadius: 12, border: '2px solid #e2e8f0', background: '#fff', cursor: 'pointer', textAlign: 'left' }}
          >
            <div style={{ fontSize: 40, marginBottom: 12 }}>🖼️</div>
            <div style={{ fontWeight: 700, fontSize: 16, marginBottom: 6, color: '#1a202c' }}>导入精灵图</div>
            <div style={{ fontSize: 13, color: '#718096', lineHeight: 1.5 }}>导入现有的横排或网格精灵图，并配置帧选择。</div>
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
          disabled={generationBusy}
          title="设置"
          style={{ background: 'none', border: '1px solid #e2e8f0', borderRadius: 8, padding: '6px 10px', cursor: generationBusy ? 'not-allowed' : 'pointer', color: '#718096', fontSize: 18, lineHeight: 1, opacity: generationBusy ? 0.5 : 1 }}
        >
          ⚙️
        </button>
      </div>
      <p style={{ color: '#718096', marginBottom: 32 }}>将参考图片变成活生生的桌面伴侣。</p>

      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 40, overflowX: 'auto' }}>
        {currentSteps.map((currentStep, index) => (
          <div key={currentStep} style={{ display: 'flex', alignItems: 'center', flexShrink: 0 }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
              <div style={{
                width: 32, height: 32, borderRadius: '50%',
                background: index <= stepIndex ? '#4f8ef7' : '#e2e8f0',
                color: index <= stepIndex ? '#fff' : '#a0aec0',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontWeight: 600, fontSize: 14,
              }}>
                {index + 1}
              </div>
              <span style={{ fontSize: 12, color: index === stepIndex ? '#4f8ef7' : '#a0aec0' }}>
                {STEP_LABELS[currentStep]}
              </span>
            </div>
            {index < currentSteps.length - 1 && (
              <div style={{ width: 32, height: 2, background: index < stepIndex ? '#4f8ef7' : '#e2e8f0', margin: '0 8px 20px' }} />
            )}
          </div>
        ))}
      </div>

      {savedPet ? (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <div style={{ fontSize: 56, marginBottom: 16 }}>🎉</div>
          <h2 style={{ marginBottom: 8 }}>{savedPet.name} 已就绪！</h2>
          <p style={{ color: '#718096' }}>宠物已保存，可从系统托盘访问。</p>
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
              onBack={() => { abandonGeneration(); setMode('choose'); }}
            />
          )}

          {step === 'analyze' && data.photoDataUrl && (
            <AnalyzeStep
              photoDataUrl={data.photoDataUrl}
              initialPrompt={data.prompt}
              initialSourceStyle={data.sourceStyle}
              onNext={(prompt, sourceStyle) => {
                updateData({
                  prompt,
                  sourceStyle,
                  generationRunId: data.generationRunId ?? makeGenerationRunId(),
                  baseDataUrl: null,
                  generatedDataUrl: null,
                  generatedConfig: null,
                });
                setStep('base-generate');
              }}
              onBack={(sourceStyle) => {
                updateData({ sourceStyle });
                setStep('upload');
              }}
            />
          )}

          {step === 'base-generate' && (
            <GenerateStep
              prompt={data.prompt}
              referenceDataUrl={data.photoDataUrl}
              sourceStyle={data.sourceStyle}
              runId={data.generationRunId ?? undefined}
              onBusyChange={setGenerationBusy}
              onNext={({ runId, dataUrl }) => {
                updateData({ generationRunId: runId, baseDataUrl: dataUrl });
                setStep('state-generate');
              }}
              onBack={() => {
                abandonGeneration();
                setStep('analyze');
              }}
            />
          )}

          {step === 'state-generate' && data.generationRunId && (
            <StateGenerationStep
              runId={data.generationRunId}
              baseDataUrl={data.baseDataUrl}
              onBusyChange={setGenerationBusy}
              onNext={(dataUrl: string, config: GeneratedSpriteConfig) => {
                updateData({ generatedDataUrl: dataUrl, generatedConfig: config });
                setStep('sprite-import');
              }}
              onBack={() => setStep('base-generate')}
            />
          )}

          {step === 'sprite-import' && (
            <ManualFramePickerStep
              onNext={(petId, states, runId) => {
                updateData({ petId, petStates: states, generationRunId: runId });
                setStep('preview');
              }}
              onBack={() => {
                if (mode === 'ai') {
                  setStep('state-generate');
                } else {
                  abandonGeneration();
                  setMode('choose');
                }
              }}
              initialDataUrl={mode === 'ai' ? data.generatedDataUrl : null}
              initialPetId={mode === 'ai' ? data.generatedConfig?.petId ?? null : null}
              initialConfig={mode === 'ai' && data.generatedConfig ? {
                runId: data.generatedConfig.runId,
                frameW: data.generatedConfig.frameW,
                frameH: data.generatedConfig.frameH,
                layout: data.generatedConfig.layout,
                rowGap: data.generatedConfig.rowGap,
                idleFrames: data.generatedConfig.idleFrames,
                sleepingFrames: data.generatedConfig.sleepingFrames,
                actingCuteFrames: data.generatedConfig.actingCuteFrames,
                workingFrames: data.generatedConfig.workingFrames,
              } : undefined}
            />
          )}

          {step === 'preview' && data.petId && (
            <PreviewStep
              petId={data.petId}
              runId={data.generationRunId ?? undefined}
              onNext={() => setStep('save')}
              onBack={() => setStep('sprite-import')}
            />
          )}

          {step === 'save' && data.petId && data.petStates && data.generationRunId && (
            <SaveStep
              petId={data.petId}
              runId={data.generationRunId}
              prompt={data.prompt}
              states={data.petStates}
              onComplete={(pet) => {
                discardGenerationRun(data.generationRunId);
                updateData({ generationRunId: null, baseDataUrl: null });
                setSavedPet(pet);
              }}
              onBack={() => setStep('preview')}
            />
          )}
        </>
      )}
    </div>
  );
}
