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
        action: "gentle in-place breathing loop distributed across the 8 frames; frames 1-4 slowly inhale so the chest and head rise by about 2% of body height, frames 5-8 slowly exhale back to the neutral pose; include one brief eye blink at a single frame; feet stay planted on the shared ground line and the silhouette is nearly unchanged; frame 8 flows seamlessly back into frame 1",
        requirements: "No walking, waving, working, jumping, emotional reactions, large gestures, item interaction, or new props.",
    },
    StateDefinition {
        key: "walking",
        label: "走路",
        delay_ms: 100,
        facing: "right (side-view, character's right side toward screen right — do not flip or mirror any frame)",
        action: "in-place walking cycle in side-view facing right, distributed evenly across the 8 frames as a loopable gait; frames 1 and 5 are contact poses with legs at maximum spread and opposite arms swung, frames 2-4 pass through the mid-stride with the back leg lifting forward, frames 6-8 mirror that leg motion on the other leg; body bobs subtly up and down but the character's absolute horizontal position stays fixed at the frame center; frame 8 flows seamlessly back into frame 1",
        requirements: "No speed lines, dust, shadows, motion trails, or detached effects; the character never translates horizontally between frames.",
    },
    StateDefinition {
        key: "waving",
        label: "挥手",
        delay_ms: 110,
        facing: "forward (front-facing, exactly as in the canonical base image)",
        action: "friendly greeting cycle distributed across the 8 frames; frames 1-2 lift the near arm from the side up to head height, frames 3-6 sway the raised open hand side-to-side in a small arc while the elbow stays roughly in place, frames 7-8 lower the arm back toward the relaxed starting pose; the other arm, torso, and both legs stay still and relaxed; frame 8 flows seamlessly back into frame 1",
        requirements: "No wave marks, motion arcs, lines, sparkles, symbols, or floating effects.",
    },
    StateDefinition {
        key: "working",
        label: "工作",
        delay_ms: 120,
        facing: "forward (front-facing, exactly as in the canonical base image)",
        action: "focused desk-activity loop performed in place, distributed across the 8 frames; small purposeful hand or paw motions such as typing, tapping, or scanning move through a short repeating cycle while the body stays centered; head may tilt slightly during the motion; feet stay planted and the character never translates horizontally; frame 8 flows seamlessly back into frame 1",
        requirements: "Only use props already present in the canonical base. No UI, code, papers, symbols, or detached props.",
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
