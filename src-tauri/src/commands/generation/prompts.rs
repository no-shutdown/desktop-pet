use super::types::{FrameVariation, StateDefinition};

const BASE_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, enclosed background fragments, background-colored holes between hair/body parts, shadow, glow, halos, particles, detached effects, color spill, gradients, vignette, or key color inside the character";
const ROW_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, column divider, slot outline, enclosed background fragments, background-colored holes between hair/body parts, shadow, glow, halos, particles, detached effects, color spill, motion blur, speed lines, dust, stray pixels, gradients, vignette, or key color inside the character";

const STATIC_FRAME_CONTRACT: &str = "The 8 columns MUST be stable copies of the same static pose; do not force visible motion or differences between columns. Keep the character, furniture, scale, baseline, and camera fixed.";
const ANIMATED_FRAME_CONTRACT: &str = "The 8 columns MUST show 8 visibly DIFFERENT frames of the same continuous motion; each neighboring column changes only by a small continuous increment and frame 8 loops smoothly back to frame 1.";

pub fn build_base_prompt(base_description: &str, chroma_hex: &str, chroma_name: &str) -> String {
    let description = truncate_description(base_description);
    let chroma_exclusion = chroma_component_exclusion(chroma_hex);

    format!(
        "Create a game-ready canonical reference image for {description}. Render one centered complete full-body character in a neutral relaxed pose facing forward, occupying roughly 85% of the frame height with both feet planted on a shared ground line near the bottom of the canvas. This base image defines the identity source for every later animation frame: preserve the face, proportions, markings, palette, materials, clothing, accessories, and props exactly so future frames can reference it. Fill the entire canvas edge-to-edge with a single flat {chroma_name} chroma background ({chroma_hex}); no visible borders, dividers, gradients, or vignette; no cropped limbs. {BASE_NEGATIVE_EXCLUSIONS}. {chroma_exclusion}"
    )
}

pub fn build_row_prompt(
    base_description: &str,
    chroma_hex: &str,
    chroma_name: &str,
    state: &StateDefinition,
) -> String {
    let description = truncate_description(base_description);
    let chroma_exclusion = chroma_component_exclusion(chroma_hex);
    let frame_contract = frame_contract(state);

    format!(
        "Create an 8-frame sprite animation cycle of {description} performing this action: {}. Requirements: {}. The attached reference sheet shows 8 tiled copies of the base pose ONLY to establish character identity and canvas size — REPLACE each column with a distinct animation frame; do not preserve the reference pose in any column. Output an image exactly 2048 pixels wide by 256 pixels tall (8:1 aspect ratio) on a flat {chroma_name} chroma background ({chroma_hex}) filling every non-character pixel edge-to-edge. Split the canvas into 8 equal-width columns of 256 pixels each, arranged left-to-right; place exactly one complete full-body pose in each column, horizontally centered inside its column with equal empty margin on both sides; do not draw any column border, divider, grid, gap, or highlight between columns. {} Keep the motion SMALL and CONTINUOUS: between any two neighbouring columns the pose changes by only a small increment of the action (no big jumps between neighbour frames), so the loop plays smoothly rather than as a slideshow of extreme poses. Every frame is shot from the SAME fixed camera at the SAME zoom — the character's absolute horizontal and vertical position on the canvas is IDENTICAL across all 8 frames aside from the small in-place motion the action requires; the character MUST NOT drift, translate, or shift position between frames. All 8 frames share identical scale and feet (or seated hips) aligned to a single shared horizontal ground line at the same vertical position; do not shift the baseline between frames beyond the small motion the action requires. Preserve identity across all 8 frames: face, proportions, markings, palette, materials, clothing, accessories, and props remain unchanged; only the pose changes to advance the animation. FACING DIRECTION LOCK — the character faces {} in every single frame without exception; never mirror, flip, or reverse the body or head orientation between frames, not even partially. {}. {}",
        state.action, state.requirements, frame_contract, state.facing, ROW_NEGATIVE_EXCLUSIONS, chroma_exclusion
    )
}

fn frame_contract(state: &StateDefinition) -> &'static str {
    match state.frame_variation {
        FrameVariation::Static => STATIC_FRAME_CONTRACT,
        FrameVariation::Animated => ANIMATED_FRAME_CONTRACT,
    }
}

fn chroma_component_exclusion(chroma_hex: &str) -> String {
    format!(
        "Never use {chroma_hex} inside any character component, including the body, face, clothing, accessories, props, highlights, particles, glow, and detached effects; no cropped limbs."
    )
}

fn truncate_description(description: &str) -> &str {
    const MAX_DESCRIPTION_BYTES: usize = 300;

    if description.len() <= MAX_DESCRIPTION_BYTES {
        return description;
    }

    let mut end = MAX_DESCRIPTION_BYTES;
    while !description.is_char_boundary(end) {
        end -= 1;
    }
    &description[..end]
}

#[cfg(test)]
mod tests {
    use super::{build_base_prompt, build_row_prompt};
    use crate::commands::generation::types::{state_definition, state_definitions};

    #[test]
    fn idle_prompt_requests_a_static_standing_hold_without_breathing_or_blinking() {
        let state = state_definition("idle").unwrap();
        let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
        let prompt = prompt.to_lowercase();

        for term in [
            "static",
            "stable copies of the same static pose",
            "no breathing",
            "no blinking",
        ] {
            assert!(prompt.contains(term), "missing term: {term}");
        }

        for term in [
            "breathing loop",
            "chest rises",
            "brief eye blink",
            "8 visibly different frames",
        ] {
            assert!(!prompt.contains(term), "unexpected term: {term}");
        }
    }

    #[test]
    fn working_prompt_requires_an_open_laptop_on_a_fixed_desk() {
        let state = state_definition("working").unwrap();
        let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
        let prompt = prompt.to_lowercase();

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
            assert!(prompt.contains(term), "missing term: {term}");
        }

        assert!(!prompt.contains("laptop or keyboard"));
        assert!(!prompt.contains("laptop/keyboard"));
    }

    #[test]
    fn animated_working_prompt_keeps_the_continuous_motion_contract() {
        let state = state_definition("working").unwrap();
        let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
        let prompt = prompt.to_lowercase();

        assert!(prompt.contains("8 visibly different frames"));
        assert!(prompt.contains("small continuous increment"));
        assert!(!prompt.contains("stable copies of the same static pose"));
    }

    #[test]
    fn base_prompt_preserves_identity_and_forbids_production_artifacts() {
        let prompt = build_base_prompt("a tiny orange fox with a blue scarf", "#FF00FF", "magenta");

        for term in [
            "tiny orange fox with a blue scarf",
            "centered",
            "complete full-body",
            "neutral relaxed pose",
            "facing forward",
            "85% of the frame height",
            "feet planted",
            "shared ground line",
            "face",
            "proportions",
            "markings",
            "palette",
            "materials",
            "clothing",
            "accessories",
            "props",
            "flat magenta chroma background",
            "#FF00FF",
            "never use #FF00FF inside any character component",
            "props, highlights",
            "no cropped limbs",
            "extra characters",
            "scenery",
            "text",
            "labels",
            "logos",
            "watermark",
            "UI",
            "grid",
            "border",
            "checkerboard",
            "shadow",
            "glow",
            "particles",
            "detached effects",
            "gradients",
            "vignette",
            "key color inside the character",
        ] {
            assert!(
                prompt.to_lowercase().contains(&term.to_lowercase()),
                "missing term: {term}"
            );
        }
    }

    #[test]
    fn base_prompt_truncates_description_on_a_utf8_boundary() {
        let description = format!("{}终点", "a".repeat(299));
        let prompt = build_base_prompt(&description, "#FF00FF", "magenta");

        assert!(prompt.contains(&"a".repeat(299)));
        assert!(!prompt.contains("终点"));
    }

    #[test]
    fn row_prompt_references_the_canonical_base_and_has_fixed_layout_contract() {
        let state = state_definition("acting_cute").unwrap();
        let prompt = build_row_prompt(
            "a tiny orange fox with a blue scarf",
            "#00FFFF",
            "cyan",
            state,
        );

        assert!(prompt.contains("hands held close to the face or chest"));
        assert!(prompt.contains("No hearts, text, symbols, motion lines"));
        assert!(!prompt.to_lowercase().contains("wave"));

        for term in [
            "8-frame sprite animation cycle",
            "attached reference sheet",
            "8 tiled copies",
            "replace each column",
            "distinct animation frame",
            "left-to-right",
            "flat cyan chroma background",
            "#00FFFF",
            "never use #00FFFF inside any character component",
            "props, highlights",
            "2048 pixels wide by 256 pixels tall",
            "8:1 aspect ratio",
            "8 equal-width columns of 256 pixels each",
            "horizontally centered inside its column",
            "8 visibly different frames",
            "small continuous increment",
            "loops smoothly",
            "shared horizontal ground line",
            "identical scale",
            "facing direction lock",
            "front-facing",
            "never mirror, flip, or reverse",
            "no cropped limbs",
            "preserve identity",
            "column divider",
            "slot outline",
            "motion blur",
            "speed lines",
            "dust",
            "stray pixels",
            "gradients",
            "vignette",
        ] {
            assert!(
                prompt.to_lowercase().contains(&term.to_lowercase()),
                "missing term: {term}"
            );
        }
    }

    #[test]
    fn row_prompt_includes_each_catalog_state_action_and_requirements() {
        for state in state_definitions() {
            let prompt = build_row_prompt("a canonical pet", "#FF00FF", "magenta", state);
            assert!(
                prompt.contains(state.action),
                "missing action for {}",
                state.key
            );
            assert!(
                prompt.contains(state.requirements),
                "missing requirements for {}",
                state.key
            );
        }
    }
}
