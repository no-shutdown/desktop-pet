import { useEffect } from 'react';

interface SpeechBubbleProps {
  text: string | null;
  onHide: () => void;
}

export default function SpeechBubble({ text, onHide }: SpeechBubbleProps) {
  useEffect(() => {
    if (!text) return;
    const timer = setTimeout(onHide, 3000);
    return () => clearTimeout(timer);
  }, [text, onHide]);

  if (!text) return null;

  return (
    <div style={{
      position: 'absolute',
      bottom: '100%',
      left: '50%',
      transform: 'translateX(-50%)',
      background: 'white',
      border: '2px solid #4f8ef7',
      borderRadius: 8,
      padding: '6px 12px',
      fontSize: 13,
      whiteSpace: 'nowrap',
      boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
      marginBottom: 8,
      zIndex: 1000,
      pointerEvents: 'none',
    }}>
      {text}
      <div style={{
        position: 'absolute',
        bottom: -8,
        left: '50%',
        transform: 'translateX(-50%)',
        width: 0,
        height: 0,
        borderLeft: '6px solid transparent',
        borderRight: '6px solid transparent',
        borderTop: '8px solid #4f8ef7',
      }} />
    </div>
  );
}
