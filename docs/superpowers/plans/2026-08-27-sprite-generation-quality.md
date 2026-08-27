# Sprite Generation Quality Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make generated idle/working sprite rows match the intended actions, keep desk geometry stable across frames, and remove enclosed chroma-key gaps cleanly.

**Architecture:** Keep state behavior in the Rust state catalog and have `build_row_prompt` choose a static or animated frame contract from that catalog. Keep image cleanup in `sprite.rs`: first remove edge/enclosed background regions, then slice every source slot with one shared transform so per-frame visible bounds cannot move furniture.

**Tech Stack:** Rust 2021, Tauri command library, `image` 0.25 (`RgbaImage`), Cargo unit tests, Vitest/TypeScript regression suite.

---

## Scope and file map

The approved design is one focused generation-pipeline change; it does not need a new provider or a frontend change.

- Modify `src-tauri/src/commands/generation/types.rs`:
  - Add a catalog-level static/animated frame-variation field.
  - Replace the idle breathing contract with a stationary standing contract.
  - Require an open laptop and a fixed desk composition for working.
- Modify `src-tauri/src/commands/generation/prompts.rs`:
  - Make the generic row prompt choose the correct static or animated wording.
  - Add explicit laptop, desk-anchor, enclosed-background, and spill constraints.
  - Add prompt regression assertions.
- Modify `src-tauri/src/commands/generation/sprite.rs`:
  - Use 8-connected matte traversal and remove small enclosed near-background holes.
  - Replace per-frame visible-bounds recentering with a common slot transform.
  - Update sprite regression tests for the new invariants.
- No new production files are required.

Before implementation starts, use the `using-git-worktrees` workflow to confirm
whether this checkout is already isolated. Preserve the existing untracked
`template.webp`; never add it to a task commit.

## Task 1: Correct the state and prompt contracts

**Files:**

- Modify: `src-tauri/src/commands/generation/types.rs:33-76, 191-207`
- Modify: `src-tauri/src/commands/generation/prompts.rs:3-27, 115-184`

### Step 1: Write the failing prompt tests

Add these tests to the existing `src-tauri/src/commands/generation/prompts.rs`
test module. They compile against the existing public prompt builders but fail
because the current catalog still asks for breathing and still permits a
keyboard-only working prop.

```rust
#[test]
fn idle_prompt_requests_a_static_standing_hold_without_breathing_or_blinking() {
    let state = state_definition("idle").unwrap();

    let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
    let lower = prompt.to_lowercase();

    assert!(lower.contains("static"));
    assert!(lower.contains("stable copies of the same static pose"));
    assert!(lower.contains("no breathing"));
    assert!(lower.contains("no blinking"));
    assert!(!lower.contains("breathing loop"));
    assert!(!lower.contains("chest rises"));
    assert!(!lower.contains("brief eye blink"));
    assert!(!lower.contains("8 visibly different frames"));
}

#[test]
fn working_prompt_requires_an_open_laptop_on_a_fixed_desk() {
    let state = state_definition("working").unwrap();

    let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
    let lower = prompt.to_lowercase();

    for term in [
        "open laptop computer",
        "hinged screen panel",
        "keyboard deck",
        "rests flat on the tabletop",
        "both hands type on the laptop keyboard",
        "fixed lower-third",
        "same desk geometry",
        "not on the character's lap",
        "not being held",
        "standalone keyboard",
        "keyboard-only output",
    ] {
        assert!(lower.contains(term), "missing working term: {term}");
    }

    assert!(!lower.contains("laptop or keyboard"));
    assert!(!lower.contains("laptop/keyboard"));
}

#[test]
fn animated_states_keep_the_distinct_frame_contract() {
    let state = state_definition("working").unwrap();

    let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
    let lower = prompt.to_lowercase();

    assert!(lower.contains("8 visibly different frames"));
    assert!(lower.contains("small continuous increment"));
    assert!(!lower.contains("stable copies of the same static pose"));
}
```

### Step 2: Run the prompt tests and verify the red state

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::prompts -- --nocapture
```

Expected: the three new tests fail with missing static/laptop terms; existing
prompt tests remain runnable. Do not change production code until this failure
is observed.

### Step 3: Implement the catalog-level variation and prompt text

In `types.rs`, add a catalog-level frame contract so generic prompt text cannot
force a static state to animate:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameVariation {
    Static,
    Animated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub delay_ms: u32,
    pub facing: &'static str,
    pub action: &'static str,
    pub requirements: &'static str,
    pub frame_variation: FrameVariation,
}
```

Set `frame_variation` to `FrameVariation::Static` only for `idle` and to
`FrameVariation::Animated` for `sleeping`, `acting_cute`, and `working`. Replace
the two idle strings with the following exact contracts:

```rust
action: "a completely static standing hold across all 8 frames; use the same upright neutral relaxed pose in every column with no breathing, body expansion, blink, sway, or pose change",
requirements: "The character MUST remain standing upright, front-facing, and planted in exactly the same position in every frame. All 8 columns are stable copies of the same waiting pose: no breathing animation, no body scaling, no blinking, no head turn, no arm or leg movement, no translation, no desk work, and no new props. Keep the camera, scale, baseline, silhouette, and background identical.",
frame_variation: FrameVariation::Static,
```

Replace the two working strings with a single explicit laptop/desk scene:

```rust
action: "the character is SEATED behind one compact rectangular desk with one OPEN LAPTOP COMPUTER resting flat on the tabletop; the hinged screen panel and keyboard deck are both clearly visible from the fixed camera, and both hands type on the laptop keyboard. The centered tabletop sits at a fixed lower-third, elbow-height position below the character, with the chair and desk fully inside the frame. Across the 8 frames, the desk, chair, laptop screen, laptop keyboard deck, camera, scale, tabletop height, and baseline stay identical; only tiny finger key-press motion advances continuously and the head remains upright without turning",
requirements: "The character MUST sit behind the same centered desk in every frame. The laptop MUST be an open laptop computer with a visible hinged screen panel and visible keyboard deck; it rests on the tabletop, never on the character's lap and never held in the hands. Both hands stay on the laptop keyboard. No standalone keyboard, no desktop keyboard, no keyboard-only output, no tablet, no closed laptop, no laptop on the lap, no handheld laptop, no screen UI, no code, no papers, no symbols, and no floating props. The same desk geometry is fixed lower-third and horizontal, fully inside every source slot, and identical across all 8 frames. Camera, character position, furniture position, scale, and baseline do not translate.",
frame_variation: FrameVariation::Animated,
```

Add `frame_variation: FrameVariation::Animated` to the existing `sleeping` and
`acting_cute` entries without changing their approved actions.

Extend `types.rs`'s existing catalog test with explicit variation assertions:

```rust
assert_eq!(
    state_definition("idle").unwrap().frame_variation,
    FrameVariation::Static
);
for key in ["sleeping", "acting_cute", "working"] {
    assert_eq!(
        state_definition(key).unwrap().frame_variation,
        FrameVariation::Animated,
        "state {key} should use an animated frame contract"
    );
}
```

In `prompts.rs`, import `FrameVariation`, add separate frame-layout clauses,
and use the catalog value when building the row prompt:

```rust
use super::types::{FrameVariation, StateDefinition};

const STATIC_FRAME_CONTRACT: &str = "The 8 columns MUST be stable copies of the same static pose; do not force visible motion or differences between columns. Keep the character, furniture, scale, baseline, and camera fixed.";
const ANIMATED_FRAME_CONTRACT: &str = "The 8 columns MUST show 8 visibly DIFFERENT frames of the same continuous motion; each neighboring column changes only by a small continuous increment and frame 8 loops smoothly back to frame 1.";

fn frame_contract(state: &StateDefinition) -> &'static str {
    match state.frame_variation {
        FrameVariation::Static => STATIC_FRAME_CONTRACT,
        FrameVariation::Animated => ANIMATED_FRAME_CONTRACT,
    }
}
```

Remove the unconditional old paragraph that says every state must have
visibly-different frames, and interpolate `frame_contract(state)` instead. Keep
the existing fixed 2048x256 layout, identity lock, facing lock, and source
reference instructions. Extend both negative-exclusion constants with:

```text
enclosed background fragments, background-colored holes between hair strands or body parts, halos, color spill
```

The final row format must still include `state.action`, `state.requirements`,
`frame_contract(state)`, `ROW_NEGATIVE_EXCLUSIONS`, and the chroma exclusion.

### Step 4: Run the prompt tests and verify the green state

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::prompts -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml generation::types -- --nocapture
```

Expected: all prompt and type tests pass, including the existing UTF-8
truncation and catalog tests. Confirm the working prompt contains no
`laptop or keyboard` allowance and the idle prompt does not contain the old
`breathing loop`/`brief eye blink` action wording.

### Step 5: Commit the prompt contract change

```powershell
git add -- src-tauri/src/commands/generation/types.rs src-tauri/src/commands/generation/prompts.rs
git commit -m "fix: make idle static and require laptop working props"
```

## Task 2: Clean enclosed and diagonal chroma-key gaps

**Files:**

- Modify: `src-tauri/src/commands/generation/sprite.rs:120-274, 574-966`

### Step 1: Write failing matte tests

Add these tests to the existing `sprite.rs` test module. They exercise the
public `remove_chroma_background` helper with synthetic images and fail against
the current edge-only 4-neighbor implementation.

```rust
#[test]
fn removes_a_diagonal_background_gap() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let foreground = Rgba([20, 30, 40, 255]);
    let mut image = RgbaImage::from_pixel(
        8,
        8,
        Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
    );

    // The center pixel at (1, 1) reaches the outer background only through
    // diagonal neighbors; its four orthogonal neighbors are foreground.
    for (x, y) in [(1, 0), (0, 1), (2, 1), (1, 2)] {
        image.put_pixel(x, y, foreground);
    }

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(1, 1).0, [0, 0, 0, 0]);
}

#[test]
fn removes_a_small_enclosed_background_hole_inside_a_foreground_ring() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let foreground = Rgba([20, 30, 40, 255]);
    let mut image = RgbaImage::from_pixel(
        9,
        9,
        Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
    );

    for x in 3..=5 {
        image.put_pixel(x, 3, foreground);
        image.put_pixel(x, 5, foreground);
    }
    for y in 3..=5 {
        image.put_pixel(3, y, foreground);
        image.put_pixel(5, y, foreground);
    }
    image.put_pixel(4, 4, Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]));

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(4, 4).0, [0, 0, 0, 0]);
}

#[test]
fn zeroes_rgb_for_preexisting_transparent_pixels() {
    let key = CHROMA_KEY_CANDIDATES[0];
    let mut image = RgbaImage::from_pixel(
        6,
        6,
        Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
    );
    for x in 1..5 {
        for y in 1..5 {
            image.put_pixel(x, y, Rgba([20, 30, 40, 255]));
        }
    }
    image.put_pixel(3, 3, Rgba([123, 45, 67, 0]));

    remove_chroma_background(&mut image, &key);

    assert_eq!(image.get_pixel(1, 1).0, [0, 0, 0, 0]);
}
```

### Step 2: Run the matte tests and verify the red state

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::sprite -- --nocapture
```

Expected: the diagonal test leaves `(1, 1)` opaque with the current 4-neighbor
walk, and the enclosed-hole test leaves `(4, 4)` opaque because the current
algorithm intentionally follows only edge-connected background. The new
transparent-RGB test also exposes that interior pre-existing transparent pixels
are not visited by the edge queue.

### Step 3: Implement 8-connected traversal and interior-hole cleanup

Replace `push_neighbors` with an 8-connected bounded neighbor enumerator:

```rust
fn push_neighbors(queue: &mut VecDeque<(u32, u32)>, x: u32, y: u32, width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let min_x = x.saturating_sub(1);
    let max_x = x.saturating_add(1).min(width - 1);
    let min_y = y.saturating_sub(1);
    let max_y = y.saturating_add(1).min(height - 1);

    for neighbor_y in min_y..=max_y {
        for neighbor_x in min_x..=max_x {
            if neighbor_x != x || neighbor_y != y {
                queue.push_back((neighbor_x, neighbor_y));
            }
        }
    }
}
```

Extract the shared color-to-alpha calculation so both edge and hole passes use
the same ramp:

```rust
fn background_alpha(
    pixel: &Rgba<u8>,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> Option<u8> {
    if pixel[3] == 0 {
        return Some(0);
    }

    let distance = (squared_rgb_distance([pixel[0], pixel[1], pixel[2]], actual_bg) as f32).sqrt();
    if distance > ramp_end {
        return None;
    }

    if distance <= threshold {
        Some(0)
    } else {
        let ratio = (distance - threshold) / (ramp_end - threshold);
        Some((f32::from(pixel[3]) * ratio).round() as u8)
    }
}

fn apply_background_alpha(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> bool {
    let pixel = *image.get_pixel(x, y);
    let Some(alpha) = background_alpha(&pixel, actual_bg, threshold, ramp_end) else {
        return false;
    };

    if alpha == 0 {
        image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
    } else {
        image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], alpha]));
    }
    true
}

fn is_background_like(
    pixel: &Rgba<u8>,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> bool {
    pixel[3] > 0 && background_alpha(pixel, actual_bg, threshold, ramp_end).is_some()
}
```

Keep `FILL_THRESHOLD = 110.0` and `RAMP_WIDTH = 30.0`, and add
`MAX_INTERIOR_HOLE_PIXELS = 4096`. After the existing edge queue finishes,
scan unprocessed background-like components with 8-connected neighbors. Remove
an interior component when it does not touch an image edge and contains at most
`MAX_INTERIOR_HOLE_PIXELS` pixels:

```rust
fn remove_interior_background_holes(
    image: &mut RgbaImage,
    actual_bg: [u8; 3],
    edge_processed: &[bool],
    threshold: f32,
    ramp_end: f32,
) {
    const MAX_INTERIOR_HOLE_PIXELS: usize = 4096;
    let width = image.width();
    let height = image.height();
    let mut visited = vec![false; (width * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let start_index = (y * width + x) as usize;
            if edge_processed[start_index]
                || visited[start_index]
                || !is_background_like(image.get_pixel(x, y), actual_bg, threshold, ramp_end)
            {
                continue;
            }

            let mut queue = VecDeque::from([(x, y)]);
            let mut region = Vec::new();
            let mut touches_edge = false;
            visited[start_index] = true;

            while let Some((current_x, current_y)) = queue.pop_front() {
                let current_index = (current_y * width + current_x) as usize;
                if edge_processed[current_index]
                    || !is_background_like(
                        image.get_pixel(current_x, current_y),
                        actual_bg,
                        threshold,
                        ramp_end,
                    )
                {
                    continue;
                }

                touches_edge |= current_x == 0
                    || current_y == 0
                    || current_x + 1 == width
                    || current_y + 1 == height;
                region.push((current_x, current_y));

                let mut neighbors = VecDeque::new();
                push_neighbors(
                    &mut neighbors,
                    current_x,
                    current_y,
                    width,
                    height,
                );
                while let Some((neighbor_x, neighbor_y)) = neighbors.pop_front() {
                    let neighbor_index = (neighbor_y * width + neighbor_x) as usize;
                    if !edge_processed[neighbor_index] && !visited[neighbor_index] {
                        visited[neighbor_index] = true;
                        queue.push_back((neighbor_x, neighbor_y));
                    }
                }
            }

            if !touches_edge && region.len() <= MAX_INTERIOR_HOLE_PIXELS {
                for (hole_x, hole_y) in region {
                    apply_background_alpha(
                        image,
                        hole_x,
                        hole_y,
                        actual_bg,
                        threshold,
                        ramp_end,
                    );
                }
            }
        }
    }
}
```

In `remove_chroma_background`, keep the sampled border color and edge queue,
replace the inline distance/ramp block with `apply_background_alpha`, call
`remove_interior_background_holes` after the edge pass, then normalize every
transparent pixel:

The edge-loop body should retain the existing connectivity rule while using the
shared helper:

```rust
let [_, _, _, alpha] = image.get_pixel(x, y).0;
if alpha == 0 {
    image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
    push_neighbors(&mut queue, x, y, width, height);
    continue;
}

if !apply_background_alpha(image, x, y, actual_bg, FILL_THRESHOLD, ramp_end) {
    continue;
}
push_neighbors(&mut queue, x, y, width, height);
```

```rust
remove_interior_background_holes(
    image,
    actual_bg,
    &processed,
    FILL_THRESHOLD,
    ramp_end,
);

for pixel in image.pixels_mut() {
    if pixel[3] == 0 {
        *pixel = Rgba([0, 0, 0, 0]);
    }
}
```

### Step 4: Run the matte tests and verify the green state

```powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::sprite -- --nocapture
```

Expected: all sprite tests pass, including border sampling, near-key alpha
ramp behavior, diagonal connectivity, enclosed hole removal, and transparent
RGB zeroing. If a test fails, change the matte implementation rather than
weakening the assertion.

### Step 5: Commit the matte change

```powershell
git add -- src-tauri/src/commands/generation/sprite.rs
git commit -m "fix: clean enclosed chroma-key gaps"
```

## Task 3: Preserve furniture coordinates with a shared row transform

**Files:**

- Modify: `src-tauri/src/commands/generation/sprite.rs:284-406, 852-934`

### Step 1: Replace the recentering regression test with a failing shared-scene test

Replace the existing test named
`per_frame_slicing_recenters_characters_that_the_ai_placed_off_center` with
this test. It deliberately shifts the desk and character in one source slot;
the desired normalizer must preserve that local source coordinate instead of
cropping each slot to its own visible bounds and moving it back to center.

```rust
#[test]
fn shared_transform_preserves_frame_local_scene_coordinates() {
    let foreground = Rgba([20, 30, 40, 255]);
    let mut source = RgbaImage::new(API_FRAME_W * DEFAULT_FRAME_COUNT, API_FRAME_H);

    for frame_index in 0..DEFAULT_FRAME_COUNT {
        let frame_x = frame_index * API_FRAME_W;
        let desk_start = if frame_index == 3 { 64 } else { 32 };
        let body_start = if frame_index == 3 { 112 } else { 80 };

        for local_x in desk_start..(desk_start + 192) {
            for y in 160..=168 {
                source.put_pixel(frame_x + local_x, y, foreground);
            }
        }
        for local_x in body_start..(body_start + 64) {
            for y in 32..=160 {
                source.put_pixel(frame_x + local_x, y, foreground);
            }
        }
    }
    // Make the global visible bounds cover the complete 8-slot canvas without
    // changing the desk band used by the assertions below.
    source.put_pixel(0, 0, foreground);
    let last_x = source.width() - 1;
    source.put_pixel(last_x, 0, foreground);

    let normalized = super::slice_row_into_frames(&source, DEFAULT_FRAME_COUNT);

    let table_left = |frame_index: u32| {
        let frame_x = frame_index * FRAME_W;
        let desk_y = (96..FRAME_H)
            .max_by_key(|y| {
                (0..FRAME_W)
                    .filter(|local_x| normalized.get_pixel(frame_x + *local_x, *y)[3] != 0)
                    .count()
            })
            .expect("desk row should be visible");
        (0..FRAME_W)
            .find(|local_x| normalized.get_pixel(frame_x + *local_x, desk_y)[3] != 0)
            .expect("desk should be visible")
    };
    let body_center = |frame_index: u32| {
        let frame_x = frame_index * FRAME_W;
        let visible = (0..FRAME_W)
            .filter(|local_x| {
                (56..112).any(|y| normalized.get_pixel(frame_x + *local_x, y)[3] != 0)
            })
            .collect::<Vec<_>>();
        (visible[0] + visible[visible.len() - 1]) / 2
    };

    assert_eq!(table_left(0), 16);
    assert_eq!(table_left(3), 32);
    assert!(body_center(3) > body_center(0) + 8);
}
```

### Step 2: Run the shared-transform test and verify the red state

```powershell
cargo test --manifest-path src-tauri/Cargo.toml generation::sprite -- --nocapture
```

Expected: the new test fails because the current `find_segment_x_bounds` and
per-frame crop/recenter logic makes both desk left edges approximately equal.
The rest of the sprite tests must remain runnable.

### Step 3: Implement one complete source segment and one common scale

In `slice_row_into_frames`, keep the global `find_visible_bounds` lookup and
the shared vertical/horizontal scale calculation, but replace the per-frame
visible-bounds block with complete equal-width slot crops. Use a ceiling segment
width so the final slot cannot exceed the common horizontal scale:

```rust
let segment_w = content_w
    .saturating_add(frame_count.saturating_sub(1))
    .checked_div(frame_count)
    .unwrap_or(1)
    .max(1);

let scale_v = f64::from(FRAME_H) / f64::from(content_h);
let scale_h = f64::from(FRAME_W) / f64::from(segment_w);
let global_scale = scale_v.min(scale_h);
let content_end = x_min.saturating_add(content_w);

for i in 0..frame_count {
    let seg_x_start = x_min.saturating_add(i.saturating_mul(segment_w));
    if seg_x_start >= content_end {
        continue;
    }

    let available_w = (content_end - seg_x_start).min(segment_w);
    let cropped = image::imageops::crop_imm(
        source,
        seg_x_start,
        y_min,
        available_w,
        content_h,
    )
    .to_image();
    let mut segment = RgbaImage::new(segment_w, content_h);
    image::imageops::overlay(&mut segment, &cropped, 0, 0);

    let scaled_w = ((f64::from(segment_w) * global_scale).round() as u32)
        .max(1)
        .min(FRAME_W);
    let scaled_h = ((f64::from(content_h) * global_scale).round() as u32)
        .max(1)
        .min(FRAME_H);
    let scaled = DynamicImage::ImageRgba8(segment)
        .resize_exact(scaled_w, scaled_h, FilterType::Lanczos3)
        .to_rgba8();

    let dst_frame_x = i * FRAME_W;
    let dst_x_offset = (FRAME_W - scaled_w) / 2;
    let dst_y_offset = FRAME_H - scaled_h;
    for (px, py, pixel) in scaled.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        let dx = dst_frame_x + dst_x_offset + px;
        let dy = dst_y_offset + py;
        if dx < dst_width && dy < FRAME_H {
            dst.put_pixel(dx, dy, *pixel);
        }
    }
}
```

Delete `find_segment_x_bounds`; it is no longer used. Update the function
comment to say that every slot uses a shared crop/scale/baseline transform and
that local furniture coordinates are preserved. Do not change
`normalize_horizontal_row`'s order: decode, remove background, then slice.

### Step 4: Run the row tests and verify the green state

```powershell
cargo fmt --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml generation::sprite -- --nocapture
```

Expected: the shared-scene test and all existing row tests pass. In particular,
vertical letterboxing still normalizes to a valid 1024x128 row, the shared
baseline test still reports the same bottom-most visible row, and empty frames
still reach `validate_sprite_row` instead of being fabricated.

### Step 5: Commit the shared-transform change

```powershell
git add -- src-tauri/src/commands/generation/sprite.rs
git commit -m "fix: preserve shared sprite scene coordinates"
```

## Task 4: Full verification and handoff

**Files:** None expected.

### Step 1: Run the complete Rust library tests

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: the full library suite passes with zero test failures. Linker output
may contain the existing Windows `linker_messages` warning; it is not a test
failure.

### Step 2: Run the complete TypeScript test suite

```powershell
npx vitest run
```

Expected: all existing Vitest files pass; no frontend behavior should change.

### Step 3: Run formatting, type-check, and production build checks

```powershell
cargo fmt --all -- --check
npm run build
```

Expected: Rust formatting is clean and `tsc && vite build` exits successfully.

### Step 4: Inspect the final diff and preserve unrelated files

```powershell
git diff --check origin/main...HEAD
git status --short --branch
```

Expected: the diff is clean; the feature history contains only the generation
source changes plus the approved design/plan documents, and `template.webp`
remains an unrelated untracked file. Do not commit or delete it.

## Plan self-review

- The static idle requirement is covered by Task 1's catalog field, prompt
  clauses, and tests; the generic animated wording is no longer contradictory.
- The mandatory open laptop and fixed desk placement are covered by Task 1's
  working action/requirements and prompt assertions.
- Artificial furniture drift is covered by Task 3's shared-transform test and
  implementation, while the model-facing desk contract remains in Task 1.
- Diagonal and enclosed hair-gap cleanup plus zeroed transparent RGB are covered
  by Task 2's tests and matte implementation.
- Existing decode, dimensions, frame validation, border sampling, baseline,
  provider, and frontend behavior are covered by Task 4's full verification.
- No unresolved placeholders, unspecified files, or contradictory field names
  remain in the task sequence: the catalog field is consistently named
  `frame_variation`, and the prompt helper is consistently named
  `frame_contract`.
