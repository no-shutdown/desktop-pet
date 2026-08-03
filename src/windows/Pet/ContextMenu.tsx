import type { CSSProperties } from 'react';

interface ContextMenuProps {
  x: number;
  y: number;
  visible: boolean;
  onClose: () => void;
  onSwitchPet: () => void;
  onOpenCreator: () => void;
  onExit: () => void;
}

const itemStyle: CSSProperties = {
  padding: '8px 16px',
  cursor: 'pointer',
  fontSize: 14,
  userSelect: 'none',
  whiteSpace: 'nowrap',
};

export default function ContextMenu({ x, y, visible, onClose, onSwitchPet, onOpenCreator, onExit }: ContextMenuProps) {
  if (!visible) return null;

  return (
    <>
      <div
        data-testid="backdrop"
        style={{ position: 'fixed', inset: 0, zIndex: 999 }}
        onClick={onClose}
      />
      <div style={{
        position: 'fixed',
        left: x,
        top: y,
        background: '#fff',
        border: '1px solid #e2e8f0',
        borderRadius: 6,
        boxShadow: '0 4px 16px rgba(0,0,0,0.12)',
        zIndex: 1000,
        overflow: 'hidden',
        minWidth: 160,
      }}>
        <div style={{ ...itemStyle, color: '#2d3748' }} onClick={onSwitchPet}>
          Switch Pet
        </div>
        <div style={{ ...itemStyle, color: '#2d3748' }} onClick={onOpenCreator}>
          Open Creator
        </div>
        <div style={{ borderTop: '1px solid #e2e8f0' }} />
        <div style={{ ...itemStyle, color: '#e53e3e' }} onClick={onExit}>
          Exit
        </div>
      </div>
    </>
  );
}
