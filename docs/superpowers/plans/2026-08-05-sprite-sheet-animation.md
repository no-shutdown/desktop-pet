# Sprite Sheet Animation Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove GIF encoding entirely; save AI-generated sprite sheet PNGs directly and animate them in a Canvas component driven by `requestAnimationFrame`.

**Architecture:** Backend pipeline changes from `sprite-sheet → slice → GIF` to `sprite-sheet → resize → apply chroma key → save PNG`. Frontend replaces stacked `<img>` tags with a `<SpriteAnimator>` Canvas component that reads frame layout from metadata stored in `pet.json`. The Creator wizard passes the states map (returned from `generate_and_assemble`) through the step chain to `SaveStep`.

**Tech Stack:** Rust/Tauri (image crate for PNG encode), React 19, TypeScript, Vitest + Testing Library, `requestAnimationFrame`

---

## File Map

| File | Action |
|---|---|
| `src-tauri/src/models.rs` | Replace `PetFrames` + `Pet.frames` with `SpriteStateInfo` + `Pet.states` |
| `src-tauri/src/commands/generate.rs` | Remove GIF fns; change CELL_SIZE→128; add `save_sprite_sheet_png`; update both commands |
| `src-tauri/src/commands/pet.rs` | Update `make_pet` test helper only |
| `src-tauri/Cargo.toml` | Remove `gif` dep |
| `src/types/pet.ts` | Add `SpriteStateInfo`; replace `PetFrames`/`frames` with `states` |
| `src/types/__tests__/pet.test.ts` | Update to new schema |
| `src/windows/Creator/steps/types.ts` | Add `petStates` field to `WizardData` |
| `src/windows/Pet/SpriteAnimator.tsx` | **New** Canvas animator component |
| `src/windows/Pet/__tests__/SpriteAnimator.test.tsx` | **New** unit tests |
| `src/windows/Pet/index.tsx` | Replace img stack with `<SpriteAnimator>`; load petsDir on mount |
| `src/windows/Creator/steps/GenerateStep.tsx` | Capture states from invoke; update `onNext` signature |
| `src/windows/Creator/steps/DirectUploadStep.tsx` | Capture states from `save_custom_frames`; update `onNext` |
| `src/windows/Creator/index.tsx` | Update callbacks for GenerateStep + DirectUploadStep; pass states to SaveStep |
| `src/windows/Creator/steps/PreviewStep.tsx` | Change `.gif` → `.png` |
| `src/windows/Creator/steps/SaveStep.tsx` | Accept `states` prop; remove GIF path construction |
| `src/store/__tests__/petStore.test.ts` | Update any `Pet` fixtures using `frames` |

---

## Task 1 — Rust models: SpriteStateInfo + updated Pet

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Write failing test**

Replace the content of `src-tauri/src/models.rs` with:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteStateInfo {
    pub cols: usize,
    pub rows: usize,
    pub frame_count: usize,
    pub frame_w: u32,
    pub frame_h: u32,
    pub delay_ms: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pet {
    pub id: String,
    pub name: String,
    pub states: HashMap<String, SpriteStateInfo>,
    pub created_at: String,
    pub prompt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sprite_state() -> SpriteStateInfo {
        SpriteStateInfo { cols: 2, rows: 2, frame_count: 4, frame_w: 128, frame_h: 128, delay_ms: 200 }
    }

    fn make_pet() -> Pet {
        let mut states = HashMap::new();
        for s in &["idle", "walking", "waving", "working"] {
            states.insert(s.to_string(), make_sprite_state());
        }
        Pet {
            id: "test-id".to_string(),
            name: "My Pet".to_string(),
            states,
            created_at: "2026-08-03T10:00:00Z".to_string(),
            prompt: "anime chibi girl".to_string(),
        }
    }

    #[test]
    fn pet_round_trips_through_json() {
        let pet = make_pet();
        let json = serde_json::to_string(&pet).unwrap();
        let result: Pet = serde_json::from_str(&json).unwrap();
        assert_eq!(pet, result);
    }

    #[test]
    fn pet_has_all_four_states() {
        let pet = make_pet();
        assert!(pet.states.contains_key("idle"));
        assert!(pet.states.contains_key("walking"));
        assert!(pet.states.contains_key("waving"));
        assert!(pet.states.contains_key("working"));
    }

    #[test]
    fn sprite_state_info_serializes_camel_case() {
        let info = make_sprite_state();
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("frameCount"));
        assert!(json.contains("frameW"));
        assert!(json.contains("frameH"));
        assert!(json.contains("delayMs"));
    }
}
```

- [ ] **Step 2: Run Rust tests (expect failures due to compile errors in pet.rs/generate.rs)**

```bash
cargo test -p desktop-pet --lib 2>&1 | head -50
```

Expected: compile errors referencing `PetFrames` in `pet.rs` and `generate.rs`. That's expected — we'll fix those next.

- [ ] **Step 3: Commit models**

```bash
git add src-tauri/src/models.rs
git commit -m "refactor: replace PetFrames with SpriteStateInfo in Rust models"
```

---

## Task 2 — Rust generate.rs: remove GIF pipeline, save PNG sprite sheets

**Files:**
- Modify: `src-tauri/src/commands/generate.rs`

- [ ] **Step 1: Replace the entire file**

Write `src-tauri/src/commands/generate.rs`:

```rust
use image::{RgbaImage, imageops};
use std::collections::HashMap;
use crate::models::SpriteStateInfo;

pub const FRAME_COUNTS: &[(&str, usize)] = &[
    ("idle",    4),
    ("walking", 6),
    ("waving",  4),
    ("working", 4),
];

pub const TOTAL_STATES: usize = FRAME_COUNTS.len();

const CELL_SIZE: u32 = 128;

fn state_action_prompt(state: &str) -> &str {
    match state {
        "idle"    => "standing still, relaxed natural pose, slight smile, subtle breathing motion",
        "walking" => "walking cycle, legs alternating steps, arms swinging naturally at sides",
        "waving"  => "waving hand high with cheerful big smile, arm raised above head",
        "working" => "focused working pose, leaning forward slightly, thinking or typing expression",
        _         => "neutral pose",
    }
}

pub struct StateSpec {
    pub state: String,
    pub frame_count: usize,
    pub cols: usize,
    pub rows: usize,
}

impl StateSpec {
    pub fn img_width(&self) -> u32  { self.cols as u32 * CELL_SIZE }
    pub fn img_height(&self) -> u32 { self.rows as u32 * CELL_SIZE }
}

pub fn build_state_specs() -> Vec<StateSpec> {
    FRAME_COUNTS.iter().map(|(state, count)| {
        let cols = 2usize;
        let rows = (count + cols - 1) / cols;
        StateSpec { state: state.to_string(), frame_count: *count, cols, rows }
    }).collect()
}

pub fn build_sprite_prompt(base_prompt: &str, spec: &StateSpec) -> String {
    let truncated = if base_prompt.len() > 300 {
        let mut end = 300;
        while !base_prompt.is_char_boundary(end) { end -= 1; }
        &base_prompt[..end]
    } else {
        base_prompt
    };
    let action = state_action_prompt(&spec.state);
    format!(
        "{}, {}, chibi pixel art, simple flat colors, pure white background, full body character, sprite sheet {}x{} grid layout, {} sequential animation frames, same consistent character in every cell",
        truncated, action, spec.cols, spec.rows, spec.frame_count
    )
}

pub fn build_pollinations_url(prompt: &str, width: u32, height: u32) -> String {
    let truncated = if prompt.len() > 450 {
        let mut end = 450;
        while !prompt.is_char_boundary(end) { end -= 1; }
        &prompt[..end]
    } else {
        prompt
    };
    let encoded = urlencoding::encode(truncated);
    format!(
        "https://image.pollinations.ai/prompt/{}?width={}&height={}&nologo=true&model=flux",
        encoded, width, height
    )
}

pub fn apply_chroma_key(img: &mut RgbaImage, threshold: u8) {
    for pixel in img.pixels_mut() {
        let [r, g, b, _] = pixel.0;
        if r > 255 - threshold && g > 255 - threshold && b > 255 - threshold {
            pixel.0 = [0, 0, 0, 0];
        }
    }
}

pub fn decode_sprite_sheet(bytes: &[u8], width: u32, height: u32) -> Result<RgbaImage, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let resized = img.resize_exact(width, height, imageops::FilterType::Lanczos3);
    Ok(resized.to_rgba8())
}

pub fn save_sprite_sheet_png(
    pets_dir: &std::path::PathBuf,
    pet_id: &str,
    state: &str,
    sheet: &RgbaImage,
) -> Result<(), String> {
    let dir = pets_dir.join(pet_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    sheet
        .save(dir.join(format!("{}.png", state)))
        .map_err(|e| e.to_string())
}

pub async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut last_err = String::new();
    for attempt in 0..3u64 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(5 * attempt)).await;
        }
        let resp = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => { last_err = e.to_string(); continue; }
        };
        if resp.status().is_success() {
            return resp.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string());
        }
        last_err = format!("HTTP {} downloading image", resp.status());
    }
    Err(last_err)
}

async fn fetch_image_siliconflow(
    prompt: &str, api_key: &str, model: &str, width: u32, height: u32,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "image_size": format!("{}x{}", width, height),
        "num_inference_steps": 20,
        "num_images": 1,
    });
    let resp = client
        .post("https://api.siliconflow.cn/v1/images/generations")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("SiliconFlow API error {}: {}", status, body_text));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let image_url = data["images"][0]["url"]
        .as_str()
        .ok_or_else(|| "SiliconFlow: missing image URL in response".to_string())?;
    download_image(image_url).await
}

async fn fetch_image_localsd(
    prompt: &str, sd_url: &str, width: u32, height: u32,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "prompt": prompt,
        "negative_prompt": "ugly, blurry, watermark, multiple characters",
        "steps": 20,
        "width": width,
        "height": height,
        "batch_size": 1,
    });
    let endpoint = format!("{}/sdapi/v1/txt2img", sd_url.trim_end_matches('/'));
    let resp = client.post(&endpoint).json(&body).send().await
        .map_err(|e| format!("Local SD connection failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Local SD API error: {}", resp.status()));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let b64 = data["images"][0].as_str()
        .ok_or_else(|| "Local SD: missing image in response".to_string())?;
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("Local SD base64 decode error: {e}"))
}

#[tauri::command]
pub async fn generate_and_assemble(
    app: tauri::AppHandle,
    pet_id: String,
    base_prompt: String,
    image_provider: Option<String>,
    image_api_key: Option<String>,
    image_model: Option<String>,
    local_sd_url: Option<String>,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    use tauri::Manager;
    use tauri::Emitter;

    let provider = image_provider.as_deref().unwrap_or("pollinations");
    let pets_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("pets");
    let state_specs = build_state_specs();
    let mut current: u32 = 0;
    let mut states: HashMap<String, SpriteStateInfo> = HashMap::new();

    for spec in &state_specs {
        let prompt = build_sprite_prompt(&base_prompt, spec);
        let target_w = spec.img_width();
        let target_h = spec.img_height();

        let fetch_result = match provider {
            "siliconflow" => {
                let key = image_api_key.as_deref()
                    .ok_or_else(|| "SiliconFlow requires an API key (configure in Settings)".to_string())?;
                let model = image_model.as_deref().unwrap_or("Tongyi-MAI/Z-Image-Turbo");
                fetch_image_siliconflow(&prompt, key, model, target_w, target_h).await
            }
            "localsd" => {
                let url = local_sd_url.as_deref().unwrap_or("http://localhost:7860");
                fetch_image_localsd(&prompt, url, target_w, target_h).await
            }
            _ => {
                if current > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
                let url = build_pollinations_url(&prompt, target_w, target_h);
                download_image(&url).await
            }
        };

        let mut sheet = fetch_result
            .and_then(|b| decode_sprite_sheet(&b, target_w, target_h))
            .map_err(|e| format!("生成「{}」动画失败: {}", spec.state, e))?;

        apply_chroma_key(&mut sheet, 30);
        save_sprite_sheet_png(&pets_dir, &pet_id, &spec.state, &sheet)?;

        let delay_ms: u32 = match spec.state.as_str() {
            "walking" | "waving" => 150,
            _ => 200,
        };

        states.insert(spec.state.clone(), SpriteStateInfo {
            cols: spec.cols,
            rows: spec.rows,
            frame_count: spec.frame_count,
            frame_w: 128,
            frame_h: 128,
            delay_ms,
        });

        current += 1;
        let _ = app.emit("generation-progress", serde_json::json!({
            "current": current,
            "total": TOTAL_STATES as u32,
        }));
    }

    Ok(states)
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let comma_pos = data_url.find(',').ok_or_else(|| "invalid data URL: missing comma".to_string())?;
    let base64_str = &data_url[comma_pos + 1..];
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| format!("base64 decode error: {e}"))
}

#[tauri::command]
pub async fn save_custom_frames(
    app: tauri::AppHandle,
    pet_id: String,
    idle: String,
    walking: String,
    waving: String,
    working: String,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    use tauri::Manager;

    let pets_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("pets");

    let entries = [
        ("idle",    &idle),
        ("walking", &walking),
        ("waving",  &waving),
        ("working", &working),
    ];

    let mut states: HashMap<String, SpriteStateInfo> = HashMap::new();

    for (state, data_url) in &entries {
        let bytes = decode_data_url(data_url)?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let resized = imageops::resize(&img.to_rgba8(), 128, 128, imageops::FilterType::Lanczos3);
        save_sprite_sheet_png(&pets_dir, pet_id.as_str(), state, &resized)?;
        states.insert(state.to_string(), SpriteStateInfo {
            cols: 1,
            rows: 1,
            frame_count: 1,
            frame_w: 128,
            frame_h: 128,
            delay_ms: 200,
        });
    }

    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn build_state_specs_counts() {
        let specs = build_state_specs();
        let map: std::collections::HashMap<_, _> =
            specs.iter().map(|s| (s.state.as_str(), s.frame_count)).collect();
        assert_eq!(map["idle"],    4);
        assert_eq!(map["walking"], 6);
        assert_eq!(map["waving"],  4);
        assert_eq!(map["working"], 4);
    }

    #[test]
    fn state_spec_grid_layout() {
        let specs = build_state_specs();
        for spec in &specs {
            assert_eq!(spec.cols, 2);
            assert_eq!(spec.rows, (spec.frame_count + 1) / 2);
            assert_eq!(spec.img_width(),  spec.cols as u32 * CELL_SIZE);
            assert_eq!(spec.img_height(), spec.rows as u32 * CELL_SIZE);
        }
    }

    #[test]
    fn build_pollinations_url_encodes_spaces() {
        let url = build_pollinations_url("anime chibi girl", 256, 256);
        assert!(url.starts_with("https://image.pollinations.ai/prompt/"));
        assert!(url.contains("anime%20chibi%20girl") || url.contains("anime+chibi+girl"));
        assert!(url.contains("width=256"));
        assert!(url.contains("height=256"));
    }

    #[test]
    fn apply_chroma_key_removes_white_pixels() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        img.put_pixel(1, 0, Rgba([100, 50, 200, 255]));
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(1, 0)[3], 255);
    }

    #[test]
    fn apply_chroma_key_keeps_near_white_below_threshold() {
        let mut img = RgbaImage::new(1, 1);
        img.put_pixel(0, 0, Rgba([200, 200, 200, 255]));
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn save_sprite_sheet_png_writes_correct_dimensions() {
        let dir = tempfile::TempDir::new().unwrap();
        let sheet = RgbaImage::new(256, 256);
        save_sprite_sheet_png(&dir.path().to_path_buf(), "pet-1", "idle", &sheet).unwrap();
        let path = dir.path().join("pet-1").join("idle.png");
        assert!(path.exists());
        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 256);
        assert_eq!(loaded.height(), 256);
    }
}
```

- [ ] **Step 2: Fix pet.rs test helper — update `make_pet`**

Open `src-tauri/src/commands/pet.rs`. Find the `make_pet` function inside `#[cfg(test)]` and replace it:

```rust
fn make_pet(id: &str) -> Pet {
    use crate::models::SpriteStateInfo;
    let mut states = std::collections::HashMap::new();
    for s in &["idle", "walking", "waving", "working"] {
        states.insert(s.to_string(), SpriteStateInfo {
            cols: 2, rows: 2, frame_count: 4, frame_w: 128, frame_h: 128, delay_ms: 200,
        });
    }
    Pet {
        id: id.to_string(),
        name: "Test Pet".to_string(),
        states,
        created_at: "2026-08-03T10:00:00Z".to_string(),
        prompt: "anime chibi".to_string(),
    }
}
```

- [ ] **Step 3: Run Rust tests**

```bash
cargo test -p desktop-pet --lib 2>&1
```

Expected: all tests pass. Pay attention to `save_sprite_sheet_png_writes_correct_dimensions` — it should PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/generate.rs src-tauri/src/commands/pet.rs
git commit -m "refactor: remove GIF pipeline; save sprite sheet PNGs with metadata"
```

---

## Task 3 — Cargo.toml: remove gif dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Remove the `gif` line**

In `src-tauri/Cargo.toml`, find and delete this line:

```toml
gif = "0.13"
```

- [ ] **Step 2: Verify it builds**

```bash
cargo build -p desktop-pet 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: remove gif crate dependency"
```

---

## Task 4 — TypeScript types: SpriteStateInfo + updated Pet

**Files:**
- Modify: `src/types/pet.ts`
- Modify: `src/types/__tests__/pet.test.ts`

- [ ] **Step 1: Rewrite `src/types/pet.ts`**

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

- [ ] **Step 2: Rewrite `src/types/__tests__/pet.test.ts`**

```ts
import { describe, it, expect } from 'vitest';
import { PET_STATES } from '../pet';
import type { Pet, SpriteStateInfo } from '../pet';

const META: SpriteStateInfo = {
  cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200,
};

describe('Pet types', () => {
  it('PET_STATES has all four states', () => {
    expect(PET_STATES).toContain('idle');
    expect(PET_STATES).toContain('walking');
    expect(PET_STATES).toContain('waving');
    expect(PET_STATES).toContain('working');
    expect(PET_STATES).toHaveLength(4);
  });

  it('Pet object matches expected shape', () => {
    const pet: Pet = {
      id: 'abc',
      name: 'Test',
      states: { idle: META, walking: META, waving: META, working: META },
      createdAt: '2026-08-03T10:00:00Z',
      prompt: 'chibi',
    };
    expect(pet.id).toBe('abc');
    expect(pet.states.idle.frameCount).toBe(4);
    expect(pet.states.walking.delayMs).toBe(200);
  });
});
```

- [ ] **Step 3: Run tests**

```bash
npx vitest run src/types 2>&1
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/types/pet.ts src/types/__tests__/pet.test.ts
git commit -m "refactor: replace PetFrames with SpriteStateInfo in TypeScript types"
```

---

## Task 5 — WizardData: add petStates field

**Files:**
- Modify: `src/windows/Creator/steps/types.ts`

- [ ] **Step 1: Update WizardData**

```ts
import type { PetState, SpriteStateInfo } from '../../../types/pet';

export interface WizardData {
  photoDataUrl: string | null;
  prompt: string;
  apiKey: string;
  petId: string | null;
  petName: string;
  petStates: Record<PetState, SpriteStateInfo> | null;
}

export const INITIAL_WIZARD_DATA: WizardData = {
  photoDataUrl: null,
  prompt: '',
  apiKey: '',
  petId: null,
  petName: '',
  petStates: null,
};
```

- [ ] **Step 2: Run TypeScript check**

```bash
npx tsc --noEmit 2>&1 | head -30
```

Expected: errors in `Creator/index.tsx`, `GenerateStep.tsx`, `DirectUploadStep.tsx`, `SaveStep.tsx` — these are placeholders we'll fix in later tasks.

- [ ] **Step 3: Commit**

```bash
git add src/windows/Creator/steps/types.ts
git commit -m "refactor: add petStates to WizardData"
```

---

## Task 6 — SpriteAnimator: new Canvas component + tests

**Files:**
- Create: `src/windows/Pet/SpriteAnimator.tsx`
- Create: `src/windows/Pet/__tests__/SpriteAnimator.test.tsx`

- [ ] **Step 1: Write failing test first**

Create `src/windows/Pet/__tests__/SpriteAnimator.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeAll, afterEach } from 'vitest';
import { render } from '@testing-library/react';
import SpriteAnimator from '../SpriteAnimator';
import type { SpriteStateInfo } from '../../../types/pet';

const mockCtx = {
  clearRect: vi.fn(),
  drawImage: vi.fn(),
};

beforeAll(() => {
  HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue(mockCtx);
  vi.stubGlobal('requestAnimationFrame', vi.fn());
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
});

afterEach(() => {
  vi.clearAllMocks();
});

const META: SpriteStateInfo = {
  cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200,
};

describe('SpriteAnimator', () => {
  it('renders a canvas element', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} />
    );
    expect(container.querySelector('canvas')).not.toBeNull();
  });

  it('uses displayW/displayH for canvas dimensions', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} displayW={200} displayH={200} />
    );
    const canvas = container.querySelector('canvas')!;
    expect(canvas.width).toBe(200);
    expect(canvas.height).toBe(200);
  });

  it('defaults canvas size to frameW/frameH when display props omitted', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} />
    );
    const canvas = container.querySelector('canvas')!;
    expect(canvas.width).toBe(128);
    expect(canvas.height).toBe(128);
  });

  it('applies pixelated image rendering', () => {
    const { container } = render(
      <SpriteAnimator sheetSrc="/test.png" meta={META} />
    );
    const canvas = container.querySelector('canvas')!;
    expect(canvas.style.imageRendering).toBe('pixelated');
  });
});
```

- [ ] **Step 2: Run test (expect failure: SpriteAnimator not found)**

```bash
npx vitest run src/windows/Pet/__tests__/SpriteAnimator.test.tsx 2>&1
```

Expected: FAIL — cannot find module `../SpriteAnimator`.

- [ ] **Step 3: Create `src/windows/Pet/SpriteAnimator.tsx`**

```tsx
import { useEffect, useRef } from 'react';
import type { SpriteStateInfo } from '../../types/pet';

interface SpriteAnimatorProps {
  sheetSrc: string;
  meta: SpriteStateInfo;
  displayW?: number;
  displayH?: number;
}

export default function SpriteAnimator({ sheetSrc, meta, displayW, displayH }: SpriteAnimatorProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const frameRef = useRef(0);
  const lastTsRef = useRef(0);
  const rafRef = useRef(0);

  const w = displayW ?? meta.frameW;
  const h = displayH ?? meta.frameH;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    frameRef.current = 0;
    lastTsRef.current = 0;
    cancelAnimationFrame(rafRef.current);

    const img = new Image();

    img.onload = () => {
      function tick(ts: number) {
        if (ts - lastTsRef.current >= meta.delayMs) {
          const col = frameRef.current % meta.cols;
          const row = Math.floor(frameRef.current / meta.cols);
          ctx.clearRect(0, 0, w, h);
          ctx.drawImage(img, col * meta.frameW, row * meta.frameH, meta.frameW, meta.frameH, 0, 0, w, h);
          frameRef.current = (frameRef.current + 1) % meta.frameCount;
          lastTsRef.current = ts;
        }
        rafRef.current = requestAnimationFrame(tick);
      }
      rafRef.current = requestAnimationFrame(tick);
    };

    img.src = sheetSrc;

    return () => {
      cancelAnimationFrame(rafRef.current);
      img.onload = null;
    };
  }, [sheetSrc, meta, w, h]);

  return (
    <canvas
      ref={canvasRef}
      width={w}
      height={h}
      style={{ imageRendering: 'pixelated', display: 'block' }}
    />
  );
}
```

- [ ] **Step 4: Run tests**

```bash
npx vitest run src/windows/Pet/__tests__/SpriteAnimator.test.tsx 2>&1
```

Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/windows/Pet/SpriteAnimator.tsx src/windows/Pet/__tests__/SpriteAnimator.test.tsx
git commit -m "feat: SpriteAnimator Canvas component with requestAnimationFrame loop"
```

---

## Task 7 — GenerateStep: capture states from backend, update onNext

**Files:**
- Modify: `src/windows/Creator/steps/GenerateStep.tsx`

- [ ] **Step 1: Update the file**

Replace `src/windows/Creator/steps/GenerateStep.tsx` with:

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { loadSettings, saveSettings, SILICONFLOW_MODELS, type ImageProvider } from '../../../lib/settings';
import type { PetState, SpriteStateInfo } from '../../../types/pet';

interface GenerateStepProps {
  prompt: string;
  onNext: (petId: string, states: Record<PetState, SpriteStateInfo>) => void;
  onBack: () => void;
}

type Status = 'idle' | 'generating' | 'done' | 'error';

const TOTAL_STATES = 4;

const IMAGE_OPTIONS: { value: ImageProvider; label: string; desc: string }[] = [
  { value: 'pollinations', label: 'Pollinations.ai（免费）', desc: '无需 API Key，Flux 模型' },
  { value: 'siliconflow',  label: '硅基流动',               desc: '有免费额度，siliconflow.cn' },
  { value: 'localsd',      label: '本地 Stable Diffusion',  desc: 'AUTOMATIC1111 WebUI' },
];

export default function GenerateStep({ prompt, onNext, onBack }: GenerateStepProps) {
  const [status, setStatus] = useState<Status>('idle');
  const [progress, setProgress] = useState({ current: 0, total: TOTAL_STATES });
  const [petId, setPetId] = useState<string | null>(null);
  const [statesResult, setStatesResult] = useState<Record<PetState, SpriteStateInfo> | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [settings, setSettings] = useState(loadSettings);

  function updateSettings(patch: Partial<ReturnType<typeof loadSettings>>) {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      saveSettings(next);
      return next;
    });
  }

  async function handleGenerate() {
    const id = crypto.randomUUID();
    setPetId(id);
    setStatesResult(null);
    setStatus('generating');
    setProgress({ current: 0, total: TOTAL_STATES });

    const unlisten = await listen<{ current: number; total: number }>(
      'generation-progress',
      (event) => setProgress(event.payload)
    );

    try {
      const states = await invoke<Record<PetState, SpriteStateInfo>>('generate_and_assemble', {
        petId: id,
        basePrompt: prompt,
        imageProvider: settings.imageProvider,
        imageApiKey: settings.imageApiKey || null,
        imageModel: settings.imageModel || null,
        localSdUrl: settings.localSdUrl || null,
      });
      setStatesResult(states);
      setStatus('done');
    } catch (err) {
      setErrorMsg((err as Error).message ?? String(err));
      setStatus('error');
    } finally {
      unlisten();
    }
  }

  const pct = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;
  const generating = status === 'generating';
  const showConfig = status === 'idle' || status === 'error';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      {showConfig && (
        <div style={{ background: '#f7fafc', borderRadius: 10, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 10 }}>
          <p style={{ margin: 0, fontSize: 12, color: '#718096', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            图像生成服务
          </p>
          {IMAGE_OPTIONS.map(({ value, label, desc }) => (
            <label key={value} style={{ display: 'flex', alignItems: 'flex-start', gap: 8, cursor: 'pointer' }}>
              <input
                type="radio"
                name="imageProvider"
                value={value}
                checked={settings.imageProvider === value}
                onChange={() => updateSettings({ imageProvider: value })}
                style={{ marginTop: 3, accentColor: '#4f8ef7' }}
              />
              <div>
                <div style={{ fontSize: 13, fontWeight: 500, color: '#2d3748' }}>{label}</div>
                <div style={{ fontSize: 11, color: '#a0aec0' }}>{desc}</div>
              </div>
            </label>
          ))}

          {settings.imageProvider === 'siliconflow' && (
            <>
              <input
                type="password"
                value={settings.imageApiKey}
                onChange={(e) => updateSettings({ imageApiKey: e.target.value })}
                placeholder="粘贴硅基流动 API Key…"
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
              />
              <select
                value={settings.imageModel}
                onChange={(e) => updateSettings({ imageModel: e.target.value })}
                style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, background: '#fff', cursor: 'pointer' }}
              >
                {SILICONFLOW_MODELS.map(({ value, label }) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </>
          )}

          {settings.imageProvider === 'localsd' && (
            <input
              type="text"
              value={settings.localSdUrl}
              onChange={(e) => updateSettings({ localSdUrl: e.target.value })}
              placeholder="http://localhost:7860"
              style={{ padding: '6px 10px', borderRadius: 6, border: '1px solid #e2e8f0', fontSize: 13, boxSizing: 'border-box' }}
            />
          )}
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center', padding: '16px 0' }}>
        {status === 'idle' && (
          <>
            <div style={{ fontSize: 48 }}>✨</div>
            <p style={{ color: '#718096', textAlign: 'center', margin: 0 }}>
              将生成 4 个动画状态，每个状态整体生成一张精灵图再保存为 PNG。
              <br />
              <span style={{ fontSize: 13, color: '#a0aec0' }}>预计需要 1～3 分钟。</span>
            </p>
          </>
        )}

        {status === 'generating' && (
          <>
            <div style={{ fontSize: 48 }}>⏳</div>
            <p style={{ color: '#4a5568', margin: 0 }}>
              正在生成第 {progress.current} / {progress.total} 个动画状态…
            </p>
            <div style={{ width: '100%', maxWidth: 360, background: '#e2e8f0', borderRadius: 6, overflow: 'hidden', height: 8 }}>
              <div style={{ width: `${pct}%`, height: '100%', background: '#4f8ef7', transition: 'width 0.3s ease' }} />
            </div>
            <p style={{ color: '#a0aec0', fontSize: 13, margin: 0 }}>{pct}%</p>
          </>
        )}

        {status === 'done' && (
          <>
            <div style={{ fontSize: 48 }}>🎉</div>
            <p style={{ color: '#38a169', margin: 0 }}>全部动画已生成！</p>
          </>
        )}

        {status === 'error' && (
          <>
            <div style={{ fontSize: 48 }}>⚠️</div>
            <p style={{ color: '#e53e3e', margin: 0 }}>{errorMsg}</p>
          </>
        )}
      </div>

      <div style={{ display: 'flex', gap: 12, justifyContent: 'flex-end' }}>
        <button
          onClick={onBack}
          disabled={generating}
          style={{
            padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0',
            background: '#fff', color: '#4a5568',
            cursor: generating ? 'not-allowed' : 'pointer',
          }}
        >
          上一步
        </button>

        {status === 'done' ? (
          <button
            onClick={() => petId && statesResult && onNext(petId, statesResult)}
            style={{ padding: '8px 24px', borderRadius: 6, border: 'none', background: '#4f8ef7', color: '#fff', cursor: 'pointer' }}
          >
            下一步
          </button>
        ) : (
          <button
            onClick={handleGenerate}
            disabled={generating}
            style={{
              padding: '8px 24px', borderRadius: 6, border: 'none',
              background: generating ? '#e2e8f0' : '#4f8ef7',
              color: '#fff', cursor: generating ? 'not-allowed' : 'pointer',
            }}
          >
            {status === 'error' ? '重试' : '开始生成'}
          </button>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Run TypeScript check**

```bash
npx tsc --noEmit 2>&1 | grep GenerateStep
```

Expected: no errors for GenerateStep (errors may exist in other files not yet updated).

- [ ] **Step 3: Commit**

```bash
git add src/windows/Creator/steps/GenerateStep.tsx
git commit -m "refactor: GenerateStep captures sprite states from backend and passes to onNext"
```

---

## Task 8 — DirectUploadStep: capture states, update onNext

**Files:**
- Modify: `src/windows/Creator/steps/DirectUploadStep.tsx`

- [ ] **Step 1: Update `src/windows/Creator/steps/DirectUploadStep.tsx`**

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { PetState, SpriteStateInfo } from '../../../types/pet';

const STATES: PetState[] = ['idle', 'walking', 'waving', 'working'];
const STATE_LABELS: Record<PetState, string> = {
  idle: '待机', walking: '行走', waving: '招手', working: '工作',
};
const STATE_HINTS: Record<PetState, string> = {
  idle: '静止站立', walking: '移动 / 行走',
  waving: '打招呼 / 挥手', working: '专注 / 打字',
};

interface DirectUploadStepProps {
  onNext: (petId: string, states: Record<PetState, SpriteStateInfo>) => void;
  onBack: () => void;
}

export default function DirectUploadStep({ onNext, onBack }: DirectUploadStepProps) {
  const [files, setFiles] = useState<Partial<Record<PetState, string>>>({});
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const allFilled = STATES.every((s) => files[s]);

  function handleFile(state: PetState, file: File) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const dataUrl = (e.target as FileReader).result as string;
      setFiles((prev) => ({ ...prev, [state]: dataUrl }));
    };
    reader.readAsDataURL(file);
  }

  async function handleNext() {
    setSaving(true);
    setError(null);
    try {
      const petId = crypto.randomUUID();
      const states = await invoke<Record<PetState, SpriteStateInfo>>('save_custom_frames', {
        petId,
        idle: files.idle!,
        walking: files.walking!,
        waving: files.waving!,
        working: files.working!,
      });
      onNext(petId, states);
    } catch (err) {
      setError((err as Error).message ?? String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <p style={{ color: '#718096', margin: 0, fontSize: 14 }}>
        每个动画状态上传一张图片或 GIF。支持 .gif .png .jpg .webp
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
        {STATES.map((state) => (
          <label
            key={state}
            style={{
              display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
              border: `2px dashed ${files[state] ? '#4f8ef7' : '#e2e8f0'}`,
              borderRadius: 10, padding: 16, cursor: 'pointer',
              background: files[state] ? '#f0f5ff' : '#fafafa',
              transition: 'border-color 0.2s, background 0.2s',
            }}
          >
            <input
              type="file"
              accept=".gif,.png,.jpg,.jpeg,.webp"
              style={{ display: 'none' }}
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) handleFile(state, file);
              }}
            />
            {files[state] ? (
              <img
                src={files[state]}
                alt={state}
                style={{ width: 80, height: 80, objectFit: 'contain', imageRendering: 'pixelated' }}
              />
            ) : (
              <div style={{
                width: 80, height: 80, display: 'flex', alignItems: 'center',
                justifyContent: 'center', fontSize: 32, color: '#cbd5e0',
              }}>
                +
              </div>
            )}
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontWeight: 600, fontSize: 13, color: '#2d3748' }}>{STATE_LABELS[state]}</div>
              <div style={{ fontSize: 11, color: '#a0aec0' }}>{STATE_HINTS[state]}</div>
            </div>
          </label>
        ))}
      </div>

      {error && <p style={{ color: '#e53e3e', fontSize: 13, margin: 0 }}>{error}</p>}

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
        <button
          onClick={onBack}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: 'pointer' }}
        >
          返回首页
        </button>
        <button
          onClick={handleNext}
          disabled={!allFilled || saving}
          style={{
            padding: '8px 24px', borderRadius: 6, border: 'none',
            background: allFilled && !saving ? '#4f8ef7' : '#e2e8f0',
            color: '#fff', cursor: allFilled && !saving ? 'pointer' : 'not-allowed',
          }}
        >
          {saving ? '保存中…' : '下一步'}
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/windows/Creator/steps/DirectUploadStep.tsx
git commit -m "refactor: DirectUploadStep captures sprite states from save_custom_frames"
```

---

## Task 9 — SaveStep: accept states prop, remove GIF path construction

**Files:**
- Modify: `src/windows/Creator/steps/SaveStep.tsx`

- [ ] **Step 1: Rewrite `src/windows/Creator/steps/SaveStep.tsx`**

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Pet, PetState, SpriteStateInfo } from '../../../types/pet';

interface SaveStepProps {
  petId: string;
  prompt: string;
  states: Record<PetState, SpriteStateInfo>;
  onComplete: (pet: Pet) => void;
  onBack: () => void;
}

export default function SaveStep({ petId, prompt, states, onComplete, onBack }: SaveStepProps) {
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSave() {
    if (!name.trim()) return;
    setSaving(true);
    setError(null);
    try {
      const pet: Pet = {
        id: petId,
        name: name.trim(),
        states,
        createdAt: new Date().toISOString(),
        prompt,
      };
      await invoke('save_pet', { pet });
      onComplete(pet);
    } catch (err) {
      setError((err as Error).message ?? String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24, alignItems: 'center', padding: '32px 0' }}>
      <div style={{ fontSize: 48 }}>💾</div>
      <p style={{ color: '#718096', margin: 0 }}>给你的宠物起个名字吧。</p>

      <input
        type="text"
        placeholder="宠物名称…"
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && handleSave()}
        maxLength={32}
        style={{
          padding: '10px 16px', borderRadius: 8, border: '1px solid #e2e8f0',
          fontSize: 16, width: 240, textAlign: 'center',
        }}
      />

      {error && <p style={{ color: '#e53e3e', fontSize: 13, margin: 0 }}>{error}</p>}

      <div style={{ display: 'flex', gap: 12 }}>
        <button
          onClick={onBack}
          disabled={saving}
          style={{ padding: '8px 20px', borderRadius: 6, border: '1px solid #e2e8f0', background: '#fff', color: '#4a5568', cursor: saving ? 'not-allowed' : 'pointer' }}
        >
          上一步
        </button>
        <button
          onClick={handleSave}
          disabled={!name.trim() || saving}
          style={{
            padding: '8px 24px', borderRadius: 6, border: 'none',
            background: name.trim() && !saving ? '#4f8ef7' : '#e2e8f0',
            color: '#fff', cursor: name.trim() && !saving ? 'pointer' : 'not-allowed',
          }}
        >
          {saving ? '保存中…' : '保存宠物'}
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/windows/Creator/steps/SaveStep.tsx
git commit -m "refactor: SaveStep uses states prop instead of constructing GIF paths"
```

---

## Task 10 — Creator/index.tsx: wire up updated data flow

**Files:**
- Modify: `src/windows/Creator/index.tsx`

- [ ] **Step 1: Update the GenerateStep, DirectUploadStep, and SaveStep calls**

In `src/windows/Creator/index.tsx`, find these three JSX blocks and replace them:

**Find** (GenerateStep block):
```tsx
          {step === 'generate' && (
            <GenerateStep
              prompt={data.prompt}
              onNext={(petId) => { updateData({ petId }); setStep('preview'); }}
              onBack={() => setStep('analyze')}
            />
          )}
```

**Replace with:**
```tsx
          {step === 'generate' && (
            <GenerateStep
              prompt={data.prompt}
              onNext={(petId, states) => { updateData({ petId, petStates: states }); setStep('preview'); }}
              onBack={() => setStep('analyze')}
            />
          )}
```

**Find** (DirectUploadStep block):
```tsx
          {step === 'direct-upload' && (
            <DirectUploadStep
              onNext={(petId) => { updateData({ petId }); setStep('preview'); }}
              onBack={() => setMode('choose')}
            />
          )}
```

**Replace with:**
```tsx
          {step === 'direct-upload' && (
            <DirectUploadStep
              onNext={(petId, states) => { updateData({ petId, petStates: states }); setStep('preview'); }}
              onBack={() => setMode('choose')}
            />
          )}
```

**Find** (SaveStep block):
```tsx
          {step === 'save' && data.petId && (
            <SaveStep
              petId={data.petId}
              prompt={data.prompt}
              onComplete={(pet) => setSavedPet(pet)}
              onBack={() => setStep('preview')}
            />
          )}
```

**Replace with:**
```tsx
          {step === 'save' && data.petId && data.petStates && (
            <SaveStep
              petId={data.petId}
              prompt={data.prompt}
              states={data.petStates}
              onComplete={(pet) => setSavedPet(pet)}
              onBack={() => setStep('preview')}
            />
          )}
```

- [ ] **Step 2: Run TypeScript check**

```bash
npx tsc --noEmit 2>&1 | grep -v "node_modules" | head -20
```

Expected: errors only in `PreviewStep.tsx` and `Pet/index.tsx` (not yet updated). Creator/index.tsx should be clean.

- [ ] **Step 3: Commit**

```bash
git add src/windows/Creator/index.tsx
git commit -m "refactor: wire petStates through Creator wizard data flow"
```

---

## Task 11 — PreviewStep: switch from .gif to .png

**Files:**
- Modify: `src/windows/Creator/steps/PreviewStep.tsx`

- [ ] **Step 1: Change `.gif` to `.png`**

In `src/windows/Creator/steps/PreviewStep.tsx`, find:

```ts
        const absPath = await join(appDir, 'pets', petId, `${state}.gif`);
```

Replace with:

```ts
        const absPath = await join(appDir, 'pets', petId, `${state}.png`);
```

Also rename the state variable from `gifSrcs` to `sheetSrcs` for clarity (both `useState` declaration and all usages):

```ts
  const [sheetSrcs, setSheetSrcs] = useState<Record<string, string>>({});
  // ...
  setSheetSrcs(srcs);
  // ...
  {sheetSrcs[state] ? (
    <img
      src={sheetSrcs[state]}
      // ...
```

- [ ] **Step 2: Commit**

```bash
git add src/windows/Creator/steps/PreviewStep.tsx
git commit -m "refactor: PreviewStep loads .png sprite sheets instead of .gif"
```

---

## Task 12 — Pet/index.tsx: replace img stack with SpriteAnimator

**Files:**
- Modify: `src/windows/Pet/index.tsx`

- [ ] **Step 1: Update imports and add petsDir state**

At the top of `src/windows/Pet/index.tsx`, add these imports:

```tsx
import { appDataDir, join } from '@tauri-apps/api/path';
import SpriteAnimator from './SpriteAnimator';
```

Remove this import if present:
```tsx
import { convertFileSrc } from '@tauri-apps/api/core';
```
(Keep `invoke` from `@tauri-apps/api/core` — only remove `convertFileSrc` if it's no longer used for anything else.)

Actually, keep `convertFileSrc` — it's used for the sprite sheet src. We still need it.

- [ ] **Step 2: Add petsDir state**

Inside `PetWindow()`, add:

```tsx
const [petsDir, setPetsDir] = useState<string | null>(null);
```

- [ ] **Step 3: Load petsDir in the init effect**

Inside the `async function init()` in the `useEffect`, add after `await loadPets()`:

```ts
      const appDir = await appDataDir();
      setPetsDir(await join(appDir, 'pets'));
```

- [ ] **Step 4: Replace the img stack with SpriteAnimator**

Find this block in the JSX:
```tsx
      {activePet && (
        <div style={{ position: 'relative', width: 200, height: 240 }}>
          {PET_STATES.map((state) => (
            <img
              key={state}
              src={convertFileSrc(activePet.frames[state])}
              width={200}
              height={240}
              alt="pet"
              draggable={false}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                display: 'block',
                visibility: state === petState ? 'visible' : 'hidden',
                imageRendering: 'pixelated',
                userSelect: 'none',
              }}
            />
          ))}
        </div>
      )}
```

Replace with:
```tsx
      {activePet && petsDir && (
        <SpriteAnimator
          sheetSrc={convertFileSrc(`${petsDir}/${activePet.id}/${petState}.png`)}
          meta={activePet.states[petState]}
          displayW={200}
          displayH={200}
        />
      )}
```

- [ ] **Step 5: Remove PET_STATES import if no longer used**

Check whether `PET_STATES` is still referenced anywhere in `Pet/index.tsx`. If not, remove it from the import:

```tsx
import { PET_STATES, type Pet, type PetState } from '../../types/pet';
// becomes:
import { type Pet, type PetState } from '../../types/pet';
```

- [ ] **Step 6: Run TypeScript check**

```bash
npx tsc --noEmit 2>&1 | grep -v "node_modules"
```

Expected: zero errors.

- [ ] **Step 7: Run all frontend tests**

```bash
npx vitest run 2>&1
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/windows/Pet/index.tsx
git commit -m "refactor: Pet window uses SpriteAnimator Canvas instead of stacked GIF img tags"
```

---

## Task 13 — Verify store test + final Rust build

**Files:**
- Check: `src/store/__tests__/petStore.test.ts`

- [ ] **Step 1: Check if petStore test uses `frames`**

```bash
grep -n "frames" src/store/__tests__/petStore.test.ts
```

If any matches reference `frames: { idle: ..., walking: ... }`, update those Pet fixtures to use `states` instead:

```ts
// Old fixture pattern:
const pet: Pet = { id: '1', name: 'Test', frames: { idle: 'i.gif', walking: 'w.gif', waving: 'wv.gif', working: 'wk.gif' }, createdAt: '...', prompt: '...' };

// New fixture pattern:
const META = { cols: 2, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200 };
const pet: Pet = { id: '1', name: 'Test', states: { idle: META, walking: META, waving: META, working: META }, createdAt: '...', prompt: '...' };
```

- [ ] **Step 2: Run all frontend tests**

```bash
npx vitest run 2>&1
```

Expected: all pass.

- [ ] **Step 3: Full Rust build and test**

```bash
cargo test -p desktop-pet --lib 2>&1
```

Expected: all pass. The removed tests (`assemble_gif_bytes_*`, `slice_sprite_sheet_*`) are gone, the new `save_sprite_sheet_png_writes_correct_dimensions` passes.

- [ ] **Step 4: TypeScript build**

```bash
npx tsc --noEmit 2>&1 | grep -v "node_modules"
```

Expected: zero errors.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "refactor: sprite sheet animation refactor complete — GIF pipeline removed"
```

---

## Self-Review Checklist

| Spec requirement | Task covering it |
|---|---|
| Remove GIF encoding | Task 2 (remove `assemble_gif_bytes`, `save_gif`) |
| Save PNG sprite sheets | Task 2 (`save_sprite_sheet_png`) |
| CELL_SIZE 256→128 | Task 2 (`const CELL_SIZE: u32 = 128`) |
| `SpriteStateInfo` Rust model | Task 1 |
| `Pet.states` replaces `Pet.frames` | Task 1 (Rust) + Task 4 (TS) |
| `generate_and_assemble` returns states map | Task 2 |
| `save_custom_frames` returns single-frame states map | Task 2 |
| Remove `gif` crate | Task 3 |
| `SpriteAnimator` Canvas component | Task 6 |
| `requestAnimationFrame` frame loop | Task 6 |
| Pet window uses SpriteAnimator | Task 12 |
| `WizardData.petStates` field | Task 5 |
| GenerateStep passes states to onNext | Task 7 |
| DirectUploadStep passes states to onNext | Task 8 |
| SaveStep uses states not GIF paths | Task 9 |
| Creator wizard data flow | Task 10 |
| PreviewStep uses .png | Task 11 |
| Manual upload = 1×1 sprite fallback | Task 2 (`save_custom_frames`) |
