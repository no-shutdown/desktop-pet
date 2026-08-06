import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  PET_STATE_CATALOG,
  PET_STATES,
  type PetState,
  type SpriteStateInfo,
} from '../../../types/pet';

type ActionKey = PetState;

interface Cell {
  col: number;
  row: number;
}

type Selections = Record<ActionKey, Cell[]>;

interface Props {
  onNext: (petId: string, states: Record<PetState, SpriteStateInfo>, runId: string) => void;
  onBack: () => void;
  initialDataUrl?: string | null;
  initialPetId?: string | null;
  initialConfig?: {
    runId?: string;
    frameW?: number;
    frameH?: number;
    colGap?: number;
    rowGap?: number;
    layout?: 'horizontalRows' | 'grid';
    idleFrames?: number;
    walkingFrames?: number;
    wavingFrames?: number;
    workingFrames?: number;
  };
}

const ACTION_COLORS: Record<PetState, string> = {
  idle: '#4f8ef7',
  walking: '#48bb78',
  waving: '#ed8936',
  working: '#e53e3e',
};

const ACTION_META = PET_STATE_CATALOG.map((definition) => ({
  ...definition,
  color: ACTION_COLORS[definition.key],
}));

function createEmptySelections(): Selections {
  return PET_STATES.reduce((result, state) => {
    result[state] = [];
    return result;
  }, {} as Selections);
}

function cellKey(cell: Cell): string {
  return `${cell.col},${cell.row}`;
}

function cellsEqual(a: Cell, b: Cell): boolean {
  return a.col === b.col && a.row === b.row;
}

function makeStagingRunId(): string {
  if (typeof globalThis.crypto?.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  return `staging-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export default function ManualFramePickerStep({
  onNext,
  onBack,
  initialDataUrl,
  initialPetId,
  initialConfig,
}: Props) {
  const [stagingRunId] = useState(() => initialConfig?.runId ?? makeStagingRunId());
  const [dataUrl, setDataUrl]       = useState<string | null>(initialDataUrl ?? null);
  const [loadedImg, setLoadedImg]   = useState<HTMLImageElement | null>(null);
  const [frameW, setFrameW]         = useState<number>(initialConfig?.frameW ?? 128);
  const [frameH, setFrameH]         = useState<number>(initialConfig?.frameH ?? 128);
  const initialLayout = initialConfig?.layout ?? (initialConfig ? 'horizontalRows' : 'grid');
  const [colGap, setColGap]         = useState<number>(initialLayout === 'horizontalRows' ? 0 : initialConfig?.colGap ?? 0);
  const [rowGap, setRowGap]         = useState<number>(initialLayout === 'horizontalRows' ? 0 : initialConfig?.rowGap ?? 0);
  const [selections, setSelections] = useState<Selections>(createEmptySelections);
  const [activeAction, setActiveAction] = useState<ActionKey>('idle');
  const [saving, setSaving]         = useState(false);
  const [error, setError]           = useState<string | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const imageIdentityRef = useRef(0);
  const loadedImageIdentityRef = useRef<number | null>(null);
  const imageSourceRef = useRef<'initial' | 'external'>(initialDataUrl ? 'initial' : 'external');

  // Treat every incoming image URL as a new image identity and ignore stale loads.
  useEffect(() => {
    const imageIdentity = ++imageIdentityRef.current;
    imageSourceRef.current = initialDataUrl ? 'initial' : 'external';
    loadedImageIdentityRef.current = null;
    setDataUrl(initialDataUrl ?? null);
    setLoadedImg(null);
    setSelections(createEmptySelections());

    if (!initialDataUrl) return;

    const img = new Image();
    img.onload = () => {
      if (imageIdentity === imageIdentityRef.current) {
        loadedImageIdentityRef.current = imageIdentity;
        setLoadedImg(img);
      }
    };
    img.src = initialDataUrl;

    return () => {
      img.onload = null;
    };
  }, [initialDataUrl]);

  // Apply generated layout settings and auto-fill its four state rows.
  useEffect(() => {
    if (!initialConfig || imageSourceRef.current !== 'initial') {
      setSelections(createEmptySelections());
      return;
    }

    const layout = initialConfig.layout ?? 'horizontalRows';
    setFrameW(initialConfig.frameW ?? 128);
    setFrameH(initialConfig.frameH ?? 128);
    setColGap(layout === 'horizontalRows' ? 0 : initialConfig.colGap ?? 0);
    setRowGap(layout === 'horizontalRows' ? 0 : initialConfig.rowGap ?? 0);

    if (!loadedImg || loadedImageIdentityRef.current !== imageIdentityRef.current) return;

    const frameCounts: Record<PetState, number> = {
      idle: initialConfig.idleFrames ?? 0,
      walking: initialConfig.walkingFrames ?? 0,
      waving: initialConfig.wavingFrames ?? 0,
      working: initialConfig.workingFrames ?? 0,
    };

    setSelections(PET_STATES.reduce((result, state, row) => {
      result[state] = Array.from({ length: frameCounts[state] }, (_, col) => ({ col, row }));
      return result;
    }, createEmptySelections()));
  }, [
    loadedImg,
    initialConfig?.layout,
    initialConfig?.frameW,
    initialConfig?.frameH,
    initialConfig?.colGap,
    initialConfig?.rowGap,
    initialConfig?.idleFrames,
    initialConfig?.walkingFrames,
    initialConfig?.wavingFrames,
    initialConfig?.workingFrames,
  ]);

  // Redraw canvas whenever relevant state changes.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !loadedImg) return;

    const scale = Math.min(460 / loadedImg.naturalWidth, 320 / loadedImg.naturalHeight, 1);
    canvas.width  = Math.round(loadedImg.naturalWidth  * scale);
    canvas.height = Math.round(loadedImg.naturalHeight * scale);

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.drawImage(loadedImg, 0, 0, canvas.width, canvas.height);

    const cellStride = frameW + colGap;
    const rowStride  = frameH + rowGap;
    const cols = cellStride > 0 ? Math.ceil(loadedImg.naturalWidth  / cellStride) : 1;
    const rows = rowStride  > 0 ? Math.ceil(loadedImg.naturalHeight / rowStride)  : 1;

    // Draw grid lines.
    ctx.strokeStyle = 'rgba(0,0,0,0.2)';
    ctx.lineWidth   = 0.5;
    for (let col = 0; col <= cols; col++) {
      const x = col * cellStride * scale;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, canvas.height);
      ctx.stroke();
    }
    for (let row = 0; row <= rows; row++) {
      const y = row * rowStride * scale;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(canvas.width, y);
      ctx.stroke();
    }

    // Build a lookup map: cellKey -> { action, index }
    const cellOwner = new Map<string, { action: ActionKey; index: number }>();
    for (const { key } of ACTION_META) {
      selections[key].forEach((cell, idx) => {
        cellOwner.set(cellKey(cell), { action: key, index: idx });
      });
    }

    // Draw colored overlays and sequence numbers for selected cells.
    cellOwner.forEach(({ action, index }, key) => {
      const parts = key.split(',');
      const col = parseInt(parts[0], 10);
      const row = parseInt(parts[1], 10);
      const meta = ACTION_META.find((m) => m.key === action)!;

      const x = col * cellStride * scale;
      const y = row * rowStride  * scale;
      const w = frameW * scale;
      const h = frameH * scale;

      ctx.fillStyle = meta.color + '44';
      ctx.fillRect(x, y, w, h);

      ctx.strokeStyle = meta.color;
      ctx.lineWidth   = 2;
      ctx.strokeRect(x + 1, y + 1, w - 2, h - 2);

      // Sequence number (1-based).
      const label = String(index + 1);
      const fontSize = Math.max(10, Math.round(13 * scale));
      ctx.font      = `bold ${fontSize}px system-ui`;
      ctx.fillStyle = meta.color;
      ctx.fillText(label, x + 4, y + fontSize + 2);
    });
  }, [loadedImg, frameW, frameH, colGap, rowGap, selections]);

  const handleCanvasClick = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas || !loadedImg) return;

      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width  / rect.width;
      const scaleY = canvas.height / rect.height;
      const xPx = (e.clientX - rect.left) * scaleX;
      const yPx = (e.clientY - rect.top)  * scaleY;

      // Convert from canvas pixels back to image pixels.
      const imgScale = Math.min(460 / loadedImg.naturalWidth, 320 / loadedImg.naturalHeight, 1);
      const imgX = xPx / imgScale;
      const imgY = yPx / imgScale;

      const cellStride = frameW + colGap;
      const rowStride  = frameH + rowGap;

      const col = Math.floor(imgX / (cellStride > 0 ? cellStride : 1));
      const row = Math.floor(imgY / (rowStride  > 0 ? rowStride  : 1));

      // Check that click lands inside a frame (not in a gap).
      const inFrameX = colGap === 0 || (imgX % cellStride) < frameW;
      const inFrameY = rowGap === 0 || (imgY % rowStride)  < frameH;
      if (!inFrameX || !inFrameY) return;

      const clicked: Cell = { col, row };
      const clickedKey    = cellKey(clicked);

      setSelections((prev) => {
        const currentList = prev[activeAction];

        // Already selected for this action -> remove it.
        if (currentList.some((cell) => cellsEqual(cell, clicked))) {
          return {
            ...prev,
            [activeAction]: currentList.filter((cell) => !cellsEqual(cell, clicked)),
          };
        }

        // Selected for a different action -> ignore (don't steal).
        for (const { key } of ACTION_META) {
          if (key !== activeAction && prev[key].some((cell) => cellKey(cell) === clickedKey)) {
            return prev;
          }
        }

        // Not selected anywhere -> add to current action.
        return {
          ...prev,
          [activeAction]: [...currentList, clicked],
        };
      });
    },
    [loadedImg, frameW, frameH, colGap, rowGap, activeAction],
  );

  function handleFile(file: File) {
    const imageIdentity = ++imageIdentityRef.current;
    imageSourceRef.current = 'external';
    loadedImageIdentityRef.current = null;
    setError(null);
    setDataUrl(null);
    setLoadedImg(null);
    setSelections(createEmptySelections());

    const reader = new FileReader();
    reader.onload = (ev) => {
      if (imageIdentity !== imageIdentityRef.current) return;

      const url = (ev.target as FileReader).result as string;
      setDataUrl(url);
      const img = new Image();
      img.onload = () => {
        if (imageIdentity !== imageIdentityRef.current) return;

        loadedImageIdentityRef.current = imageIdentity;
        setLoadedImg(img);
        // Auto-suggest frame sizes based on image dimensions.
        const suggestH = Math.round(img.height / 4);
        const suggestW = img.width > 0 ? Math.round(img.width / 4) : 128;
        if (suggestH > 0 && suggestW > 0) {
          setFrameW(suggestW);
          setFrameH(suggestH);
        }
      };
      img.src = url;
    };
    reader.readAsDataURL(file);
  }

  async function handleSave() {
    if (!dataUrl) return;
    setSaving(true);
    setError(null);
    try {
      const petId = initialPetId ?? crypto.randomUUID();
      const result = await invoke<Record<PetState, SpriteStateInfo>>('stage_frame_selections', {
        runId: stagingRunId,
        dataUrl,
        frameW,
        frameH,
        colGap,
        rowGap,
        idleCells:    selections.idle,
        walkingCells: selections.walking,
        wavingCells:  selections.waving,
        workingCells: selections.working,
      });
      onNext(petId, result, stagingRunId);
    } catch (err) {
      setError((err as Error).message ?? String(err));
    } finally {
      setSaving(false);
    }
  }

  const allReady = ACTION_META.every(({ key }) => selections[key].length > 0);
  const canSave  = !!dataUrl && !saving && allReady;

  const THUMB_SIZE = 40;

  function renderThumbnail(cell: Cell, actionColor: string, idx: number) {
    if (!loadedImg) return null;
    const srcX    = cell.col * (frameW + colGap);
    const srcY    = cell.row * (frameH + rowGap);
    const thumbScale = THUMB_SIZE / Math.max(frameW, frameH, 1);
    const thumbW  = Math.round(frameW * thumbScale);
    const thumbH  = Math.round(frameH * thumbScale);

    return (
      <div key={idx} style={{ position: 'relative', flexShrink: 0 }}>
        <div
          style={{
            width: thumbW,
            height: thumbH,
            backgroundImage: `url(${dataUrl})`,
            backgroundPosition: `-${srcX * thumbScale}px -${srcY * thumbScale}px`,
            backgroundSize: `${loadedImg.naturalWidth * thumbScale}px ${loadedImg.naturalHeight * thumbScale}px`,
            backgroundRepeat: 'no-repeat',
            imageRendering: 'pixelated' as const,
            border: `1px solid ${actionColor}`,
            borderRadius: 4,
            flexShrink: 0,
          }}
        />
        <button
          onClick={() => {
            setSelections((prev) => ({
              ...prev,
              [activeAction]: prev[activeAction].filter((_, i) => i !== idx),
            }));
          }}
          style={{
            position: 'absolute',
            top: -5,
            right: -5,
            width: 14,
            height: 14,
            borderRadius: '50%',
            border: 'none',
            background: '#e53e3e',
            color: '#fff',
            fontSize: 9,
            lineHeight: '14px',
            textAlign: 'center',
            cursor: 'pointer',
            padding: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          x
        </button>
      </div>
    );
  }

  const activeMeta    = ACTION_META.find((m) => m.key === activeAction)!;
  const activeFrames  = selections[activeAction];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {/* Upload zone */}
      <label
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 8,
          border: `2px dashed ${dataUrl ? '#4f8ef7' : '#e2e8f0'}`,
          borderRadius: 10,
          padding: dataUrl ? '10px 16px' : '20px 16px',
          cursor: 'pointer',
          background: dataUrl ? '#f0f5ff' : '#fafafa',
          transition: 'border-color 0.2s',
        }}
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => {
          e.preventDefault();
          const file = e.dataTransfer.files[0];
          if (file) handleFile(file);
        }}
      >
        <input
          type="file"
          accept=".png,.jpg,.jpeg,.webp"
          style={{ display: 'none' }}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) handleFile(file);
          }}
        />
        {dataUrl ? (
          <span style={{ fontSize: 13, color: '#4f8ef7' }}>已上传 — 点击更换</span>
        ) : (
          <>
            <span style={{ fontSize: 36 }}>🖼️</span>
            <span style={{ fontSize: 13, color: '#718096' }}>点击或拖入精灵图 PNG</span>
          </>
        )}
      </label>

      {/* Two-panel layout: canvas left, config right */}
      {loadedImg && (
        <div style={{ display: 'flex', gap: 16, alignItems: 'flex-start' }}>
          {/* Left: canvas */}
          <div
            style={{
              flex: '0 0 auto',
              border: '1px solid #e2e8f0',
              borderRadius: 8,
              overflow: 'hidden',
              cursor: 'crosshair',
            }}
          >
            <canvas
              ref={canvasRef}
              onClick={handleCanvasClick}
              style={{ display: 'block' }}
            />
          </div>

          {/* Right: config + action picker + thumbnails */}
          <div
            style={{
              width: 220,
              flexShrink: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 12,
            }}
          >
            {/* Grid size config */}
            <div style={{ fontSize: 11, fontWeight: 700, color: '#a0aec0', textTransform: 'uppercase', letterSpacing: '0.06em' }}>
              网格尺寸
            </div>

            {([
              { label: '帧宽 (px)',  value: frameW, setter: setFrameW, min: 1 },
              { label: '帧高 (px)',  value: frameH, setter: setFrameH, min: 1 },
              { label: '列间距 (px)', value: colGap, setter: setColGap, min: 0 },
              { label: '行间距 (px)', value: rowGap, setter: setRowGap, min: 0 },
            ] as const).map(({ label, value, setter, min }) => (
              <label
                key={label}
                style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 8, fontSize: 13 }}
              >
                <span style={{ color: '#4a5568', whiteSpace: 'nowrap' }}>{label}</span>
                <input
                  type="number"
                  min={min}
                  value={value}
                  onChange={(e) => setter(Math.max(min, parseInt(e.target.value) || 0))}
                  style={{ width: 60, padding: '3px 6px', borderRadius: 4, border: '1px solid #e2e8f0', fontSize: 13, textAlign: 'right' }}
                />
              </label>
            ))}

            {/* Action tab buttons */}
            <div style={{ fontSize: 11, fontWeight: 700, color: '#a0aec0', textTransform: 'uppercase', letterSpacing: '0.06em', marginTop: 4 }}>
              当前动作
            </div>

            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
              {ACTION_META.map(({ key, label, color }) => {
                const count    = selections[key].length;
                const isActive = key === activeAction;
                return (
                  <button
                    key={key}
                    onClick={() => setActiveAction(key)}
                    style={{
                      padding: '4px 10px',
                      borderRadius: 6,
                      border: `1px solid ${color}`,
                      background: isActive ? color : '#fff',
                      color: isActive ? '#fff' : color,
                      cursor: 'pointer',
                      fontSize: 12,
                      fontWeight: 600,
                      transition: 'background 0.15s, color 0.15s',
                    }}
                  >
                    {label}({count})
                  </button>
                );
              })}
            </div>

            {/* Selected frames thumbnails */}
            <div style={{ fontSize: 11, fontWeight: 700, color: '#a0aec0', textTransform: 'uppercase', letterSpacing: '0.06em', marginTop: 4 }}>
              已选帧 {activeMeta.label}
            </div>

            <div
              style={{
                display: 'flex',
                flexWrap: 'wrap',
                gap: 6,
                minHeight: 48,
                padding: 6,
                border: '1px solid #e2e8f0',
                borderRadius: 6,
                background: '#fafafa',
              }}
            >
              {activeFrames.length === 0 ? (
                <span style={{ fontSize: 12, color: '#a0aec0', alignSelf: 'center', margin: '0 auto' }}>
                  （空）
                </span>
              ) : (
                activeFrames.map((cell, idx) => renderThumbnail(cell, activeMeta.color, idx))
              )}
            </div>

            {activeFrames.length > 0 && (
              <button
                onClick={() => setSelections((prev) => ({ ...prev, [activeAction]: [] }))}
                style={{
                  alignSelf: 'flex-start',
                  padding: '3px 10px',
                  borderRadius: 4,
                  border: '1px solid #e2e8f0',
                  background: '#fff',
                  color: '#718096',
                  fontSize: 12,
                  cursor: 'pointer',
                }}
              >
                清空
              </button>
            )}

            <p style={{ fontSize: 11, color: '#a0aec0', margin: 0, lineHeight: 1.5 }}>
              点击左侧图片中的格子来选择帧
            </p>

            {/* Validation status */}
            <div style={{ fontSize: 12, lineHeight: 1.6 }}>
              {allReady ? (
                <span style={{ color: '#38a169' }}>各动作已就绪</span>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  {ACTION_META.filter(({ key }) => selections[key].length === 0).map(({ key, label }) => (
                    <span key={key} style={{ color: '#e53e3e' }}>
                      {label} 需要至少1帧
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {error && (
        <p style={{ color: '#e53e3e', fontSize: 13, margin: 0, whiteSpace: 'pre-wrap' }}>{error}</p>
      )}

      {/* Bottom buttons */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12, marginTop: 4 }}>
        <button
          onClick={onBack}
          disabled={saving}
          style={{
            padding: '8px 20px',
            borderRadius: 6,
            border: '1px solid #e2e8f0',
            background: '#fff',
            color: '#4a5568',
            cursor: saving ? 'not-allowed' : 'pointer',
          }}
        >
          返回
        </button>
        <button
          onClick={handleSave}
          disabled={!canSave}
          style={{
            padding: '8px 24px',
            borderRadius: 6,
            border: 'none',
            background: canSave ? '#4f8ef7' : '#e2e8f0',
            color: '#fff',
            cursor: canSave ? 'pointer' : 'not-allowed',
          }}
        >
          {saving ? '处理中…' : '确认导入'}
        </button>
      </div>
    </div>
  );
}
