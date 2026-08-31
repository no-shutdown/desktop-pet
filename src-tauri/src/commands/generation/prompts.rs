use super::types::{FrameVariation, SourceStyle, StateDefinition, CANONICAL_FACING};

const BASE_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, enclosed background fragments, background-colored holes between hair/body parts, shadow, glow, halos, particles, detached effects, color spill, gradients, vignette, or key color inside the character";
const ROW_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, column divider, slot outline, enclosed background fragments, background-colored holes between hair/body parts, shadow, glow, halos, particles, detached effects, color spill, motion blur, speed lines, dust, stray pixels, gradients, vignette, or key color inside the character";

const STATIC_FRAME_CONTRACT: &str = "STATIC FRAME CONTRACT: all 8 columns are stable copies of the same static pose; keep the same static pose and do not force visible motion or differences between columns.";
const ANIMATED_FRAME_CONTRACT: &str = "ANIMATED FRAME CONTRACT: the 8 columns MUST show 8 visibly DIFFERENT frames of the same continuous motion; each column is a distinct moment, frame 1 loops smoothly back to frame 8, and neighbouring columns change only by a small continuous increment.";
const STATIC_REFERENCE_CONTRACT: &str = "The attached reference sheet shows 8 tiled copies of the base pose ONLY to establish character identity and canvas size; keep every column as a stable copy of that same static pose and do not replace it with a different motion frame.";
const ANIMATED_REFERENCE_CONTRACT: &str = "The attached reference sheet shows 8 tiled copies of the base pose ONLY to establish character identity and canvas size; REPLACE each column with a distinct animation frame and do not preserve the reference pose in any column.";
const REALISTIC_BASE_STYLE_CONTRACT: &str = "SOURCE STYLE CONTRACT: convert the realistic human photo into a cute 2D chibi character in a clean flat illustration style with rounded proportions, simplified shapes, and a charming approachable expression. No photorealistic rendering, no 3D render, no CGI, no realistic skin pores, no plastic materials, and no cinematic lighting.";
const STYLIZED_BASE_STYLE_CONTRACT: &str = "SOURCE STYLE CONTRACT: preserve the original art style and source medium of the reference artwork. Preserve its line quality, proportions, palette, shading, and texture, including its existing cartoon, anime, illustration, or pixel art treatment. Do not restyle, re-render, or convert the artwork into a different medium; no 3D or CGI restyling.";
const REALISTIC_ROW_STYLE_CONTRACT: &str = "ROW STYLE CONTRACT: the canonical base image is already a cute 2D chibi character in a clean flat illustration style derived from the source. Preserve that established cute 2D chibi/flat illustration style, composition, orientation, rounded proportions, simplified shapes, palette, shading, line quality, and front-facing camera relationship across every frame; change only the requested state motion. Do not reinterpret the canonical base as a realistic image; no photorealistic rendering, no 3D render, no CGI, no realistic skin pores, no plastic materials, and no cinematic lighting.";
const STYLIZED_ROW_STYLE_CONTRACT: &str = "ROW STYLE CONTRACT: the canonical base image already preserves the original art medium of the reference artwork. Preserve the canonical base's exact line quality, proportions, palette, shading, and texture, as well as its style, composition, and orientation, including its existing cartoon, anime, illustration, or pixel art treatment across every frame; change only the requested state motion. Do not restyle, re-render, reinterpret, or convert the canonical base into a different medium.";
const BASE_FACING_LOCK: &str = "CANONICAL FACING LOCK: use one fixed forward-facing character orientation for the canonical base; lock the character facing direction, head angle, body orientation, gaze direction, camera relationship, and composition; no mirror, no flip, no turn, no side turn, and no three-quarter view.";
const ROW_FACING_LOCK: &str = "FACING DIRECTION LOCK: match the canonical base exactly in every frame; lock the character facing direction, head angle, body orientation, gaze direction, camera relationship, and composition with no change between frames.";
const STYLE_REFERENCE_CONTRACT: &str = " STYLE REFERENCE CONTRACT: image 1 is the original character identity reference and image 2 is a pure style reference. Preserve image 1's identity, face, clothing, and accessories; borrow only image 2's line quality, palette, materials, shading, proportions, and overall charm. do not copy image 2's subject, clothing, pose, background, composition, props, or text.";

pub fn build_base_prompt(
    base_description: &str,
    source_style: SourceStyle,
    chroma_hex: &str,
    chroma_name: &str,
) -> String {
    build_base_prompt_with_style_reference(
        base_description,
        source_style,
        chroma_hex,
        chroma_name,
        false,
    )
}

pub fn build_base_prompt_with_style_reference(
    base_description: &str,
    source_style: SourceStyle,
    chroma_hex: &str,
    chroma_name: &str,
    has_style_reference: bool,
) -> String {
    let description = truncate_description(base_description);
    let chroma_exclusion = chroma_component_exclusion(chroma_hex);
    let style_contract = base_source_style_contract(source_style);
    let facing_lock = BASE_FACING_LOCK;
    let style_reference_contract = if has_style_reference {
        STYLE_REFERENCE_CONTRACT
    } else {
        ""
    };

    format!(
        "Create a game-ready canonical reference image for {description}. {style_contract} {facing_lock}{style_reference_contract} Render one centered complete full-body character in a neutral relaxed pose facing {CANONICAL_FACING}, occupying roughly 85% of the frame height with both feet planted on a shared ground line near the bottom of the canvas. This base image defines the identity source for every later animation frame: preserve the face, proportions, markings, palette, materials, clothing, accessories, and props exactly so future frames can reference it. Fill the entire canvas edge-to-edge with a single flat {chroma_name} chroma background ({chroma_hex}); no visible borders, dividers, gradients, or vignette; no cropped limbs. {BASE_NEGATIVE_EXCLUSIONS}. {chroma_exclusion}"
    )
}

pub fn build_row_prompt(
    base_description: &str,
    source_style: SourceStyle,
    chroma_hex: &str,
    chroma_name: &str,
    state: &StateDefinition,
) -> String {
    let description = truncate_description(base_description);
    let chroma_exclusion = chroma_component_exclusion(chroma_hex);
    let style_contract = row_source_style_contract(source_style);
    let (intro, reference_contract, frame_contract, motion_contract) =
        match state.frame_variation {
            FrameVariation::Static => (
                "Create an 8-frame sprite sheet",
                STATIC_REFERENCE_CONTRACT,
                STATIC_FRAME_CONTRACT,
                "Keep the character, furniture, scale, baseline, and camera fixed across all columns with no visible motion.",
            ),
            FrameVariation::Animated => (
                "Create an 8-frame sprite animation cycle",
                ANIMATED_REFERENCE_CONTRACT,
                ANIMATED_FRAME_CONTRACT,
                "Keep the motion SMALL and CONTINUOUS: no big jumps between neighbouring frames, so the loop plays smoothly rather than as a slideshow of extreme poses.",
            ),
        };
    let facing_exclusions = facing_exclusions();

    format!(
        "{intro} of {description} performing this action: {}. Requirements: {}. {style_contract} {reference_contract} Output an image exactly 2048 pixels wide by 256 pixels tall (8:1 aspect ratio) on a flat {chroma_name} chroma background ({chroma_hex}) filling every non-character pixel edge-to-edge. Split the canvas into 8 equal-width columns of 256 pixels each, arranged left-to-right; place exactly one complete full-body pose in each column, horizontally centered inside its column with equal empty margin on both sides; do not draw any column border, divider, grid, gap, or highlight between columns. {frame_contract} {motion_contract} Every frame is shot from the SAME fixed camera at the SAME zoom; the character's absolute horizontal and vertical position on the canvas is IDENTICAL across all 8 frames; the character MUST NOT drift, translate, or shift position between frames. All 8 frames share identical scale and feet (or seated hips) aligned to a single shared horizontal ground line at the same vertical position; do not shift the baseline between frames. Preserve identity across all 8 frames: face, proportions, markings, palette, materials, clothing, accessories, and props remain unchanged. {} The character faces {} in every single frame without exception; same camera, same composition, same body orientation, same head angle, and same gaze direction in every frame. {} Never mirror, flip, or reverse the body or head orientation between frames, not even partially. {}. {}",
        state.action,
        state.requirements,
        ROW_FACING_LOCK,
        state.facing,
        facing_exclusions,
        ROW_NEGATIVE_EXCLUSIONS,
        chroma_exclusion
    )
}

fn base_source_style_contract(source_style: SourceStyle) -> &'static str {
    match source_style {
        SourceStyle::Realistic => REALISTIC_BASE_STYLE_CONTRACT,
        SourceStyle::Stylized => STYLIZED_BASE_STYLE_CONTRACT,
    }
}

fn row_source_style_contract(source_style: SourceStyle) -> &'static str {
    match source_style {
        SourceStyle::Realistic => REALISTIC_ROW_STYLE_CONTRACT,
        SourceStyle::Stylized => STYLIZED_ROW_STYLE_CONTRACT,
    }
}

fn facing_exclusions() -> &'static str {
    "Keep the character front-facing. No mirror, no flip, no turn, no side turn, no three-quarter view or change, and no partial reversal of the body or head."
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
    use super::{build_base_prompt, build_base_prompt_with_style_reference, build_row_prompt};
    use crate::commands::generation::types::{
        state_definition, state_definitions, SourceStyle, CANONICAL_FACING,
    };

    #[test]
    fn base_prompt_assigns_identity_to_image_one_and_style_to_image_two() {
        let prompt = build_base_prompt_with_style_reference(
            "a character",
            SourceStyle::Realistic,
            "#FF00FF",
            "magenta",
            true,
        );

        assert!(prompt.contains("image 1 is the original character identity reference"));
        assert!(prompt.contains("image 2 is a pure style reference"));
        assert!(prompt.contains("do not copy image 2's subject"));
        assert!(prompt.contains("no three-quarter view. STYLE REFERENCE CONTRACT:"));

        let prompt_without_style = build_base_prompt_with_style_reference(
            "a character",
            SourceStyle::Realistic,
            "#FF00FF",
            "magenta",
            false,
        );
        assert!(!prompt_without_style.contains("image 2 is a pure style reference"));
    }

    #[test]
    fn realistic_source_style_requires_a_cute_two_dimensional_chibi_contract() {
        let realistic = build_base_prompt(
            "a person with black hair",
            SourceStyle::Realistic,
            "#FF00FF",
            "magenta",
        )
        .to_lowercase();

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
    }

    #[test]
    fn realistic_row_preserves_the_canonical_cute_style_without_repeating_photo_conversion() {
        let base = build_base_prompt(
            "a person with black hair",
            SourceStyle::Realistic,
            "#FF00FF",
            "magenta",
        )
        .to_lowercase();
        let row = build_row_prompt(
            "a canonical cute chibi person",
            SourceStyle::Realistic,
            "#FF00FF",
            "magenta",
            state_definition("working").unwrap(),
        )
        .to_lowercase();

        assert!(base.contains("convert the realistic human photo"));
        for term in [
            "canonical base image is already a cute 2d chibi character",
            "preserve that established cute 2d chibi/flat illustration style",
            "no photorealistic rendering",
            "no 3d render",
            "no cgi",
        ] {
            assert!(row.contains(term), "missing realistic row term: {term}");
        }
        assert!(!row.contains("convert the realistic human photo"));
    }

    #[test]
    fn stylized_source_style_preserves_the_original_art_medium_and_quality() {
        let stylized = build_base_prompt(
            "a pixel-art orange fox",
            SourceStyle::Stylized,
            "#FF00FF",
            "magenta",
        )
        .to_lowercase();

        for term in ["preserve the original art style", "line quality", "pixel art", "no 3d"] {
            assert!(stylized.contains(term), "missing stylized term: {term}");
        }
        assert!(stylized.contains("do not restyle"));
    }

    #[test]
    fn stylized_row_preserves_the_canonical_art_style_without_restylization() {
        let row = build_row_prompt(
            "a canonical pixel-art orange fox",
            SourceStyle::Stylized,
            "#FF00FF",
            "magenta",
            state_definition("acting_cute").unwrap(),
        )
        .to_lowercase();

        for term in [
            "canonical base image already preserves",
            "original art medium",
            "preserve the canonical base's exact line quality",
            "pixel art treatment",
            "do not restyle",
        ] {
            assert!(row.contains(term), "missing stylized row term: {term}");
        }
        assert!(!row.contains("convert the realistic human photo"));
    }

    #[test]
    fn every_state_uses_a_strict_canonical_facing_lock() {
        for state in state_definitions() {
            assert_eq!(state.facing, CANONICAL_FACING, "non-canonical state: {}", state.key);
            let prompt = build_row_prompt(
                "a canonical pet",
                SourceStyle::Stylized,
                "#FF00FF",
                "magenta",
                state,
            )
            .to_lowercase();
            assert!(prompt.contains("facing direction lock"));
            assert!(prompt.contains("no mirror"));
            assert!(prompt.contains("no flip"));
            assert!(prompt.contains("head angle"));
            assert!(prompt.contains("gaze direction"));
            assert!(prompt.contains("camera relationship"));
            assert!(prompt.contains("composition"));
            assert!(prompt.contains("no side turn"));
            assert!(prompt.contains("no partial reversal"));
            assert!(
                prompt.contains("no three-quarter"),
                "missing strict three-quarter prohibition for {}",
                state.key
            );
        }
    }

    #[test]
    fn base_prompt_locks_facing_head_gaze_camera_and_composition() {
        let prompt = build_base_prompt(
            "a canonical pet",
            SourceStyle::Stylized,
            "#FF00FF",
            "magenta",
        )
        .to_lowercase();

        for term in [
            "canonical facing lock",
            "head angle",
            "body orientation",
            "gaze direction",
            "camera relationship",
            "composition",
            "no mirror",
            "no flip",
            "no turn",
            "no three-quarter",
        ] {
            assert!(prompt.contains(term), "missing base facing term: {term}");
        }
    }

    #[test]
    fn idle_prompt_requests_a_static_standing_hold_without_breathing_or_blinking() {
        let state = state_definition("idle").unwrap();
        let prompt = build_row_prompt(
            "a canonical pet",
            SourceStyle::Stylized,
            "#FF00FF",
            "magenta",
            state,
        )
        .to_lowercase();

        for term in ["static", "same static pose", "no breathing", "no blinking"] {
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
    fn working_prompt_requires_a_complete_open_laptop_on_a_fixed_desk() {
        let state = state_definition("working").unwrap();
        let prompt = build_row_prompt(
            "a canonical pet",
            SourceStyle::Stylized,
            "#FF00FF",
            "magenta",
            state,
        )
        .to_lowercase();

        for term in [
            "complete open laptop",
            "open laptop computer",
            "hinged screen panel",
            "keyboard deck",
            "both hands type",
            "same desk geometry",
            "standalone keyboard",
            "keyboard-only",
            "tablet",
            "closed laptop",
            "lap-held",
            "held device",
            "independent monitor",
        ] {
            assert!(prompt.contains(term), "missing working term: {term}");
        }

        for term in [
            "only localized arm, wrist, hand, and finger typing motion may change",
            "head angle, body orientation, gaze direction, and camera relationship are locked and unchanged",
        ] {
            assert!(prompt.contains(term), "missing working lock term: {term}");
        }
        for term in ["head may tilt", "head tilt"] {
            assert!(!prompt.contains(term), "unexpected working term: {term}");
        }
    }

    #[test]
    fn base_prompt_preserves_identity_and_forbids_production_artifacts() {
        let prompt = build_base_prompt(
            "a tiny orange fox with a blue scarf",
            SourceStyle::Stylized,
            "#FF00FF",
            "magenta",
        );

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
        let prompt = build_base_prompt(&description, SourceStyle::Stylized, "#FF00FF", "magenta");

        assert!(prompt.contains(&"a".repeat(299)));
        assert!(!prompt.contains("终点"));
    }

    #[test]
    fn row_prompt_references_the_canonical_base_and_has_fixed_layout_contract() {
        let state = state_definition("acting_cute").unwrap();
        let prompt = build_row_prompt(
            "a tiny orange fox with a blue scarf",
            SourceStyle::Stylized,
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
            "distinct moment",
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
            let prompt = build_row_prompt(
                "a canonical pet",
                SourceStyle::Stylized,
                "#FF00FF",
                "magenta",
                state,
            );
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
