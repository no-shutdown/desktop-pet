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
    pub action: &'static str,
    pub requirements: &'static str,
}

const STATE_DEFINITIONS: [StateDefinition; 4] = [
    StateDefinition {
        key: "idle",
        label: "待机",
        delay_ms: 150,
        action: "calm low-distraction resting loop, subtle breathing, tiny blink, slight head or body bob, nearly unchanged silhouette and planted baseline",
        requirements: "No walking, waving, working, jumping, emotional reactions, large gestures, item interaction, or new props.",
    },
    StateDefinition {
        key: "walking",
        label: "走路",
        delay_ms: 100,
        action: "rightward walking cycle, alternating legs and opposite arm swing, clear directional cadence, stable body scale and baseline",
        requirements: "No speed lines, dust, shadows, motion trails, or detached effects.",
    },
    StateDefinition {
        key: "waving",
        label: "挥手",
        delay_ms: 110,
        action: "friendly greeting shown only through the raised paw, hand, wing, or limb",
        requirements: "No wave marks, motion arcs, lines, sparkles, symbols, or floating effects.",
    },
    StateDefinition {
        key: "working",
        label: "工作",
        delay_ms: 120,
        action: "focused active-task processing, typing, thinking, scanning, or purposeful hand/paw motion; not literal foot-running",
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
