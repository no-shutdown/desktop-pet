use std::{collections::BTreeMap, fmt};

pub const FRAME_W: u32 = 128;
pub const FRAME_H: u32 = 128;
pub const API_FRAME_W: u32 = 256;
pub const API_FRAME_H: u32 = 256;
pub const DEFAULT_FRAME_COUNT: u32 = 8;

#[derive(Clone)]
pub struct ProviderConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_model: String,
    pub reference_model: String,
    pub local_sd_url: String,
    pub denoising_strength: f32,
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderConfig")
            .field("provider", &self.provider)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("base_model", &self.base_model)
            .field("reference_model", &self.reference_model)
            .field("local_sd_url", &self.local_sd_url)
            .field("denoising_strength", &self.denoising_strength)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub delay_ms: u32,
    pub facing: &'static str,
    pub action: &'static str,
    pub requirements: &'static str,
}

const STATE_DEFINITIONS: [StateDefinition; 4] = [
    StateDefinition {
        key: "idle",
        label: "待机",
        delay_ms: 150,
        facing: "forward (front-facing, exactly as in the canonical base image)",
        action: "very subtle in-place breathing loop distributed evenly across the 8 frames; frames 1-4 slowly inhale so the chest rises by only about 1-2% of body height, frames 5-8 slowly exhale back to the neutral pose; include one brief eye blink on a single frame; the silhouette is nearly identical across all 8 frames — the arms, legs, hands, head, and feet stay in place, only the chest expands very slightly and eyes blink; each frame differs from its neighbor by only a tiny continuous increment; frame 8 flows seamlessly back into frame 1",
        requirements: "Keep motion tiny. No walking, waving, working, jumping, emotional reactions, arm gestures, head turns, item interaction, or new props. Camera is fixed; the character stays absolutely centered and does not translate horizontally or vertically between frames.",
    },
    StateDefinition {
        key: "walking",
        label: "走路",
        delay_ms: 100,
        facing: "right (strict side-view profile, character's right side toward screen right — every one of the 8 frames must show the EXACT same side-view orientation; NEVER flip, mirror, rotate, or reverse the body, head, or face between frames; if the character has asymmetric markings or clothing, the SAME side of the character is visible in every single frame)",
        action: "small in-place walking cycle in strict side-view profile facing right, distributed evenly across the 8 frames as a loopable gait; frames 1 and 5 are contact poses with legs at a modest spread and opposite arms slightly swung, frames 2-4 pass through mid-stride with the back leg lifting forward, frames 6-8 do the same leg-swap for the other leg while the head, torso, face, and side-view orientation stay IDENTICAL to every other frame; keep arm swings small and body-bob tiny; each frame differs from its neighbor by only a small continuous increment of the gait; the character's absolute horizontal position stays fixed at the frame center — the character walks in place and never translates between frames; frame 8 flows seamlessly back into frame 1",
        requirements: "No speed lines, dust, shadows, motion trails, or detached effects. Never mirror or flip any frame — the character faces the same side (right) in every single frame without exception. Camera is fixed; the character never translates horizontally or vertically between frames.",
    },
    StateDefinition {
        key: "waving",
        label: "挥手",
        delay_ms: 110,
        facing: "forward (front-facing, exactly as in the canonical base image)",
        action: "small friendly greeting cycle distributed evenly across the 8 frames; frames 1-2 lift the near arm from the side up to about shoulder-to-head height, frames 3-6 sway the raised open hand gently side-to-side in a small arc (the elbow stays roughly in place and the hand moves less than one hand-width each way), frames 7-8 lower the arm back toward the relaxed starting pose; the other arm, torso, head, and both legs stay still and relaxed; each frame differs from its neighbor by only a small continuous increment; frame 8 flows seamlessly back into frame 1",
        requirements: "No wave marks, motion arcs, lines, sparkles, symbols, or floating effects. Keep the waving arm's motion contained — do not extend it far from the body. Camera is fixed; the character stays absolutely centered and does not translate horizontally or vertically between frames.",
    },
    StateDefinition {
        key: "working",
        label: "工作",
        delay_ms: 120,
        facing: "forward (front-facing, exactly as in the canonical base image)",
        action: "the character is SEATED on a chair or stool behind a small desk, with a laptop or keyboard placed on the desk in front of them and both hands resting on the keyboard; distributed evenly across the 8 frames, the fingers make tiny up-and-down typing keypress motions while the wrists, arms, elbows, shoulders, torso, head, and lower body stay essentially still; the desk, chair, and laptop/keyboard are present and IDENTICAL in every frame (same position, style, size, and colour); the head may tilt by only 1-2 degrees between frames and never turns; each frame differs from its neighbor by only a small continuous increment; frame 8 flows seamlessly back into frame 1",
        requirements: "The character MUST be sitting behind a desk with a laptop or keyboard on it in every single frame — no standing, walking, waving, or leaving the desk. Keep the desk, chair, and laptop/keyboard visually identical across all 8 frames. No UI, screen content, code, papers, symbols, floating props, or detached effects. Camera is fixed; the character and the furniture stay absolutely centered and do not translate horizontally or vertically between frames.",
    },
];

pub fn state_definitions() -> &'static [StateDefinition] {
    &STATE_DEFINITIONS
}

pub fn state_definition(key: &str) -> Option<&'static StateDefinition> {
    state_definitions().iter().find(|state| state.key == key)
}

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
pub enum ArtifactStatus {
    Pending,
    Generating,
    Complete,
    Failed,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRunManifest {
    pub version: u32,
    pub run_id: String,
    pub base_prompt: String,
    pub provider: String,
    pub base: ArtifactRecord,
    pub states: BTreeMap<String, ArtifactRecord>,
    pub frame_w: u32,
    pub frame_h: u32,
    pub frame_count: u32,
    pub chroma_key: String,
}

impl GenerationRunManifest {
    pub fn new(
        run_id: String,
        provider: String,
        frame_count: u32,
        chroma_key: String,
        base_prompt: String,
    ) -> Self {
        let states = state_definitions()
            .iter()
            .map(|state| {
                (
                    state.key.to_string(),
                    pending_artifact(format!("rows/{}.png", state.key)),
                )
            })
            .collect();

        Self {
            version: 1,
            run_id,
            base_prompt,
            provider,
            base: pending_artifact("base.png".to_string()),
            states,
            frame_w: FRAME_W,
            frame_h: FRAME_H,
            frame_count,
            chroma_key,
        }
    }
}

fn pending_artifact(path: String) -> ArtifactRecord {
    ArtifactRecord {
        status: ArtifactStatus::Pending,
        path,
        attempts: 0,
        error: None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_one_catalog_has_four_states_and_fixed_timing() {
        assert_eq!(
            state_definitions()
                .iter()
                .map(|state| state.key)
                .collect::<Vec<_>>(),
            vec!["idle", "walking", "waving", "working"]
        );
        assert_eq!(state_definition("walking").unwrap().delay_ms, 100);
        assert_eq!(state_definition("working").unwrap().delay_ms, 120);
    }

    #[test]
    fn manifest_round_trip_keeps_retryable_statuses() {
        let manifest = GenerationRunManifest::new(
            "run-1".into(),
            "siliconflow".into(),
            8,
            "#FF00FF".into(),
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

    #[test]
    fn provider_config_debug_redacts_api_key() {
        let config = ProviderConfig {
            provider: "siliconflow".into(),
            api_key: Some("secret-api-key".into()),
            base_model: "base".into(),
            reference_model: "reference".into(),
            local_sd_url: "http://127.0.0.1:7860".into(),
            denoising_strength: 0.55,
        };

        let debug = format!("{config:?}");
        assert!(!debug.contains("secret-api-key"));
        assert!(debug.contains("redacted"));
    }
}
