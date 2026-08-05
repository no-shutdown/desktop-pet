# Sprite Sheet Animation Refactor

**Date:** 2026-08-05  
**Status:** Approved

## Background

The project currently generates sprite sheets via AI, then immediately slices them into individual frames and re-encodes them as animated GIFs. This pipeline discards the sprite sheet, relies on the browser's GIF decoder for timing, and produces lower quality than keeping the source material. The goal is to remove GIF encoding entirely: save sprite sheet PNGs directly, store layout metadata in `pet.json`, and drive animation from the frontend using Canvas + `requestAnimationFrame`.

## Goals

- Remove all GIF encoding from the pipeline
- Store one PNG sprite sheet per animation state
- Animate via Canvas with precise frame timing and state-switch control
- Manual-upload flow continues to work (single-frame fallback)
- Clean schema break — no backward compatibility with old GIF pets (project is in early development)

## Storage Layout

```
{app_data}/pets/{id}/
  pet.json        ← pet metadata + sprite state descriptors
  idle.png        ← sprite sheet, 256×256 (2 cols × 2 rows × 128px cells)
  walking.png     ← sprite sheet, 256×384 (2 cols × 3 rows × 128px cells)
  waving.png      ← sprite sheet, 256×256 (2 cols × 2 rows × 128px cells)
  working.png     ← sprite sheet, 256×256 (2 cols × 2 rows × 128px cells)
```

Sprite sheet file paths are derived from state name (`{state}.png`) and never stored in `pet.json`. Cell size is uniformly **128×128 px**.

## pet.json Schema

```jsonc
{
  "id": "string",
  "name": "string",
  "prompt": "string",
  "createdAt": "ISO-8601 string",
  "states": {
    "idle":    { "cols": 2, "rows": 2, "frameCount": 4, "frameW": 128, "frameH": 128, "delayMs": 200 },
    "walking": { "cols": 2, "rows": 3, "frameCount": 6, "frameW": 128, "frameH": 128, "delayMs": 150 },
    "waving":  { "cols": 2, "rows": 2, "frameCount": 4, "frameW": 128, "frameH": 128, "delayMs": 150 },
    "working": { "cols": 2, "rows": 2, "frameCount": 4, "frameW": 128, "frameH": 128, "delayMs": 200 }
  }
}
```

The old `frames: { idle, walking, waving, working }` field is removed. `states` replaces it with layout descriptors instead of file paths.

## Backend Changes (`src-tauri`)

### `src-tauri/src/models.rs`

- Remove `PetFrames` struct
- Replace `Pet.frames: PetFrames` with `Pet.states: HashMap<String, SpriteStateInfo>`
- Add `SpriteStateInfo { cols, rows, frame_count, frame_w, frame_h, delay_ms }`

### `src-tauri/src/commands/generate.rs`

**Remove:**
- `assemble_gif_bytes` function
- `save_gif` function

**Change:**
- `decode_sprite_sheet`: resize target changes from `cols × CELL_SIZE` to `cols × 128` (CELL_SIZE effectively becomes 128)
- `apply_chroma_key`: apply to the whole sprite sheet in one pass (no per-frame loop needed)

**Add:**
- `save_sprite_sheet_png(pets_dir, pet_id, state, sheet: &RgbaImage) -> Result<(), String>` — encodes sheet as PNG and writes to `{state}.png`

**`generate_and_assemble` new return type:** `Result<HashMap<String, SpriteStateInfo>, String>`
- Returns the states map to the frontend after all states are saved
- Frontend uses this to build the `Pet` object and pass it to `SaveStep`

**`generate_and_assemble` new flow per state:**
```
build prompt
→ fetch from AI provider
→ decode + resize sheet to cols×128 × rows×128
→ apply_chroma_key to sheet
→ save_sprite_sheet_png
→ emit progress
```

After all states complete, return the assembled `states` map.

**`save_custom_frames`:**
- Accept uploaded image (any format, via data URL) for each state
- Decode → resize to 128×128 → save as 1×1 PNG sprite sheet
- State descriptor: `{ cols: 1, rows: 1, frameCount: 1, frameW: 128, frameH: 128, delayMs: 200 }`

### `Cargo.toml`

Remove `gif` crate dependency.

## Frontend Changes (`src`)

### `src/types/pet.ts`

```ts
export interface SpriteStateInfo {
  cols: number;
  rows: number;
  frameCount: number;
  frameW: number;
  frameH: number;
  delayMs: number;
}

export interface Pet {
  id: string;
  name: string;
  prompt: string;
  createdAt: string;
  states: Record<PetState, SpriteStateInfo>;
}

export type PetState = 'idle' | 'walking' | 'waving' | 'working';
export const PET_STATES: PetState[] = ['idle', 'walking', 'waving', 'working'];
```

### `src/windows/Pet/SpriteAnimator.tsx` (new file)

Canvas-based animator component.

**Props:**
```ts
interface SpriteAnimatorProps {
  sheetSrc: string;          // convertFileSrc path to PNG
  meta: SpriteStateInfo;
  displayW?: number;         // defaults to meta.frameW
  displayH?: number;         // defaults to meta.frameH
}
```

**Behavior:**
- Maintains a `frameIndex` ref and a `lastTimestamp` ref
- `requestAnimationFrame` loop: when elapsed ≥ `meta.delayMs`, advance frame, `clearRect`, `drawImage` with correct `srcX/srcY`
- When `sheetSrc` or `meta` changes (state switch): reset `frameIndex` to 0, reload `Image` object, restart RAF loop
- Canvas dimensions: `displayW × displayH` (defaults to `frameW × frameH` = 128×128, but `Pet/index.tsx` will pass 200×200 for the display size)
- `imageRendering: pixelated` on canvas element

**Frame indexing:**
```
frameIndex → row = floor(frameIndex / cols), col = frameIndex % cols
srcX = col * frameW, srcY = row * frameH
```

### `src/windows/Pet/index.tsx`

- Remove `PET_STATES.map(state => <img ...>)` block
- Replace with:
  ```tsx
  <SpriteAnimator
    sheetSrc={convertFileSrc(`${petsBasePath}/${activePet.id}/${petState}.png`)}
    meta={activePet.states[petState]}
    displayW={200}
    displayH={200}
  />
  ```
- Add a new Tauri command `get_pets_dir() -> Result<String, String>` that returns the absolute path to the pets directory. `Pet/index.tsx` calls this once on mount and stores the result in a ref. Sheet src is then `convertFileSrc(\`\${petsDir}/${activePet.id}/${petState}.png\`)`.

### `src/windows/Creator/steps/SaveStep.tsx`

- Update the pet object it saves to use `states` instead of `frames`
- The `states` map is passed down from `GenerateStep` (which received it from the backend after `generate_and_assemble`)

### `src/windows/Creator/steps/GenerateStep.tsx`

- After `generate_and_assemble` succeeds, the command now returns `HashMap<String, SpriteStateInfo>` (or the full `Pet` object)
- Pass this to `SaveStep` via `onNext`

## Data Flow

```
GenerateStep
  → invoke generate_and_assemble(pet_id, prompt, ...)
  → backend saves {state}.png files + returns states map
  → onNext(petId, statesMap)

SaveStep
  → invoke save_pet({ id, name, prompt, createdAt, states })
  → backend writes pet.json
  → emits "pet-saved" with Pet object

PetWindow
  → receives "pet-saved" or loads from list_pets
  → activePet.states[petState] drives SpriteAnimator
  → petState changes → SpriteAnimator switches sheet src + resets frame counter
```

## Testing Notes

- Existing unit tests for `build_state_specs`, `apply_chroma_key`, `slice_sprite_sheet` can be removed or adapted (slice_sprite_sheet no longer needed in the pipeline)
- New test: `save_sprite_sheet_png` writes a valid PNG with correct dimensions
- New test: `SpriteAnimator` renders frame 0 on mount, advances to frame 1 after `delayMs` ms (using fake timers)
- `types/pet.test.ts`: update to new schema

## File Change Summary

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | Remove `gif` crate |
| `src-tauri/src/models.rs` | Remove `PetFrames`, add `SpriteStateInfo`, update `Pet` |
| `src-tauri/src/commands/generate.rs` | Remove GIF fns, save PNG sheets, update `save_custom_frames` |
| `src-tauri/src/commands/pet.rs` | Add `get_pets_dir` command |
| `src-tauri/src/lib.rs` | Register `get_pets_dir` in invoke handler |
| `src/types/pet.ts` | Replace `PetFrames`/`frames` with `SpriteStateInfo`/`states` |
| `src/windows/Pet/SpriteAnimator.tsx` | **New** Canvas animator |
| `src/windows/Pet/index.tsx` | Replace img stack with `<SpriteAnimator>` |
| `src/windows/Creator/steps/SaveStep.tsx` | Use `states` field |
| `src/windows/Creator/steps/GenerateStep.tsx` | Receive and forward `states` from backend |
| `src/types/__tests__/pet.test.ts` | Update to new schema |
