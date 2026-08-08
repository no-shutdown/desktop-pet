use super::types::StateDefinition;

const BASE_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, shadow, glow, particles, detached effects, gradients, vignette, or key color inside the character";
const ROW_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, column divider, slot outline, shadow, glow, particles, detached effects, motion blur, speed lines, dust, stray pixels, gradients, vignette, or key color inside the character";

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

    format!(
        "The attached canonical base image is a reference sheet showing 8 identical copies of the same character for {description}, laid out left-to-right in 8 equal-width slots on a flat {chroma_name} chroma background ({chroma_hex}). Replace each of the 8 copies' pose so together they form a single loopable animation cycle, while keeping every other visual property unchanged. Output an image that is exactly 2048 pixels wide by 256 pixels tall (8:1 aspect ratio) on the same flat {chroma_name} chroma background ({chroma_hex}) filling every non-character pixel edge-to-edge. Split the canvas into 8 equal-width columns of 256 pixels each; place exactly one complete full-body pose in each column, horizontally centered inside its column with equal empty margin on both sides; do not draw any visible column border, divider, grid, gap, or highlight between columns. All 8 characters share identical scale and feet aligned to a single shared horizontal ground line at the same vertical position in every frame; do not shift the baseline between frames beyond the small breathing amount required by the animation. Preserve identity exactly: face, proportions, markings, palette, materials, clothing, accessories, and props remain unchanged across all 8 frames; only the pose changes. FACING DIRECTION LOCK — the character faces {} in every single frame without exception; never mirror, flip, or reverse the body or head orientation between frames, not even partially. State action: {}. State requirements: {}. {}. {}",
        state.facing, state.action, state.requirements, ROW_NEGATIVE_EXCLUSIONS, chroma_exclusion
    )
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
        let state = state_definition("walking").unwrap();
        let prompt = build_row_prompt(
            "a tiny orange fox with a blue scarf",
            "#00FFFF",
            "cyan",
            state,
        );

        for term in [
            "attached canonical base image",
            "8 identical copies",
            "left-to-right",
            "flat cyan chroma background",
            "#00FFFF",
            "never use #00FFFF inside any character component",
            "props, highlights",
            "2048 pixels wide by 256 pixels tall",
            "8:1 aspect ratio",
            "8 equal-width columns of 256 pixels each",
            "horizontally centered inside its column",
            "shared horizontal ground line",
            "identical scale",
            "facing direction lock",
            "facing right",
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
