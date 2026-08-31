# Sprite Generation Quality V2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `subagent-driven-development` (recommended) or `executing-plans` to implement this plan task-by-task.

**Goal:** Make AI sprite generation style-aware for real photos, preserve existing illustration styles, produce reliable transparent mattes, lock orientation, require a complete working laptop, and keep shared scene elements aligned across frames.

**Architecture:** Add an explicit `SourceStyle` value at the creator boundary and persist it in the generation manifest. The Rust prompt layer will render separate realistic-photo and stylized-art contracts, while all state prompts share one canonical facing contract. The sprite normalizer will use key-aware 8-connected matte cleanup followed by fixed global source-slot transforms instead of per-frame recentering.

**Tech Stack:** React 19 + TypeScript, Tauri 2, Rust, `image` crate, Vitest, Testing Library, Cargo library tests.

---

## Task 1: Carry the source-style choice through the creator UI

**Files:**

- Modify: `src/windows/Creator/steps/types.ts`
- Modify: `src/windows/Creator/index.tsx`
- Modify: `src/windows/Creator/steps/AnalyzeStep.tsx`
- Modify: `src/windows/Creator/steps/GenerateStep.tsx`
- Test: `src/windows/Creator/steps/__tests__/AnalyzeStep.test.tsx`
- Test: `src/windows/Creator/steps/__tests__/GenerateStep.test.tsx`

- [ ] **Step 1: Write failing UI tests for the explicit style contract**

Extend the existing `AnalyzeStep` tests with the two radio labels and a payload assertion. The default should be `realistic`, because the current AI creator starts from a user-uploaded reference photo and the reported regression is an unwanted 3D result; a user with a cartoon image can select `stylized` before continuing.

```tsx
it('lets the user choose realistic-photo conversion or preserve stylized art', () => {
  render(<AnalyzeStep {...defaultProps} />);
  expect(screen.getByRole('radio', { name: /真人|realistic/i })).toBeTruthy();
  expect(screen.getByRole('radio', { name: /卡通|stylized/i })).toBeTruthy();
  expect((screen.getByRole('radio', { name: /真人|realistic/i }) as HTMLInputElement).checked).toBe(true);
});

it('passes the selected source style with the description', () => {
  const onNext = vi.fn();
  render(<AnalyzeStep {...defaultProps} onNext={onNext} />);
  fireEvent.change(screen.getByRole('textbox', { name: /character description/i }), {
    target: { value: 'pink cartoon character' },
  });
  fireEvent.click(screen.getByRole('radio', { name: /卡通|stylized/i }));
  fireEvent.click(screen.getByRole('button', { name: /下一步|涓嬩竴姝?/i }));
  expect(onNext).toHaveBeenCalledWith('pink cartoon character', 'stylized');
});
```

Add `sourceStyle: 'realistic'` to `defaultProps` in the `GenerateStep` test and assert that the invoke payload contains `sourceStyle: 'realistic'`.

- [ ] **Step 2: Run the focused UI tests and verify they fail**

Run:

```text
npx vitest run src/windows/Creator/steps/__tests__/AnalyzeStep.test.tsx src/windows/Creator/steps/__tests__/GenerateStep.test.tsx
```

Expected: TypeScript/test failures because the props, radio controls, callback signature, and backend payload do not yet contain `sourceStyle`.

- [ ] **Step 3: Implement the style value and UI data flow**

In `steps/types.ts`, add:

```ts
export type SourceStyle = 'realistic' | 'stylized';
```

Add `sourceStyle: SourceStyle` to `WizardData` and set `INITIAL_WIZARD_DATA.sourceStyle` to `'realistic'`. Update `AnalyzeStepProps` to accept `initialSourceStyle?: SourceStyle` and `onNext: (prompt: string, sourceStyle: SourceStyle) => void`. Keep the local state initialized from the prop or `'realistic'`, render the two radio choices, and call `onNext(prompt, sourceStyle)`.

Pass `data.sourceStyle` into `AnalyzeStep` from `CreatorWindow`. In its `onNext`, update both `prompt` and `sourceStyle` before entering the base-generation step. Pass the value into `GenerateStep`; add `sourceStyle: SourceStyle` to its props and include the camel-case Tauri argument:

```ts
sourceStyle,
```

Do not change the sprite-import mode; it has no source image style to classify.

- [ ] **Step 4: Run the focused UI tests and verify they pass**

Run the same Vitest command from Step 2. Expected: all existing and new tests pass.

- [ ] **Step 5: Commit the UI boundary**

```text
git add src/windows/Creator/steps/types.ts src/windows/Creator/index.tsx src/windows/Creator/steps/AnalyzeStep.tsx src/windows/Creator/steps/GenerateStep.tsx src/windows/Creator/steps/__tests__/AnalyzeStep.test.tsx src/windows/Creator/steps/__tests__/GenerateStep.test.tsx
git commit -m "feat: add source style choice to sprite generation"
```

## Task 2: Make vision analysis describe, rather than overwrite, source style

**Files:**

- Modify: `src/lib/vision.ts`
- Modify: `src/lib/claude-vision.ts`
- Test: `src/lib/__tests__/claude-vision.test.ts`

- [ ] **Step 1: Write a failing request-contract test**

Inspect the JSON sent by `analyzePhoto` and assert that the system/user instructions require source-medium identification and faithful style description:

```ts
it('asks vision analysis to identify and preserve the input medium', async () => {
  mockFetch.mockResolvedValueOnce({
    ok: true,
    json: async () => ({ content: [{ type: 'text', text: 'stylized cartoon, orange fox' }] }),
  });

  await analyzePhoto('data:image/jpeg;base64,abc123', 'sk-ant-test');
  const body = JSON.parse(mockFetch.mock.calls[0][1].body as string) as {
    system: string;
    messages: Array<{ content: Array<{ type: string; text?: string }> }>;
  };
  const instruction = `${body.system} ${body.messages[0].content.find((item) => item.type === 'text')?.text ?? ''}`.toLowerCase();
  expect(instruction).toContain('source style');
  expect(instruction).toContain('preserve');
  expect(instruction).toContain('photorealistic');
});
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```text
npx vitest run src/lib/__tests__/claude-vision.test.ts
```

Expected: FAIL because the current prompt always asks for a Q-version chibi description and does not mention preserving the source medium.

- [ ] **Step 3: Update both vision prompt constants and message text**

Use the same wording in `vision.ts` and `claude-vision.ts` so all providers receive the same contract:

```text
Analyze the reference image for a desktop-pet generator. First identify the source medium/style as either realistic human photo or stylized artwork (cartoon, anime, illustration, or pixel art). Describe the character's recognizable features and also the source style's line quality, proportions, palette, shading, and texture. Do not convert an existing stylized artwork into generic Q-version wording. Output one concise comma-separated character description under 80 words; the caller separately chooses whether a realistic photo should be transformed into a cute 2D chibi illustration.
```

Change the user text to ask for a faithful source-style description. The explicit UI choice remains authoritative; the vision service only enriches the description.

- [ ] **Step 4: Run the focused test and verify it passes**

Run the command from Step 2. Expected: all vision tests pass.

- [ ] **Step 5: Commit the analysis contract**

```text
git add src/lib/vision.ts src/lib/claude-vision.ts src/lib/__tests__/claude-vision.test.ts
git commit -m "fix: preserve source style in vision descriptions"
```

## Task 3: Add backend source-style persistence and prompt contracts

**Files:**

- Modify: `src-tauri/src/commands/generation/types.rs`
- Modify: `src-tauri/src/commands/generation/run.rs`
- Modify: `src-tauri/src/commands/generation/mod.rs`
- Modify: `src-tauri/src/commands/generation/prompts.rs`
- Test: `src-tauri/src/commands/generation/types.rs`
- Test: `src-tauri/src/commands/generation/run.rs`
- Test: `src-tauri/src/commands/generation/prompts.rs`

- [ ] **Step 1: Write failing Rust tests for style, orientation, idle, and working contracts**

Add the serializable style enum expected by the UI:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStyle {
    Realistic,
    #[default]
    Stylized,
}
```

Before implementing it, add prompt assertions with the intended future signatures:

```rust
let realistic = build_base_prompt(
    "a person with black hair",
    SourceStyle::Realistic,
    "#FF00FF",
    "magenta",
).to_lowercase();
for term in [
    "2d",
    "chibi",
    "cute",
    "flat illustration",
    "no photorealistic",
    "no 3d",
    "no cgi",
] {
    assert!(realistic.contains(term), "missing realistic term: {term}");
}

let stylized = build_base_prompt(
    "a pixel-art orange fox",
    SourceStyle::Stylized,
    "#FF00FF",
    "magenta",
).to_lowercase();
for term in ["preserve the original art style", "line quality", "pixel art", "no 3d"] {
    assert!(stylized.contains(term), "missing stylized term: {term}");
}
```

Add tests that every state uses the same exact facing string and that the sleeping prompt does not contain `three-quarter`. Add the working assertions for `open laptop computer`, `hinged screen panel`, `keyboard deck`, `both hands type`, `same desk geometry`, `standalone keyboard`, and `keyboard-only`. Add the static idle assertions from the approved design: `static`, `same static pose`, `no breathing`, and no `8 visibly different frames`.

Add a manifest round-trip test where `source_style` is serialized, plus a JSON fixture/value with the field omitted that deserializes to `SourceStyle::Stylized`.

- [ ] **Step 2: Run the generation library tests and verify they fail**

Run:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib generation:: -- --nocapture
```

Expected: compile/test failures because the new enum, prompt parameters, manifest field, and contracts are absent.

- [ ] **Step 3: Implement source-style parsing and manifest compatibility**

In `types.rs`, add `SourceStyle`, add `#[serde(default)] pub source_style: SourceStyle` to `GenerationRunManifest`, and pass the value through `GenerationRunManifest::new`.

In `run.rs`, update `create_run_at` to accept `SourceStyle`, pass it into new manifests, and leave `load_manifest` compatible with old JSON through the serde default. Existing run files must not be rewritten merely by loading them.

In `mod.rs`, add a small parser next to provider validation:

```rust
fn source_style(value: Option<&str>) -> Result<SourceStyle, String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(SourceStyle::Stylized),
        Some("realistic") => Ok(SourceStyle::Realistic),
        Some("stylized") => Ok(SourceStyle::Stylized),
        Some(value) => Err(format!("unsupported source style: {value}")),
    }
}
```

Add `source_style: Option<String>` to `generate_base_preview`. Parse it before creating/loading the run, assign it to `manifest.source_style` when a retry reuses the run, save the manifest, and pass the enum to `build_base_prompt`. Pass `manifest.source_style` to `build_row_prompt` in `generate_state_row`, ensuring retries and later state generation use the same style as the confirmed base.

- [ ] **Step 4: Implement state and prompt contracts**

Add `FrameVariation::{Static, Animated}` to `types.rs` and the field to `StateDefinition`. Set idle to `Static`, keep the other three animated, and use one canonical facing literal for every state:

```rust
const CANONICAL_FACING: &str =
    "forward, straight-on, exactly the same camera angle and left-right orientation as the canonical base image";
```

Replace the sleeping three-quarter allowance. Make working describe one complete open laptop with a visibly hinged screen panel and horizontal keyboard deck, placed flat on the fixed lower-third desk, and explicitly prohibit standalone keyboard/tablet/closed/lap-held/held devices and independent monitor substitution.

In `prompts.rs`, keep separate static and animated frame contracts so idle is not contradicted by the generic row text. Add `source_style_contract(SourceStyle)` and insert it into both base and row prompts. The realistic contract must say 2D cute chibi/flat illustration and forbid photorealistic, 3D render, CGI, realistic skin pores, plastic materials, and cinematic lighting. The stylized contract must say to preserve the input artwork's medium, line quality, proportions, palette, shading, and texture and not restyle it.

The row prompt must repeat the facing lock in positive and negative wording: same camera, same body/head orientation, no mirror, flip, side turn, three-quarter change, or partial reversal in any frame.

- [ ] **Step 5: Run the generation library tests and verify they pass**

Run the command from Step 2. Expected: all generation prompt, type, and run tests pass.

- [ ] **Step 6: Commit the backend prompt/data contract**

```text
git add src-tauri/src/commands/generation/types.rs src-tauri/src/commands/generation/run.rs src-tauri/src/commands/generation/mod.rs src-tauri/src/commands/generation/prompts.rs
git commit -m "fix: make sprite prompts style and direction aware"
```

## Task 4: Make chroma-key cleanup remove fine enclosed gaps safely

**Files:**

- Modify: `src-tauri/src/commands/generation/sprite.rs`
- Test: `src-tauri/src/commands/generation/sprite.rs`

- [ ] **Step 1: Write failing matte regression tests**

Add three synthetic-image tests using a requested magenta key and an off-key sampled border color such as `[12, 12, 12]`, so the tests distinguish key cleanup from sampled-background flood traversal:

```rust
#[test]
fn removes_a_diagonally_connected_sampled_background_pixel() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let background = Rgba([12, 12, 12, 255]);
    let foreground = Rgba([220, 220, 220, 255]);
    let mut image = RgbaImage::from_pixel(3, 3, foreground);
    image.put_pixel(0, 0, background);
    image.put_pixel(1, 1, background); // only 8-connected to (0, 0)

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(1, 1).0, [0, 0, 0, 0]);
}

#[test]
fn removes_a_small_enclosed_background_hole_like_a_hair_gap() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let background = Rgba([12, 12, 12, 255]);
    let foreground = Rgba([220, 220, 220, 255]);
    let mut image = RgbaImage::from_pixel(9, 9, background);
    for y in 2..7 {
        for x in 2..7 {
            image.put_pixel(x, y, foreground);
        }
    }
    image.put_pixel(4, 4, background);

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(4, 4).0, [0, 0, 0, 0]);
}

#[test]
fn preserves_an_oversized_enclosed_background_like_region() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let background = Rgba([12, 12, 12, 255]);
    let foreground = Rgba([220, 220, 220, 255]);
    let mut image = RgbaImage::from_pixel(120, 120, background);
    for y in 10..110 {
        for x in 10..110 {
            image.put_pixel(x, y, foreground);
        }
    }
    for y in 20..100 {
        for x in 20..100 {
            image.put_pixel(x, y, background);
        }
    }

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(60, 60).0, background.0);
}
```

Add a separate transparent-RGB assertion using a 1x1 exact key pixel:

```rust
#[test]
fn normalizes_rgb_for_every_pixel_made_fully_transparent() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let mut image = RgbaImage::from_pixel(1, 1, Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]));

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 0]);
}
```

- [ ] **Step 2: Run the focused Rust tests and verify they fail**

Run:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib generation::sprite -- --nocapture
```

Expected: the diagonal and enclosed-hole assertions fail against the current four-neighbor edge-only implementation.

- [ ] **Step 3: Implement bounded 8-connected matte cleanup**

Replace the four-neighbor helper with an 8-neighbor iterator that enqueues each coordinate at most once. Keep an `edge_processed` bitmap. Sample the output border before mutation, then:

1. Apply the existing key-color alpha ramp globally to the configured chroma key. This is safe for the requested key because the prompt forbids that color inside character components and it removes key-colored enclosed hair gaps even when they are not edge-connected.
2. Run an 8-connected BFS from all image borders against the sampled output background color. Use the existing hard threshold/ramp; only matching pixels enqueue neighbors, and protect non-background character pixels.
3. Scan unprocessed background-like pixels as connected components. Use 8-neighbor traversal, stop storing a component after `MAX_COMPONENT_SIZE = 4096`, and clear only components that are enclosed and below the cap. Use the same alpha ramp and normalize every zero-alpha pixel's RGB.

Factor the repeated color-distance logic into helpers with these signatures:

```rust
fn background_alpha(pixel: &Rgba<u8>, color: [u8; 3], threshold: f32, ramp_end: f32) -> Option<u8>;
fn apply_background_alpha(image: &mut RgbaImage, x: u32, y: u32, color: [u8; 3], threshold: f32, ramp_end: f32) -> bool;
fn is_background_like(pixel: &Rgba<u8>, color: [u8; 3], threshold: f32, ramp_end: f32) -> bool;
```

Use a queued bitmap for both border BFS and component BFS so a large flat background cannot produce duplicate queue entries or unbounded memory growth.

- [ ] **Step 4: Run the focused Rust tests and verify they pass**

Run the command from Step 2. Expected: all sprite tests, including existing border sampling and alpha normalization tests, pass.

- [ ] **Step 5: Commit the matte implementation**

```text
git add src-tauri/src/commands/generation/sprite.rs
git commit -m "fix: clean enclosed sprite background gaps"
```

## Task 5: Preserve shared desk/table coordinates while normalizing rows

**Files:**

- Modify: `src-tauri/src/commands/generation/sprite.rs`
- Test: `src-tauri/src/commands/generation/sprite.rs`

- [ ] **Step 1: Write a failing shared-scene alignment test**

Create a 2048x256 synthetic row with a unique green desk marker at the same relative x/y in every 256-pixel source slot. Give frame 0 a character protrusion on the left and frame 1 a protrusion on the right, while adding opaque anchors at source x=0 and x=2047 so the global visible bounds line up with the eight source slots. After normalization, find the green marker bounds in each destination frame and assert the marker's relative left coordinate is identical.

```rust
#[test]
fn row_normalization_keeps_shared_desk_x_coordinates_when_character_width_changes() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let width = API_FRAME_W * DEFAULT_FRAME_COUNT;
    let mut source = RgbaImage::from_pixel(
        width,
        API_FRAME_H,
        Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
    );
    let desk = Rgba([20, 220, 40, 255]);
    let character = Rgba([220, 80, 80, 255]);
    let anchor = Rgba([80, 80, 220, 255]);

    for frame_index in 0..DEFAULT_FRAME_COUNT {
        let source_x = frame_index * API_FRAME_W;
        for x in (source_x + 80)..(source_x + 160) {
            for y in 190..203 {
                source.put_pixel(x, y, desk);
            }
        }
        let character_start = match frame_index {
            0 => source_x + 20,
            1 => source_x + 180,
            _ => source_x + 105,
        };
        for x in character_start..(character_start + 30) {
            for y in 40..180 {
                source.put_pixel(x, y, character);
            }
        }
    }
    source.put_pixel(0, 0, anchor);
    source.put_pixel(width - 1, 0, anchor);

    let normalized = normalize_horizontal_row(&png_bytes(&source), &key).unwrap();
    let offsets = (0..DEFAULT_FRAME_COUNT)
        .map(|frame_index| {
            let start_x = frame_index * FRAME_W;
            (start_x..start_x + FRAME_W)
                .filter(|x| {
                    (0..FRAME_H).any(|y| {
                        let pixel = normalized.get_pixel(*x, y);
                        pixel[1] > 150 && pixel[0] < 100 && pixel[2] < 100
                    })
                })
                .map(|x| x - start_x)
                .next()
                .expect("desk marker should be present in every frame")
        })
        .collect::<Vec<_>>();

    assert!(
        offsets.windows(2).all(|pair| pair[0] == pair[1]),
        "shared desk marker drifted between frames: {offsets:?}"
    );
}
```

The helper may inspect pixels whose green channel is greater than 150 and whose red/blue channels are below 100; this remains stable through Lanczos3 interpolation.

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib generation::sprite::tests::row_normalization_keeps_shared_desk_x_coordinates_when_character_width_changes -- --nocapture
```

Expected: FAIL because the current implementation finds each segment's visible x bounds and recenters it independently.

- [ ] **Step 3: Implement one shared source-slot transform**

In `slice_row_into_frames`:

1. Find the full row visible bounds once.
2. Compute `content_w = x_max - x_min + 1`, `content_h = y_max - y_min + 1`, and `segment_w = ceil(content_w / frame_count)` using checked arithmetic.
3. Compute one `global_scale = min(FRAME_H/content_h, FRAME_W/segment_w)`.
4. For every frame, take the fixed source range `[x_min + i*segment_w, x_min + min((i+1)*segment_w, content_end))`; pad a short final range to `segment_w` with transparent pixels.
5. Resize every padded slot with the same dimensions and place it at the same `dst_frame_x`, horizontal offset, and bottom baseline.

Delete `find_segment_x_bounds` and all per-frame recentering. Keep empty slots transparent so existing `validate_sprite_row` reports an actionable empty-frame error. Use `saturating_add`/`checked_mul` consistently for `frame_count` and slot boundaries.

- [ ] **Step 4: Run the focused and complete Rust library tests**

Run:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib generation::sprite -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --lib generation:: -- --nocapture
```

Expected: all sprite and generation library tests pass, including the new shared-desk alignment assertion.

- [ ] **Step 5: Commit the fixed-coordinate normalizer**

```text
git add src-tauri/src/commands/generation/sprite.rs
git commit -m "fix: preserve shared sprite scene coordinates"
```

## Task 6: Run the complete verification suite and inspect the final diff

**Files:**

- No new files; inspect all files changed by Tasks 1–5.

- [ ] **Step 1: Run Rust formatting check without rewriting unrelated files**

Run:

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

If the repository's pre-existing unrelated formatting differences make this command fail, record that limitation and do not reformat unrelated files. Ensure changed files have no newly introduced formatting errors.

- [ ] **Step 2: Run the full Rust library test suite**

Run:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: exit code 0 and all library tests pass.

- [ ] **Step 3: Run all frontend tests and production build**

Run:

```text
npx vitest run
npm run build
```

Expected: all Vitest tests pass and Vite/TypeScript build exits 0.

- [ ] **Step 4: Run repository diff checks**

Run:

```text
git diff --check origin/main...HEAD
git status --short --branch
```

Confirm only intended source/docs commits are present and `template.webp` remains untracked and unstaged.

- [ ] **Step 5: Request code review before integration**

Use `requesting-code-review` with the implementation commits and test output. Resolve any correctness findings, then run the relevant tests again before claiming completion.

- [ ] **Step 6: Keep the final commit scope clean**

If the review requires a correction, stage only the exact source/test file named by the review, rerun its relevant verification command, and use a commit message describing that correction. If no correction is required, do not create an empty follow-up commit. In all cases, leave `template.webp`, generated build output, API keys, and unrelated working-tree files unstaged.
