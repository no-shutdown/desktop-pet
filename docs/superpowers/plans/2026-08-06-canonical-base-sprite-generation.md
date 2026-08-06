# Canonical Base Sprite Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the Creator and Tauri image pipeline into a canonical-base-first workflow that generates four 8-frame horizontal sprite rows through SiliconFlow or Local SD img2img, while preserving external sprite import, existing pet.json files, and legacy 128×128 animation playback.

**Architecture:** Separate pure sprite/prompt logic, provider request adapters, and temporary generation-run storage from Tauri command orchestration. The Creator will use two explicit generation stages—Base preview/confirmation and per-state row generation—then hand the assembled run preview to the existing frame picker and final save path. A generation run lives under app_data_dir/runs/<run_id> until the user saves; pets/<pet_id> is written only by the final save command.

**Tech Stack:** Rust 2021, Tauri 2 commands, reqwest, serde_json, image, base64; React 19, TypeScript, Vitest, Testing Library, Tauri invoke/listen APIs.

---

## Scope and execution safety

This plan implements the approved first phase only:

- Four states remain the product contract: idle, walking, waving, and working.
- Each generated row contains exactly eight horizontal 128×128 frames.
- SiliconFlow is the cloud path; Local SD uses AUTOMATIC1111 txt2img for Base and img2img for rows.
- Pollinations is not adapted for scheme three. Existing saved settings containing Pollinations are migrated to SiliconFlow before use.
- The nine-state hatch-pet catalog and 192×208 atlas are reserved for a later version and are not introduced here.
- External combined sprite sheets remain supported; the existing 4×2 grid picker continues to accept them and the import path flattens selected cells into horizontal state sheets.

The checkout currently contains user-owned uncommitted changes. During execution, inspect git diff before every edit, stage only the exact files named by the current task, and never use git add -A, git reset --hard, or checkout commands. The pre-existing changes in src-tauri/src/commands/generate.rs, src-tauri/src/lib.rs, src/windows/Creator/index.tsx, src/windows/Creator/steps/GenerateStep.tsx, src/windows/Creator/steps/types.ts, src/windows/Pet/SpriteAnimator.tsx, the deleted DirectUploadStep.tsx, and the untracked ManualFramePickerStep.tsx must be reviewed as part of the overlapping implementation rather than silently discarded.

## File map

Create the following focused Rust modules:

- Create src-tauri/src/commands/generation/mod.rs — Tauri command implementations and orchestration for Base, state rows, assembly, cleanup, and final frame selection save.
- Create src-tauri/src/commands/generation/types.rs — state catalog, provider configuration, run manifest, command result, and status types.
- Create src-tauri/src/commands/generation/prompts.rs — hatch-pet-inspired Base and Row prompt templates, state action requirements, and prompt truncation.
- Create src-tauri/src/commands/generation/providers.rs — SiliconFlow and Local SD request builders, response decoding, image download, and provider dispatch.
- Create src-tauri/src/commands/generation/run.rs — run-directory paths, manifest persistence, attempt/status transitions, and safe cleanup.
- Create src-tauri/src/commands/generation/sprite.rs — chroma-key selection/application, row normalization, assembly, data URLs, and deterministic image validation.

Modify the following Rust integration points and tests:

- Modify src-tauri/src/commands/mod.rs — register the new generation module while retaining generate as a compatibility re-export module.
- Modify src-tauri/src/commands/generate.rs — replace the monolithic implementation with compatibility re-exports for save_combined_sprite_sheet, save_frame_selections, and shared public helpers used by any existing code.
- Modify src-tauri/src/lib.rs — register the new four generation commands plus the preserved import/save commands.
- Modify src-tauri/src/models.rs — keep the current serialized Pet/SpriteStateInfo shape and add compatibility tests for old 128×128 metadata if needed.

Modify the following TypeScript/settings files:

- Modify src/lib/settings.ts — add separate SiliconFlow Base/reference models, Local SD denoising strength, and legacy Pollinations migration.
- Modify src/windows/Creator/SettingsPanel.tsx — expose only the two scheme-three providers, separate model selectors, and the Local SD denoising slider.
- Modify src/types/pet.ts — retain the four-state public type and centralize the state catalog used by preview/import code.
- Modify src/windows/Creator/steps/types.ts — add run/base/row state types and preserve the existing generated-config/import data contract.
- Modify src/windows/Creator/steps/GenerateStep.tsx — turn the existing all-in-one generator into the Base preview/confirm/retry step.
- Create src/windows/Creator/steps/StateGenerationStep.tsx — new per-state row generation, retry, progress, and assembly UI.
- Modify src/windows/Creator/index.tsx — add base-generate and state-generate steps and run cleanup transitions.
- Modify src/windows/Creator/steps/ManualFramePickerStep.tsx — support the new zero-gap horizontal-row preview while preserving external 4×2 grid import.
- Modify src/windows/Creator/steps/PreviewStep.tsx — consume the state catalog/metadata rather than duplicating state names.
- Modify src/windows/Pet/SpriteAnimator.tsx — verify and, only if required by tests, harden playback for both old multi-row sheets and new horizontal sheets.

Create or update tests beside each implementation:

- Rust unit tests in the new generation modules.
- Create src/lib/__tests__/settings.test.ts — provider defaults and legacy settings migration.
- Modify src/windows/Creator/steps/__tests__/GenerateStep.test.tsx — Base command, confirm, retry, and failure behavior.
- Create src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx — sequential rows, progress, row retry, and no fallback provider.
- Create src/windows/Creator/steps/__tests__/ManualFramePickerStep.test.tsx — horizontal generated input and external grid input.
- Modify src/windows/Creator/steps/__tests__/PreviewStep.test.tsx — catalog-driven four-state preview.
- Create src/windows/Pet/__tests__/SpriteAnimator.test.tsx — old cols=4, rows=2 metadata and new cols=8, rows=1 metadata.

## Task 1: Freeze the shared generation contract

**Files:**

- Create src-tauri/src/commands/generation/types.rs
- Modify src-tauri/src/commands/mod.rs
- Modify src/types/pet.ts
- Modify src/windows/Creator/steps/types.ts
- Test Rust unit tests in src-tauri/src/commands/generation/types.rs
- Test src/lib/__tests__/settings.test.ts

- [ ] **Step 1: Write failing Rust contract tests**

Add tests for the state catalog and manifest serialization before implementing the types:

~~~
#[test]
fn phase_one_catalog_has_four_states_and_fixed_timing() {
    assert_eq!(state_definitions().iter().map(|s| s.key).collect::<Vec<_>>(),
        vec!["idle", "walking", "waving", "working"]);
    assert_eq!(state_definition("walking").unwrap().delay_ms, 100);
    assert_eq!(state_definition("working").unwrap().delay_ms, 120);
}

#[test]
fn manifest_round_trip_keeps_retryable_statuses() {
    let manifest = GenerationRunManifest::new(
        "run-1".into(), "siliconflow".into(), 8, "#FF00FF".into(),
        "anime chibi girl".into(),
    );
    let json = serde_json::to_string(&manifest).unwrap();
    let decoded: GenerationRunManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.version, 1);
    assert_eq!(decoded.base.status, ArtifactStatus::Pending);
    assert_eq!(decoded.states["idle"].attempts, 0);
    assert_eq!(decoded.frame_w, 128);
    assert_eq!(decoded.frame_h, 128);
}
~~~

The test must initially fail because the module and types do not exist.

- [ ] **Step 2: Run the focused Rust test and verify the red state**

Run:

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::types -- --nocapture
~~~

Expected: FAIL during compilation with unresolved generation::types symbols.

- [ ] **Step 3: Implement the fixed phase-one types**

Define the following public contract in types.rs:

~~~
pub const FRAME_W: u32 = 128;
pub const FRAME_H: u32 = 128;
pub const API_FRAME_W: u32 = 256;
pub const API_FRAME_H: u32 = 256;
pub const DEFAULT_FRAME_COUNT: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub delay_ms: u32,
    pub action: &'static str,
    pub requirements: &'static str,
}

pub fn state_definitions() -> &'static [StateDefinition];
pub fn state_definition(key: &str) -> Option<&'static StateDefinition>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub status: ArtifactStatus,
    pub path: String,
    pub attempts: u32,
    pub error: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactStatus { Pending, Generating, Complete, Failed }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRunManifest {
    pub version: u32,
    pub run_id: String,
    pub base_prompt: String,
    pub provider: String,
    pub base: ArtifactRecord,
    pub states: std::collections::BTreeMap<String, ArtifactRecord>,
    pub frame_w: u32,
    pub frame_h: u32,
    pub frame_count: u32,
    pub chroma_key: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasePreviewResult {
    pub run_id: String,
    pub data_url: String,
    pub chroma_key: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateRowResult {
    pub run_id: String,
    pub state: String,
    pub data_url: String,
    pub frame_w: u32,
    pub frame_h: u32,
    pub frame_count: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssembleRunPreviewResult {
    pub run_id: String,
    pub data_url: String,
    pub frame_w: u32,
    pub frame_h: u32,
    pub frame_count: u32,
    pub row_gap: u32,
}
~~~

GenerationRunManifest::new must create four pending state records with paths rows/<state>.png, set version=1, frame_w=128, frame_h=128, and keep the prompt but never store the API key.

Update the TypeScript types to mirror the serialized result names exactly:

~~~
export type GenerationProvider = 'siliconflow' | 'localsd';
export type GenerationPhase = 'base' | 'state' | 'assemble';
export type GenerationStatus = 'pending' | 'generating' | 'complete' | 'failed';

export interface GenerationRunPreview {
  runId: string;
  dataUrl: string;
  frameW: number;
  frameH: number;
  frameCount: number;
  rowGap: number;
}

export interface GeneratedSpriteConfig {
  petId: string;
  runId: string;
  frameW: number;
  frameH: number;
  rowGap: number;
  layout: 'horizontalRows' | 'grid';
  idleFrames: number;
  walkingFrames: number;
  wavingFrames: number;
  workingFrames: number;
}
~~~

Keep PetState and PET_STATES as the four existing values, and extend WizardData with nullable generationRunId and baseDataUrl fields while retaining generatedDataUrl and generatedConfig for the existing picker/save boundary.

- [ ] **Step 4: Run the contract tests and TypeScript settings test**

Run:

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::types -- --nocapture
npx vitest run src/lib/__tests__/settings.test.ts
~~~

Expected: Rust contract tests PASS. The settings test may still fail until Task 5 completes; record that expected red test and do not weaken it.

- [ ] **Step 5: Commit the contract-only change**

~~~
git add src-tauri/src/commands/generation/types.rs src-tauri/src/commands/mod.rs src/types/pet.ts src/windows/Creator/steps/types.ts src/lib/__tests__/settings.test.ts
git commit -m "refactor: define canonical sprite generation contracts"
~~~

## Task 2: Implement hatch-pet-inspired prompts and sprite processing

**Files:**

- Create src-tauri/src/commands/generation/prompts.rs
- Create src-tauri/src/commands/generation/sprite.rs
- Test unit tests in both new modules

- [ ] **Step 1: Write failing prompt, chroma, and validation tests**

Add tests that pin the production contract rather than exact wording order:

~~~
#[test]
fn base_prompt_locks_identity_and_forbids_scene_elements() {
    let prompt = build_base_prompt("anime chibi girl, red sailor uniform", "#FF00FF", "magenta");
    assert!(prompt.contains("one centered, complete character"));
    assert!(prompt.contains("same face, proportions, markings, palette"));
    assert!(prompt.contains("#FF00FF"));
    assert!(prompt.contains("No extra characters, scenery, text, labels, logos, watermark"));
}

#[test]
fn row_prompt_contains_state_requirements_and_horizontal_layout() {
    let prompt = build_row_prompt("anime chibi girl", "#00FFFF", "cyan", state_definition("waving").unwrap());
    assert!(prompt.contains("attached canonical base image"));
    assert!(prompt.contains("exactly 8 full-body frames from left to right"));
    assert!(prompt.contains("raised paw, hand, wing, or limb"));
    assert!(prompt.contains("No wave marks, motion arcs, lines, sparkles"));
}

#[test]
fn chroma_key_chooses_the_farthest_candidate_and_zeroes_transparent_rgb() {
    let reference = solid_image([255, 0, 0, 255]);
    let key = choose_chroma_key(Some(&reference));
    assert_eq!(key.hex, "#00FFFF");

    let mut row = RgbaImage::from_pixel(2, 1, Rgba([255, 0, 255, 255]));
    row.put_pixel(1, 0, Rgba([20, 20, 20, 255]));
    apply_chroma_key(&mut row, &key, 35);
    assert_eq!(row.get_pixel(0, 0).0, [0, 0, 0, 0]);
    assert_eq!(row.get_pixel(1, 0).0[3], 255);
}

#[test]
fn row_validation_rejects_wrong_dimensions_and_empty_frames() {
    assert!(validate_sprite_row(&RgbaImage::new(1024, 128), 128, 128, 8).is_err());
    let empty = RgbaImage::from_pixel(1024, 128, Rgba([0, 0, 0, 0]));
    assert!(validate_sprite_row(&empty, 128, 128, 8).unwrap_err().contains("empty"));
}
~~~

- [ ] **Step 2: Run the focused tests and verify they fail**

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::prompts generation::sprite -- --nocapture
~~~

Expected: FAIL because the prompt/sprite modules and functions are not implemented.

- [ ] **Step 3: Implement the prompt layer**

Implement these exact public functions:

~~~
pub fn build_base_prompt(base_description: &str, chroma_hex: &str, chroma_name: &str) -> String;
pub fn build_row_prompt(base_description: &str, chroma_hex: &str, chroma_name: &str, state: &StateDefinition) -> String;
~~~

Use concise production prompts modeled on the hatch-pet structure: identity, style contract, sprite layout contract, state action, state-specific prohibitions, chroma-key contract, and a short negative clause. The Base prompt must request one centered full-body neutral pose. The Row prompt must request exactly eight equal-width horizontal slots, one complete pose per slot, stable scale/baseline, and explicit absence of text, UI, scenery, grid, shadow, glow, motion blur, detached effects, extra characters, cropped limbs, and chroma-key colors inside the pet. Truncate the user description on a UTF-8 boundary at 300 bytes before interpolation.

Use the four approved state definitions:

~~~
idle: "calm low-distraction resting loop, subtle breathing, tiny blink, slight head or body bob, nearly unchanged silhouette and planted baseline"
walking: "rightward walking cycle, alternating legs and opposite arm swing, clear directional cadence, stable body scale and baseline"
waving: "friendly greeting shown only through the raised paw, hand, wing, or limb"
working: "focused active-task processing, typing, thinking, scanning, or purposeful hand/paw motion; not literal foot-running"
~~~

Encode the corresponding state prohibitions in requirements so future states can be added by catalog entry rather than a command-level match.

- [ ] **Step 4: Implement chroma-key and deterministic sprite helpers**

In sprite.rs, define:

~~~
pub const CHROMA_KEY_CANDIDATES: &[ChromaKey] = &[
    ChromaKey::new("magenta", "#FF00FF", [255, 0, 255]),
    ChromaKey::new("cyan",    "#00FFFF", [0, 255, 255]),
    ChromaKey::new("yellow",  "#FFFF00", [255, 255, 0]),
    ChromaKey::new("blue",    "#0000FF", [0, 0, 255]),
    ChromaKey::new("orange",  "#FF7F00", [255, 127, 0]),
    ChromaKey::new("green",   "#00FF00", [0, 255, 0]),
];

pub fn choose_chroma_key(reference: Option<&RgbaImage>) -> ChromaKey;
pub fn apply_chroma_key(image: &mut RgbaImage, key: &ChromaKey, threshold: u8);
pub fn normalize_horizontal_row(bytes: &[u8], key: &ChromaKey) -> Result<RgbaImage, String>;
pub fn validate_sprite_row(image: &RgbaImage, frame_w: u32, frame_h: u32, frame_count: u32) -> Result<(), String>;
pub fn assemble_rows(rows: &[RgbaImage], frame_w: u32, frame_h: u32) -> Result<RgbaImage, String>;
pub fn image_to_data_url(image: &RgbaImage) -> Result<String, String>;
~~~

Sample reference pixels on a fixed grid, choose the candidate with the largest minimum squared RGB distance, and use magenta when no reference image is supplied. Remove pixels by distance to the selected target rather than dark/light background detection. Use a short linear alpha ramp for near-key edge pixels, set RGB to [0,0,0] whenever alpha becomes zero, and reject rows whose dimensions are not exactly frame_w * frame_count by frame_h or contain a fully transparent frame. Resize provider output to the phase-one dimensions before validation. assemble_rows must create a 1024×512 combined preview with four 1024×128 rows and no row gap.

- [ ] **Step 5: Run the prompt and sprite tests**

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::prompts generation::sprite -- --nocapture
~~~

Expected: PASS, including tests for all four state requirement strings, candidate fallback, alpha/RGB normalization, dimensions, empty frames, and row assembly.

- [ ] **Step 6: Commit the pure generation logic**

~~~
git add src-tauri/src/commands/generation/prompts.rs src-tauri/src/commands/generation/sprite.rs
git commit -m "feat: add canonical sprite prompts and validation"
~~~

## Task 3: Add provider adapters for SiliconFlow and Local SD img2img

**Files:**

- Create src-tauri/src/commands/generation/providers.rs
- Modify src-tauri/src/commands/generation/types.rs
- Test unit tests in providers.rs

- [ ] **Step 1: Write failing request-builder tests**

Test the JSON bodies without making network calls:

~~~
#[test]
fn siliconflow_base_request_has_no_reference_image() {
    let body = siliconflow_base_body("Tongyi-MAI/Z-Image-Turbo", "base prompt", 256, 256);
    assert_eq!(body["model"], "Tongyi-MAI/Z-Image-Turbo");
    assert!(body.get("image").is_none());
    assert_eq!(body["image_size"], "256x256");
}

#[test]
fn siliconflow_row_request_contains_canonical_base_image() {
    let body = siliconflow_row_body(
        "Qwen/Qwen-Image-Edit-2509", "row prompt", "data:image/png;base64,BASE", 2048, 256,
    );
    assert_eq!(body["model"], "Qwen/Qwen-Image-Edit-2509");
    assert_eq!(body["image"], "data:image/png;base64,BASE");
    assert_eq!(body["image_size"], "2048x256");
}

#[test]
fn local_sd_row_request_uses_init_images_and_bounded_denoising() {
    let body = local_sd_row_body("row prompt", "BASE", 2048, 256, 0.55);
    assert_eq!(body["init_images"][0], "BASE");
    assert_eq!(body["denoising_strength"], 0.55);
    assert_eq!(clamp_denoising_strength(0.1), 0.35);
    assert_eq!(clamp_denoising_strength(0.9), 0.75);
}
~~~

- [ ] **Step 2: Run provider tests and verify the red state**

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::providers -- --nocapture
~~~

Expected: FAIL because the request builders are not present.

- [ ] **Step 3: Implement provider configuration and payload builders**

Add:

~~~
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_model: String,
    pub reference_model: String,
    pub local_sd_url: String,
    pub denoising_strength: f32,
}

pub fn clamp_denoising_strength(value: f32) -> f32;
pub fn siliconflow_base_body(model: &str, prompt: &str, width: u32, height: u32) -> serde_json::Value;
pub fn siliconflow_row_body(model: &str, prompt: &str, image_data_url: &str, width: u32, height: u32) -> serde_json::Value;
pub fn local_sd_base_body(prompt: &str, width: u32, height: u32) -> serde_json::Value;
pub fn local_sd_row_body(prompt: &str, init_image: &str, width: u32, height: u32, denoising_strength: f32) -> serde_json::Value;
pub async fn generate_base(config: &ProviderConfig, prompt: &str) -> Result<Vec<u8>, String>;
pub async fn generate_row(config: &ProviderConfig, prompt: &str, base_data_url: &str) -> Result<Vec<u8>, String>;
~~~

SiliconFlow must call POST https://api.siliconflow.cn/v1/images/generations with bearer authentication. Base uses base_model and no image; Row uses reference_model and the canonical Base data URL in image. Decode either images[0].url followed by an HTTP download or images[0].b64_json. Local SD must call /sdapi/v1/txt2img for Base and /sdapi/v1/img2img for Row; Row must send init_images: [base_data_url], denoising_strength, width, and height. Reject missing API keys, unsupported provider names, non-success HTTP responses, missing image fields, invalid base64, and invalid denoising values after clamping. Do not add a Pollinations branch or text-only fallback.

- [ ] **Step 4: Run provider tests**

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::providers -- --nocapture
~~~

Expected: PASS with no network dependency.

- [ ] **Step 5: Commit provider adapters**

~~~
git add src-tauri/src/commands/generation/providers.rs src-tauri/src/commands/generation/types.rs
git commit -m "feat: support canonical base and img2img providers"
~~~

## Task 4: Add run storage and the split Tauri generation commands

**Files:**

- Create src-tauri/src/commands/generation/run.rs
- Create src-tauri/src/commands/generation/mod.rs
- Modify src-tauri/src/commands/mod.rs
- Modify src-tauri/src/commands/generate.rs
- Modify src-tauri/src/lib.rs
- Test unit tests in run.rs and command-level pure helpers in mod.rs

- [ ] **Step 1: Write failing run lifecycle tests**

Use tempfile::TempDir and a plain root path so tests do not require a Tauri window:

~~~
#[test]
fn run_lifecycle_preserves_completed_rows_and_resets_only_requested_row() {
    let dir = TempDir::new().unwrap();
    let mut manifest = create_run_at(dir.path(), "run-1", "siliconflow", "pet prompt").unwrap();
    mark_base_complete(&dir.path().join("runs/run-1"), &mut manifest).unwrap();
    mark_state_generating(&dir.path().join("runs/run-1"), &mut manifest, "idle").unwrap();
    mark_state_complete(&dir.path().join("runs/run-1"), &mut manifest, "idle").unwrap();
    mark_state_generating(&dir.path().join("runs/run-1"), &mut manifest, "walking").unwrap();
    let reloaded = load_manifest(&dir.path().join("runs/run-1")).unwrap();
    assert_eq!(reloaded.states["idle"].status, ArtifactStatus::Complete);
    assert_eq!(reloaded.states["walking"].status, ArtifactStatus::Generating);
    assert_eq!(reloaded.states["walking"].attempts, 1);
}

#[test]
fn discard_run_removes_only_the_run_directory() {
    let dir = TempDir::new().unwrap();
    create_run_at(dir.path(), "run-1", "localsd", "prompt").unwrap();
    std::fs::create_dir_all(dir.path().join("pets/existing")).unwrap();
    discard_run_at(dir.path(), "run-1").unwrap();
    assert!(!dir.path().join("runs/run-1").exists());
    assert!(dir.path().join("pets/existing").exists());
}
~~~

- [ ] **Step 2: Run run tests and verify they fail**

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation::run -- --nocapture
~~~

Expected: FAIL because the run storage module does not exist.

- [ ] **Step 3: Implement run storage and state transitions**

Implement:

~~~
pub fn create_run_at(app_data_dir: &Path, run_id: &str, provider: &str, base_prompt: &str) -> Result<GenerationRunManifest, String>;
pub fn run_dir(app_data_dir: &Path, run_id: &str) -> Result<PathBuf, String>;
pub fn manifest_path(run_dir: &Path) -> PathBuf;
pub fn load_manifest(run_dir: &Path) -> Result<GenerationRunManifest, String>;
pub fn save_manifest(run_dir: &Path, manifest: &GenerationRunManifest) -> Result<(), String>;
pub fn mark_base_generating(run_dir: &Path, manifest: &mut GenerationRunManifest) -> Result<(), String>;
pub fn mark_base_complete(run_dir: &Path, manifest: &mut GenerationRunManifest) -> Result<(), String>;
pub fn mark_state_generating(run_dir: &Path, manifest: &mut GenerationRunManifest, state: &str) -> Result<(), String>;
pub fn mark_state_complete(run_dir: &Path, manifest: &mut GenerationRunManifest, state: &str) -> Result<(), String>;
pub fn mark_failed(run_dir: &Path, manifest: &mut GenerationRunManifest, state: Option<&str>, message: &str) -> Result<(), String>;
pub fn reset_rows_after_base_retry(run_dir: &Path, manifest: &mut GenerationRunManifest) -> Result<(), String>;
pub fn discard_run_at(app_data_dir: &Path, run_id: &str) -> Result<(), String>;
~~~

Validate run_id as a UUID or a path-safe run identifier before joining it to runs; reject path separators and .. . Create only runs/<run_id>/manifest.json, base.png, and rows/*.png. Base retry increments Base attempts and deletes only incomplete run artifacts plus all state rows; state retry increments only that state. Never write to pets from run helpers.

- [ ] **Step 4: Implement the four Tauri commands**

In generation/mod.rs, expose these exact command signatures:

~~~
#[tauri::command]
pub async fn generate_base_preview(
    app: tauri::AppHandle,
    run_id: Option<String>,
    base_prompt: String,
    reference_data_url: Option<String>,
    image_provider: String,
    image_api_key: Option<String>,
    base_model: Option<String>,
    reference_model: Option<String>,
    local_sd_url: Option<String>,
    denoising_strength: Option<f32>,
) -> Result<BasePreviewResult, String>;

#[tauri::command]
pub async fn generate_state_row(
    app: tauri::AppHandle,
    run_id: String,
    state: String,
    image_provider: String,
    image_api_key: Option<String>,
    reference_model: Option<String>,
    local_sd_url: Option<String>,
    denoising_strength: Option<f32>,
) -> Result<StateRowResult, String>;

#[tauri::command]
pub fn assemble_run_preview(app: tauri::AppHandle, run_id: String) -> Result<AssembleRunPreviewResult, String>;

#[tauri::command]
pub fn discard_generation_run(app: tauri::AppHandle, run_id: String) -> Result<(), String>;
~~~

generate_base_preview creates or reuses the run, selects the chroma key from the optional reference photo (magenta fallback), builds the Base prompt, calls the selected provider’s text-to-image method, normalizes and validates base.png, updates the manifest, and returns a data URL. A provider or processing failure marks Base failed and is returned to the UI; it must not invoke another provider.

generate_state_row loads the manifest and Base, validates the requested state, builds the Row prompt, sends the Base data URL as the reference image, normalizes the provider output to one 1024×128 row, writes rows/<state>.png, updates only that state’s manifest record, and emits generation-progress with { runId, phase: "state", state, current: 1, total: 1 }. The command must reject a missing/incomplete Base and unknown states.

assemble_run_preview requires Base plus all four complete rows, stacks them into 1024×512, returns the data URL and { rowGap: 0 }, and does not write into pets. discard_generation_run removes only the validated run directory. Use a base progress event as well so the React step can show current phase without guessing.

Keep save_frame_selections and save_combined_sprite_sheet behavior available through the compatibility module. save_frame_selections remains the only command that writes generated PNGs under pets/<pet_id> before save_pet writes pet.json; it must retain the current four state names, frame metadata, and external grid cropping behavior.

- [ ] **Step 5: Register commands and preserve compatibility exports**

Update commands/mod.rs:

~~~
pub mod generation;
pub mod generate; // compatibility re-exports for existing callers
~~~

Make commands/generate.rs re-export the preserved import/save functions from generation, then register in src-tauri/src/lib.rs:

~~~
commands::generation::generate_base_preview,
commands::generation::generate_state_row,
commands::generation::assemble_run_preview,
commands::generation::discard_generation_run,
commands::generation::save_combined_sprite_sheet,
commands::generation::save_frame_selections,
~~~

Remove generate_and_assemble from the active Creator path and do not register a Pollinations fallback command. Keep old pet commands and plugin/settings commands unchanged.

- [ ] **Step 6: Run Rust tests and compile the Tauri command surface**

~~~
cargo test --manifest-path src-tauri/Cargo.toml generation -- --nocapture
cargo check --manifest-path src-tauri/Cargo.toml
~~~

Expected: all generation tests PASS and cargo check exits 0. No test should create pets/<id> during Base, Row, or assembly commands.

- [ ] **Step 7: Commit the run and command layer**

~~~
git add src-tauri/src/commands/generation/run.rs src-tauri/src/commands/generation/mod.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/generate.rs src-tauri/src/lib.rs
git commit -m "refactor: split sprite generation into retryable runs"
~~~

## Task 5: Update provider settings and migrate existing preferences

**Files:**

- Modify src/lib/settings.ts
- Modify src/windows/Creator/SettingsPanel.tsx
- Test src/lib/__tests__/settings.test.ts

- [ ] **Step 1: Write the settings migration tests**

Cover the default scheme-three path and old localStorage values:

~~~
it('defaults image generation to SiliconFlow with separate base/reference models', () => {
  localStorage.clear();
  const settings = loadSettings();
  expect(settings.imageProvider).toBe('siliconflow');
  expect(settings.imageBaseModel).toBe('Tongyi-MAI/Z-Image-Turbo');
  expect(settings.imageReferenceModel).toBe('Qwen/Qwen-Image-Edit-2509');
  expect(settings.localSdDenoisingStrength).toBe(0.55);
});

it('migrates legacy Pollinations settings to SiliconFlow without losing the API key', () => {
  localStorage.setItem('desktop-pet-settings', JSON.stringify({
    imageProvider: 'pollinations', imageApiKey: 'sf-key', imageModel: 'Tongyi-MAI/Z-Image-Turbo',
  }));
  const settings = loadSettings();
  expect(settings.imageProvider).toBe('siliconflow');
  expect(settings.imageApiKey).toBe('sf-key');
  expect(settings.imageBaseModel).toBe('Tongyi-MAI/Z-Image-Turbo');
});
~~~

- [ ] **Step 2: Run the settings tests and verify the red state**

~~~
npx vitest run src/lib/__tests__/settings.test.ts
~~~

Expected: FAIL because the new fields and migration are not implemented.

- [ ] **Step 3: Implement settings compatibility and model catalogs**

Keep imageModel as a legacy input during deserialization, add:

~~~
export type ImageProvider = 'siliconflow' | 'localsd' | 'pollinations';

export interface AppSettings {
  visionProvider: VisionProvider;
  visionApiKey: string;
  visionModel: string;
  imageProvider: ImageProvider;
  imageApiKey: string;
  imageModel: string;
  imageBaseModel: string;
  imageReferenceModel: string;
  localSdUrl: string;
  localSdDenoisingStrength: number;
}

export const SILICONFLOW_BASE_MODELS = [
  { value: 'Tongyi-MAI/Z-Image-Turbo', label: 'Z-Image-Turbo (Base)' },
  { value: 'Tongyi-MAI/Z-Image', label: 'Z-Image (Base)' },
  { value: 'baidu/ERNIE-Image-Turbo', label: 'ERNIE-Image-Turbo (Base)' },
];

export const SILICONFLOW_REFERENCE_MODELS = [
  { value: 'Qwen/Qwen-Image-Edit-2509', label: 'Qwen-Image-Edit-2509 (img2img)' },
  { value: 'Kwai-Kolors/Kolors', label: 'Kolors (img2img)' },
];
~~~

Set SiliconFlow as the default provider, Qwen/Qwen-Image-Edit-2509 as the reference model, and denoising strength 0.55. In loadSettings, merge defaults, copy legacy imageModel into imageBaseModel when the new field is absent, set a missing reference model/default denoising value, and map pollinations to siliconflow. saveSettings writes the new fields and may retain imageModel for one-version backward compatibility.

- [ ] **Step 4: Update SettingsPanel for scheme-three configuration**

Remove the Pollinations radio option from IMAGE_OPTIONS. Keep SiliconFlow and Local SD. For SiliconFlow render API key, Base model, and reference model selectors. For Local SD render URL and a number input/range input constrained to min=0.35, max=0.75, step=0.05, with 0.55 default. Save the new fields through the existing saveSettings function. The panel must not expose an option that sends a row request without a canonical Base image.

- [ ] **Step 5: Run settings and existing Creator tests**

~~~
npx vitest run src/lib/__tests__/settings.test.ts src/windows/Creator/steps/__tests__/GenerateStep.test.tsx
~~~

Expected: settings migration tests PASS. The old GenerateStep tests are expected to be updated in Task 6; do not retain assertions for generate_and_assemble.

- [ ] **Step 6: Commit settings changes**

~~~
git add src/lib/settings.ts src/lib/__tests__/settings.test.ts src/windows/Creator/SettingsPanel.tsx
git commit -m "feat: configure SiliconFlow and Local SD img2img"
~~~

## Task 6: Refactor Creator into Base confirmation and per-state generation

**Files:**

- Modify src/windows/Creator/steps/GenerateStep.tsx
- Create src/windows/Creator/steps/StateGenerationStep.tsx
- Modify src/windows/Creator/index.tsx
- Modify src/windows/Creator/steps/types.ts
- Modify src/windows/Creator/steps/__tests__/GenerateStep.test.tsx
- Create src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx

- [ ] **Step 1: Write failing Base-step tests**

Replace the old all-animation assertions with tests for the explicit Base contract:

~~~
it('generates a Base preview and confirms it without starting rows', async () => {
  mockInvoke.mockResolvedValue({ runId: 'run-1', dataUrl: 'data:image/png;base64,BASE', chromaKey: '#FF00FF' });
  render(<GenerateStep prompt="anime chibi girl" referenceDataUrl="data:image/jpeg;base64,REF" {...defaultProps} />);
  fireEvent.click(screen.getByRole('button', { name: /生成 Base/ }));
  await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('generate_base_preview', expect.objectContaining({
    basePrompt: 'anime chibi girl', referenceDataUrl: 'data:image/jpeg;base64,REF',
  })));
  expect(screen.getByAltText(/canonical base/i)).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: /确认 Base/ }));
  expect(defaultProps.onNext).toHaveBeenCalledWith({ runId: 'run-1', dataUrl: 'data:image/png;base64,BASE' });
});

it('retries only Base and never silently changes provider', async () => {
  mockInvoke.mockResolvedValue({ runId: 'run-1', dataUrl: 'data:image/png;base64,BASE2', chromaKey: '#FF00FF' });
  render(<GenerateStep prompt="pet" {...defaultProps} />);
  fireEvent.click(screen.getByRole('button', { name: /生成 Base/ }));
  await screen.findByRole('button', { name: /重新生成 Base/ });
  fireEvent.click(screen.getByRole('button', { name: /重新生成 Base/ }));
  await waitFor(() => expect(mockInvoke).toHaveBeenCalledTimes(2));
  expect(mockInvoke.mock.calls.every(([name]) => name === 'generate_base_preview')).toBe(true);
});
~~~

- [ ] **Step 2: Run Base tests and verify the red state**

~~~
npx vitest run src/windows/Creator/steps/__tests__/GenerateStep.test.tsx
~~~

Expected: FAIL because the current component calls generate_and_assemble and has no Base confirmation UI.

- [ ] **Step 3: Implement the Base step**

GenerateStep.tsx must accept:

~~~
interface GenerateStepProps {
  prompt: string;
  referenceDataUrl?: string;
  onNext: (base: { runId: string; dataUrl: string }) => void;
  onBack: () => void;
}
~~~

Load settings at render time, call generate_base_preview with basePrompt, referenceDataUrl, imageProvider, imageApiKey, imageBaseModel, imageReferenceModel, localSdUrl, and localSdDenoisingStrength, show the returned Base image and chroma key, and expose 生成 Base, 重新生成 Base, and 确认 Base actions. While pending, disable Back and generation controls. On failure, show the provider error in place and keep the user on Base; do not invoke another provider or auto-advance.

- [ ] **Step 4: Write failing state-row tests**

Add tests with a controlled mockInvoke sequence:

~~~
it('generates rows sequentially, displays progress, and assembles after all four complete', async () => {
  mockInvoke
    .mockResolvedValueOnce({ dataUrl: 'idle', state: 'idle', frameW: 128, frameH: 128, frameCount: 8 })
    .mockResolvedValueOnce({ dataUrl: 'walking', state: 'walking', frameW: 128, frameH: 128, frameCount: 8 })
    .mockResolvedValueOnce({ dataUrl: 'waving', state: 'waving', frameW: 128, frameH: 128, frameCount: 8 })
    .mockResolvedValueOnce({ dataUrl: 'working', state: 'working', frameW: 128, frameH: 128, frameCount: 8 })
    .mockResolvedValueOnce({ runId: 'run-1', dataUrl: 'combined', frameW: 128, frameH: 128, frameCount: 8, rowGap: 0 });
  render(<StateGenerationStep runId="run-1" {...defaultProps} />);
  fireEvent.click(screen.getByRole('button', { name: /生成全部动作/ }));
  await waitFor(() => expect(defaultProps.onNext).toHaveBeenCalledWith('combined', expect.objectContaining({
    petId: 'run-1', runId: 'run-1', layout: 'horizontalRows', rowGap: 0,
  })));
  const rowCalls = mockInvoke.mock.calls.filter(([name]) => name === 'generate_state_row');
  expect(rowCalls.map(([, args]) => args.state)).toEqual(['idle', 'walking', 'waving', 'working']);
  expect(mockInvoke).toHaveBeenLastCalledWith('assemble_run_preview', { runId: 'run-1' });
});

it('retries one failed state without regenerating completed states', async () => {
  mockInvoke.mockRejectedValueOnce(new Error('walking failed'));
  render(<StateGenerationStep runId="run-1" {...defaultProps} />);
  fireEvent.click(screen.getByRole('button', { name: /生成 walking/ }));
  await screen.findByRole('button', { name: /重试 walking/ });
  fireEvent.click(screen.getByRole('button', { name: /重试 walking/ }));
  await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('generate_state_row', expect.objectContaining({ state: 'walking', runId: 'run-1' })));
  expect(mockInvoke.mock.calls.filter(([name]) => name === 'generate_state_row').length).toBe(2);
});
~~~

- [ ] **Step 5: Run state tests and verify the red state**

~~~
npx vitest run src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx
~~~

Expected: FAIL because StateGenerationStep.tsx does not exist.

- [ ] **Step 6: Implement StateGenerationStep**

Use PET_STATES as the iteration order. The component must:

1. Call generate_state_row once for each incomplete state, sequentially, with the selected provider settings and runId.
2. Keep completed row results in local state and show each row preview/status.
3. Expose 生成全部动作 when no rows are running, and 重试 <state> only for failed rows.
4. Subscribe to generation-progress, filter events by runId, and display current state / 4 plus the provider error returned by the command.
5. Call assemble_run_preview({ runId }) only after all four row commands succeed, then call onNext with GeneratedSpriteConfig using petId=runId, runId=runId, rowGap=0, layout=horizontalRows, and four frame counts of 8.
6. Never invoke generate_and_assemble, Pollinations, or a text-only fallback.

The row retry handler must call only the failed state and then reassemble if all states are complete.

- [ ] **Step 7: Update Creator state transitions**

Change index.tsx as follows:

~~~
type Step = 'upload' | 'analyze' | 'base-generate' | 'state-generate' | 'sprite-import' | 'preview' | 'save';
const AI_STEPS: Step[] = ['upload', 'analyze', 'base-generate', 'state-generate', 'sprite-import', 'preview', 'save'];
~~~

Pass data.photoDataUrl to GenerateStep. On Base confirmation store generationRunId and baseDataUrl, then enter state-generate. On state assembly store generatedDataUrl and generatedConfig, then enter sprite-import. Back from state generation returns to Base without discarding the run. Resetting the Creator, returning to the mode chooser after a generated run, or completing final save must invoke discard_generation_run({ runId }) and clear generationRunId; failures in cleanup are displayed only in development logging and do not block final navigation.

- [ ] **Step 8: Run Creator tests and TypeScript compilation**

~~~
npx vitest run src/windows/Creator/steps/__tests__/GenerateStep.test.tsx src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx
npx tsc --noEmit
~~~

Expected: both component suites PASS and TypeScript exits 0.

- [ ] **Step 9: Commit the staged Creator flow**

~~~
git add src/windows/Creator/steps/GenerateStep.tsx src/windows/Creator/steps/StateGenerationStep.tsx src/windows/Creator/index.tsx src/windows/Creator/steps/types.ts src/windows/Creator/steps/__tests__/GenerateStep.test.tsx src/windows/Creator/steps/__tests__/StateGenerationStep.test.tsx
git commit -m "feat: add canonical base confirmation flow"
~~~

## Task 7: Preserve manual import, preview, save, and old animation playback

**Files:**

- Modify src/windows/Creator/steps/ManualFramePickerStep.tsx
- Modify src/windows/Creator/steps/PreviewStep.tsx
- Modify src/windows/Creator/steps/__tests__/PreviewStep.test.tsx
- Create src/windows/Creator/steps/__tests__/ManualFramePickerStep.test.tsx
- Modify src/windows/Creator/steps/SaveStep.tsx only if cleanup needs a final-save callback
- Modify src/windows/Pet/SpriteAnimator.tsx only if the compatibility test exposes a defect
- Create src/windows/Pet/__tests__/SpriteAnimator.test.tsx
- Test existing src/windows/Creator/steps/__tests__/SaveStep.test.tsx

- [ ] **Step 1: Write failing picker and playback compatibility tests**

Cover both layouts and old metadata:

~~~
it('preselects all four horizontal rows from a generated run without gaps', () => {
  render(<ManualFramePickerStep
    initialDataUrl="data:image/png;base64,COMBINED"
    initialPetId="run-1"
    initialConfig={{ frameW: 128, frameH: 128, rowGap: 0, layout: 'horizontalRows', idleFrames: 8, walkingFrames: 8, wavingFrames: 8, workingFrames: 8 }}
    {...defaultProps}
  />);
  expect(screen.getByText(/idle.*8/)).toBeTruthy();
  expect(screen.getByText(/walking.*8/)).toBeTruthy();
});

it('keeps the existing external grid import defaults', () => {
  render(<ManualFramePickerStep {...defaultProps} />);
  fireEvent.change(screen.getByLabelText(/PNG/), { target: { files: [makeFourByTwoPngFile()] } });
  expect(screen.getByLabelText(/列间距/)).toBeTruthy();
  expect(screen.getByLabelText(/行间距/)).toBeTruthy();
});
~~~

For the animator, mock Image and requestAnimationFrame, render metadata { cols: 4, rows: 2, frameCount: 4, frameW: 128, frameH: 128, delayMs: 200 }, then { cols: 8, rows: 1, frameCount: 8, ... }; assert both initialize a canvas and draw frames without out-of-range source coordinates.

- [ ] **Step 2: Run compatibility tests and verify the red state**

~~~
npx vitest run src/windows/Creator/steps/__tests__/ManualFramePickerStep.test.tsx src/windows/Pet/__tests__/SpriteAnimator.test.tsx
~~~

Expected: FAIL until the picker understands layout=horizontalRows and the test harness covers the existing animator behavior.

- [ ] **Step 3: Implement horizontal generated-preview support without changing external import**

Update ManualFramePickerStep so initialConfig includes rowGap and layout. For horizontalRows, initialize cells as (col=0..7,row=0..3), set colGap=0, set rowGap=0, and keep the four state frame counts from the config. For grid, retain the current configurable frameW, frameH, colGap, rowGap fields and 4×2 selection behavior. The save_frame_selections payload remains idleCells, walkingCells, wavingCells, and workingCells, so both layouts converge to the existing horizontal PNG output.

- [ ] **Step 4: Make PreviewStep consume the shared catalog**

Replace the local hard-coded STATES declaration with PET_STATES and a label map exported from src/types/pet.ts. Keep four preview images, existing appDataDir/join path resolution, and Back/Next button behavior. Preserve the current SpriteStateInfo metadata supplied by SaveStep. Update tests to assert the catalog labels and four image sources.

- [ ] **Step 5: Verify SpriteAnimator compatibility and make the smallest fix if needed**

Keep the existing frame calculation:

~~~
const col = frameRef.current % cols;
const row = Math.floor(frameRef.current / cols);
ctx.drawImage(img, col * frameW, row * frameH, frameW, frameH, 0, 0, w, h);
frameRef.current = (frameRef.current + 1) % frameCount;
~~~

It must remain valid for old 4×2 sheets and new 8×1 sheets. If the compatibility test finds a defect, fix only the frame reset/canvas cleanup required by the test; do not migrate old image files or change pet.json field names.

- [ ] **Step 6: Run the full React compatibility suite**

~~~
npx vitest run src/windows/Creator/steps/__tests__ src/windows/Pet/__tests__
npx tsc --noEmit
~~~

Expected: all Creator and Pet tests PASS and TypeScript exits 0. Existing external import and SaveStep tests must remain green.

- [ ] **Step 7: Commit compatibility work**

~~~
git add src/windows/Creator/steps/ManualFramePickerStep.tsx src/windows/Creator/steps/PreviewStep.tsx src/windows/Creator/steps/__tests__/ManualFramePickerStep.test.tsx src/windows/Creator/steps/__tests__/PreviewStep.test.tsx src/windows/Pet/SpriteAnimator.tsx src/windows/Pet/__tests__/SpriteAnimator.test.tsx
git commit -m "fix: preserve imported sheets and legacy animation playback"
~~~

## Task 8: Verify provider payloads, run isolation, build output, and acceptance criteria

**Files:**

- Modify only files revealed by failing verification commands.
- Test all Rust and TypeScript tests.

- [ ] **Step 1: Run the complete Rust test suite**

~~~
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
~~~

Expected: PASS. This must include state catalog, prompt, chroma, request-body, manifest lifecycle, row validation, old save_combined_sprite_sheet, and save_frame_selections tests.

- [ ] **Step 2: Run the complete frontend test suite**

~~~
npx vitest run
~~~

Expected: PASS for every existing and new test file, including Analyze, Upload, Generate/Base, StateGeneration, ManualFramePicker, Preview, Save, settings, and SpriteAnimator tests.

- [ ] **Step 3: Run typecheck and production build**

~~~
npx tsc --noEmit
npm run build
~~~

Expected: both commands exit 0. npm run build must not reintroduce a call to generate_and_assemble or a Pollinations branch in the active Creator bundle.

- [ ] **Step 4: Run static contract checks**

~~~
rg -n "generate_and_assemble|pollinations|imageReferenceModel|denoising_strength|init_images|Qwen/Qwen-Image-Edit-2509|rows/<state>" src src-tauri
git diff --check
~~~

Expected: generate_and_assemble appears only in an intentional compatibility comment/test exclusion if retained; active React code contains only siliconflow and localsd provider choices; Rust contains SiliconFlow Row image, Local SD Row init_images, and the Qwen reference model; git diff --check reports no new whitespace errors.

- [ ] **Step 5: Perform a manual local acceptance run**

With SiliconFlow configured, verify:

1. Base generation shows one preview and 确认 Base advances without generating rows.
2. Base retry invokes only generate_base_preview and replaces Base.
3. Four rows run in order, each includes the Base image in the provider payload, and a failed row exposes only its own retry.
4. Assembly produces a 1024×512 four-row preview; picker preselects 8 frames per state with zero gaps.
5. Final save creates only pets/<pet_id>/pet.json and four state PNGs; cancelling before final save leaves no new pets/<pet_id> directory and discard_generation_run removes only runs/<run_id>.

With Local SD configured, verify Base calls /sdapi/v1/txt2img, Row calls /sdapi/v1/img2img with init_images and the selected denoising value, and the same save path succeeds.

- [ ] **Step 6: Review the final diff and commit only implementation files**

~~~
git status --short
git diff --stat
git diff --check
~~~

Confirm that unrelated user changes are preserved and every implementation commit contains only the files named by its task. Do not create a cleanup commit that stages unrelated work.

## Plan self-review

Spec coverage checked against the approved design:

- Canonical Base first, Base confirmation/retry, per-state retry, and no silent fallback: Tasks 4 and 6.
- SiliconFlow Base text-to-image plus Row image-to-image with separate models: Tasks 3, 5, and 6.
- Local SD txt2img plus img2img, init_images, and denoising range: Tasks 3 and 5.
- Run manifest, status/attempt tracking, retry isolation, cleanup, and no writes to pets before save: Task 4 and Task 8.
- Hatch-pet-style concise prompts, state requirements, chroma candidates, distance selection, and transparent RGB normalization: Task 2.
- Four-state current contract with future expansion boundary: Task 1.
- Existing external import, old pet.json, 128×128 playback, and save path: Task 7.
- Rust/React automated coverage and acceptance verification: Tasks 1–3, 6–8.

The plan was checked for unresolved instructions; every implementation step names exact files, code/signatures, commands, and expected output. Function names, result property names, provider fields, and GeneratedSpriteConfig fields are consistent across the Rust command signatures and TypeScript invoke payloads.
