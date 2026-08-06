use super::types::StateDefinition;

const BASE_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, shadow, glow, particles, detached effects, or key color inside the character";
const ROW_NEGATIVE_EXCLUSIONS: &str = "no extra characters, scenery, text, labels, logos, watermark, UI, grid, border, checkerboard, shadow, glow, particles, detached effects, motion blur, speed lines, dust, stray pixels, or key color inside the character";

pub fn build_base_prompt(base_description: &str, chroma_hex: &str, chroma_name: &str) -> String {
    let description = truncate_description(base_description);

    format!(
        "Create a concise game-ready canonical reference image for {description}. One centered complete full-body character in a neutral relaxed pose; preserve the face, proportions, markings, palette, materials, clothing, accessories, and props as the identity source for every later animation frame. Use a flat {chroma_name} chroma background ({chroma_hex}). {BASE_NEGATIVE_EXCLUSIONS}."
    )
}

pub fn build_row_prompt(
    base_description: &str,
    chroma_hex: &str,
    chroma_name: &str,
    state: &StateDefinition,
) -> String {
    let description = truncate_description(base_description);

    format!(
        "Using the attached canonical base image as the only identity reference for {description}, create exactly 8 full-body frames left-to-right on a flat {chroma_name} chroma background ({chroma_hex}). Arrange equal-width invisible slots with one centered complete pose per slot; no clipping, no overlap, no empty slots, no labels, and no borders. Keep stable scale and baseline, preserve identity exactly, and preserve the face, proportions, markings, palette, materials, clothing, accessories, and props. State action: {}. State requirements: {}. {}.",
        state.action, state.requirements, ROW_NEGATIVE_EXCLUSIONS
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
            "exactly 8 full-body frames",
            "left-to-right",
            "flat cyan chroma background",
            "#00FFFF",
            "equal-width invisible slots",
            "one centered complete pose per slot",
            "no clipping",
            "no overlap",
            "no empty slots",
            "no labels",
            "no borders",
            "stable scale",
            "baseline",
            "preserve identity",
            "motion blur",
            "speed lines",
            "dust",
            "stray pixels",
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
