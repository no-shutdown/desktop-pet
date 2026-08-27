# Sprite Generation Quality Improvements Design

**Date:** 2026-08-27
**Status:** Approved for implementation planning

## Problem

The current canonical-sprite generation pipeline has three observable quality issues:

1. The `working` state sometimes produces only a standalone keyboard instead of a
   laptop computer.
2. Desk/table geometry can drift between generated frames or be shifted by the
   post-processing crop/re-centering step.
3. Background-colored pixels enclosed by small structures such as hair gaps can
   survive chroma-key removal.

The `idle` state also has an incorrect motion contract: it currently asks the
model for whole-body breathing and a blink, while the desired result is a
stationary standing wait pose.

## Goals

- Make `working` explicitly require one open laptop computer, including both the
  hinged display and keyboard deck, with the laptop resting on a desk and both
  hands typing on that laptop.
- Make the desk a stable scene anchor: centered, below the seated character, at
  a fixed tabletop height, fully inside each frame, and identical across all
  working frames.
- Make `idle` a truly static standing loop: all eight frames use the same pose,
  with no breathing scale, blink, sway, translation, or limb movement.
- Prevent post-processing from introducing furniture drift by applying one
  shared crop/scale transform to every frame in a row instead of independently
  cropping and re-centering each frame.
- Remove both edge-connected background and small enclosed background holes,
  including diagonal hair gaps, while preserving the existing transparent-RGB
  cleanup contract.
- Add focused Rust regression tests for prompt contracts, shared scene
  alignment, enclosed/diagonal background gaps, and existing sprite validation.

## Non-goals

- No new image-generation provider or model selection UI.
- No semantic object detector or external segmentation model.
- No changes to manually uploaded sprite selection or playback timing.
- No attempt to repair arbitrary square/grid outputs from an image model; the
  row pipeline continues to normalize a single horizontal row and report empty
  frames when a valid row cannot be recovered.

## Design

### 1. State prompt contracts

The state catalog remains the single source of state-specific generation rules.
`StateDefinition` will gain a frame-variation contract (static versus animated)
so the generic row prompt does not contradict a static `idle` definition.

`idle` will say that all eight columns are identical or effectively identical
copies of one upright, relaxed, front-facing standing pose. It will explicitly
forbid breathing, body scaling, blinking, swaying, head turns, arm/leg motion,
translation, new props, and desk work.

`working` will be rewritten with these positive constraints:

- The character is seated behind one compact rectangular desk.
- The desk is centered; its horizontal tabletop is below the character at a
  fixed lower-third/elbow-height position, with the desk geometry fully inside
  each 256-pixel source slot.
- One open laptop computer is placed flat on the tabletop. The hinged screen
  panel and keyboard deck must both be visible from the fixed camera.
- Both hands rest on and type on the laptop keyboard.
- The laptop is not on the character's lap and is not being held.
- The desk, chair, laptop, camera, scale, tabletop height, and baseline are
  identical in all eight frames; only tiny finger/key-press motion may vary.

It will explicitly forbid a standalone desktop keyboard, keyboard-only output,
tablet, closed laptop, laptop held in the hands, screen/UI content, papers, and
floating or detached props. The generic row prompt will use the new frame
variation contract: animated states require eight small continuous moments;
static states require eight stable copies and must not be forced to differ.

The common prompt exclusions will also mention enclosed background fragments,
background-color holes between hair strands/body parts, halos, and color spill.

### 2. Stable row layout normalization

`normalize_horizontal_row` will continue to key the decoded image before frame
normalization. The row slicer will retain one global visible-content bounds and
one shared scale calculation, but each destination frame will be built from its
complete equal-width source segment rather than from that segment's own visible
bounds.

The shared transform will:

1. Compute the visible row bounds once.
2. Divide that content width into eight equal source segments.
3. Compute one uniform scale from the global row height and segment width.
4. Resize every complete segment with that same scale.
5. Place every resized segment with the same horizontal slot offset and shared
   bottom baseline.

This preserves the desk/tabletop's relative x/y coordinates and prevents the
post-processor from shifting a frame merely because one frame has a different
visible-width outline. Empty segments remain transparent so the existing row
validator reports the precise frame index.

The generator prompt remains responsible for making the source desk geometry
actually identical. The normalizer only removes a source of artificial drift;
it does not infer desk semantics or move furniture independently.

### 3. Chroma-key matte cleanup

The matte code will keep sampling the actual background color from the border,
but its flood traversal will use 8-connected neighbors so one-pixel diagonal
paths through hair gaps are reachable.

After the edge-connected pass, the code will inspect enclosed connected regions
whose pixels remain within the sampled background-color ramp. Those small
interior background holes will receive the same hard removal/anti-aliased ramp
as edge background. All pixels made fully transparent will be normalized to
`[0, 0, 0, 0]`.

The selected chroma key remains reserved for the background by the prompt
contract. This permits enclosed exact/near-background holes to be removed while
keeping ordinary character colors outside the sampled background ramp. No
global removal of arbitrary colors is introduced.

### 4. Data flow and failure behavior

The existing flow remains:

```text
state catalog
  -> build_row_prompt
  -> provider generates horizontal row
  -> decode PNG
  -> sample border + remove chroma background/holes
  -> shared-transform frame slicing
  -> validate 8 non-empty 128x128 frames
  -> persist row and preview
```

Malformed images still fail during decode. A row with a missing/empty slot still
fails validation. No fallback provider, automatic regeneration, or silent
frame fabrication is added.

## Testing strategy

Focused Rust tests will assert:

- `idle` prompt contains static/no-motion requirements and does not retain the
  old breathing/blink contract.
- `working` prompt requires an open laptop with visible screen and keyboard,
  places it on a fixed desk, and rejects keyboard-only wording.
- Animated and static frame contracts are both represented in generated row
  prompts without contradictory "must be different" instructions.
- A synthetic row with the same desk geometry in every source slot keeps the
  tabletop at the same normalized coordinates after slicing.
- A character-colored off-center object is not independently re-centered by
  the new shared transform.
- Diagonal background connectivity and an enclosed hair-gap-like background
  region are removed, including RGB zeroing for fully transparent pixels.
- Existing dimensions, non-empty-frame, baseline, and border-sampling tests
  continue to pass.

The implementation will run the focused generation tests first, then the full
Rust library suite and the existing Vitest suite before completion.

## Acceptance criteria

- A generated `working` prompt contains no allowance for "keyboard or" as the
  primary prop and clearly requires an open laptop computer on the desk.
- A generated `idle` prompt no longer asks for breathing, body expansion, or a
  blink and describes eight stationary frames.
- Frame normalization uses a common transform for all slots, so furniture is
  not moved by per-frame visible-bounds recentering.
- Hair-gap-like enclosed or diagonal background regions become transparent and
  fully transparent pixels have zero RGB.
- All existing tests plus the new regression tests pass.
