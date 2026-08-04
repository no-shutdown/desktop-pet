import { useState, useRef } from 'react';
import type { CSSProperties } from 'react';

interface UploadStepProps {
  onNext: (photoDataUrl: string) => void;
}

const dropZoneStyle: CSSProperties = {
  border: '2px dashed #cbd5e0',
  borderRadius: 12,
  padding: '48px 32px',
  textAlign: 'center',
  cursor: 'pointer',
  background: '#f7fafc',
  transition: 'border-color 0.2s',
};

export default function UploadStep({ onNext }: UploadStepProps) {
  const [preview, setPreview] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  function loadFile(file: File) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const result = (e.target as FileReader).result as string;
      setPreview(result);
    };
    reader.readAsDataURL(file);
  }

  function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (file) loadFile(file);
  }

  function handleDrop(e: React.DragEvent) {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files[0];
    if (file) loadFile(file);
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div
        style={{ ...dropZoneStyle, borderColor: dragging ? '#4f8ef7' : '#cbd5e0' }}
        onClick={() => inputRef.current?.click()}
        onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
        onDragLeave={() => setDragging(false)}
        onDrop={handleDrop}
      >
        {preview ? (
          <img alt="preview" src={preview} style={{ maxHeight: 200, maxWidth: '100%', borderRadius: 8 }} />
        ) : (
          <>
            <div style={{ fontSize: 48, marginBottom: 12 }}>📷</div>
            <p style={{ color: '#718096', margin: 0 }}>
              Drag &amp; drop or click to upload a photo
            </p>
            <p style={{ color: '#a0aec0', fontSize: 13, marginTop: 8 }}>
              Supports JPG, PNG, WEBP
            </p>
          </>
        )}
        <input
          data-testid="file-input"
          ref={inputRef}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          style={{ display: 'none' }}
          onChange={handleFileChange}
        />
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button
          onClick={() => preview && onNext(preview)}
          disabled={!preview}
          style={{
            padding: '8px 24px', borderRadius: 6, border: 'none',
            background: preview ? '#4f8ef7' : '#e2e8f0',
            color: '#fff', cursor: preview ? 'pointer' : 'not-allowed', fontSize: 14,
          }}
        >
          Next
        </button>
      </div>
    </div>
  );
}
