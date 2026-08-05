import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { Pet } from '../../types/pet';

interface PetPickerProps {
  pets: Pet[];
  activePetId: string | null;
  onSelect: (pet: Pet) => void;
  onClose: () => void;
}

export default function PetPicker({ pets, activePetId, onSelect, onClose }: PetPickerProps) {
  const [srcMap, setSrcMap] = useState<Record<string, string>>({});

  useEffect(() => {
    async function buildSrcMap() {
      const { appDataDir, join } = await import('@tauri-apps/api/path');
      const appDir = await appDataDir();
      const entries = await Promise.all(
        pets.map(async (pet) => {
          const path = await join(appDir, 'pets', pet.id, 'idle.png');
          return [pet.id, convertFileSrc(path)] as const;
        })
      );
      setSrcMap(Object.fromEntries(entries));
    }
    buildSrcMap();
  }, [pets]);

  return (
    <div style={{
      position: 'fixed', inset: 0, zIndex: 2000,
      background: 'rgba(0,0,0,0.55)',
      display: 'flex', flexDirection: 'column',
    }}>
      {/* Header */}
      <div style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '8px 10px', background: '#1a202c',
      }}>
        <span style={{ color: '#fff', fontSize: 12, fontWeight: 600 }}>选择宠物</span>
        <button
          onClick={onClose}
          style={{ background: 'none', border: 'none', color: '#a0aec0', fontSize: 16, cursor: 'pointer', lineHeight: 1, padding: 0 }}
        >
          ×
        </button>
      </div>

      {/* Pet list */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '6px 0' }}>
        {pets.length === 0 ? (
          <p style={{ color: '#a0aec0', fontSize: 12, textAlign: 'center', margin: '16px 0' }}>
            暂无宠物
          </p>
        ) : (
          pets.map((pet) => {
            const isActive = pet.id === activePetId;
            return (
              <div
                key={pet.id}
                onClick={() => onSelect(pet)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '6px 10px', cursor: 'pointer',
                  background: isActive ? 'rgba(79,142,247,0.15)' : 'transparent',
                  borderLeft: isActive ? '3px solid #4f8ef7' : '3px solid transparent',
                  transition: 'background 0.15s',
                }}
                onMouseEnter={(e) => {
                  if (!isActive) (e.currentTarget as HTMLDivElement).style.background = 'rgba(255,255,255,0.08)';
                }}
                onMouseLeave={(e) => {
                  if (!isActive) (e.currentTarget as HTMLDivElement).style.background = 'transparent';
                }}
              >
                <img
                  src={srcMap[pet.id]}
                  alt={pet.name}
                  style={{ width: 40, height: 40, objectFit: 'contain', imageRendering: 'pixelated', flexShrink: 0 }}
                />
                <span style={{
                  fontSize: 13, color: isActive ? '#4f8ef7' : '#e2e8f0',
                  fontWeight: isActive ? 600 : 400,
                  overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                }}>
                  {pet.name}
                </span>
                {isActive && (
                  <span style={{ marginLeft: 'auto', fontSize: 10, color: '#4f8ef7', flexShrink: 0 }}>✓</span>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
