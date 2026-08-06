import { useEffect, useState } from 'react';
import { appDataDir, join } from '@tauri-apps/api/path';
import { convertFileSrc } from '@tauri-apps/api/core';
import { PET_STATE_CATALOG, PET_STATES } from '../../../types/pet';

interface PreviewStepProps {
  petId: string;
  onNext: () => void;
  onBack: () => void;
}

export default function PreviewStep({ petId, onNext, onBack }: PreviewStepProps) {
  const [sheetSrcs, setSheetSrcs] = useState<Record<string, string>>({});

  useEffect(() => {
    async function loadPaths() {
      const appDir = await appDataDir();
      const srcs: Record<string, string> = {};
      for (const state of PET_STATES) {
        const absPath = await join(appDir, 'pets', petId, `${state}.png`);
        srcs[state] = convertFileSrc(absPath);
      }
      setSheetSrcs(srcs);
    }
    loadPaths();
  }, [petId]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <p style={{ color: '#718096', margin: 0 }}>
        以下是生成的各动画状态，效果满意吗？
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 16 }}>
        {PET_STATES.map((state) => {
          const definition = PET_STATE_CATALOG.find((item) => item.key === state);
          return (
          <div key={state} style={{ textAlign: 'center' }}>
            <div style={{
              background: '#f7fafc', borderRadius: 8, padding: 8,
              border: '1px solid #e2e8f0', minHeight: 100,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}>
              {sheetSrcs[state] ? (
                <img
                  src={sheetSrcs[state]}
                  alt={state}
                  style={{ maxWidth: '100%', maxHeight: 120, imageRendering: 'pixelated' }}
                />
              ) : (
                <span style={{ color: '#a0aec0', fontSize: 13 }}>加载中…</span>
              )}
            </div>
            <p style={{ margin: '6px 0 0', fontSize: 12, color: '#4a5568' }}>
              {definition?.label ?? state}
            </p>
          </div>
          );
        })}
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
        <button onClick={onBack} style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}>
          上一步
        </button>
        <button onClick={onNext} style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer' }}>
          下一步
        </button>
      </div>
    </div>
  );
}
