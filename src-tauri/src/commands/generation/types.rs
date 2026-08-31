use std::{collections::BTreeMap, fmt};

pub const FRAME_W: u32 = 128;
pub const FRAME_H: u32 = 128;
pub const API_FRAME_W: u32 = 256;
pub const API_FRAME_H: u32 = 256;
pub const DEFAULT_FRAME_COUNT: u32 = 8;
pub const CANONICAL_FACING: &str =
    "forward, straight-on, exactly the same camera angle and left-right orientation as the canonical base image";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStyle {
    Realistic,
    #[default]
    Stylized,
}

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
pub enum FrameVariation {
    Static,
    Animated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub delay_ms: u32,
    pub frame_variation: FrameVariation,
    pub facing: &'static str,
    pub action: &'static str,
    pub requirements: &'static str,
}

const STATE_DEFINITIONS: [StateDefinition; 4] = [
    StateDefinition {
        key: "idle",
        label: "待机",
        delay_ms: 150,
        frame_variation: FrameVariation::Static,
        facing: CANONICAL_FACING,
        action: "static standby/static standing hold: stable copies of the same static pose across all 8 columns; keep the character upright, front-facing, centered, and nearly identical in every column with no breathing, no full-body breathing, no body scaling, no swaying, no head movement, no limb movement, and no blinking",
        requirements: "The character stands upright in a front-facing neutral pose with eyes OPEN in every frame. No breathing, no body scaling, no swaying, no head movement, no limb movement, no eye blinking, no translation, no interaction, and no new props. Keep the character centered on the same baseline with stable copies of the same static pose across all 8 columns.",
    },
    StateDefinition {
        key: "sleeping",
        label: "睡觉",
        delay_ms: 200,
        frame_variation: FrameVariation::Animated,
        facing: CANONICAL_FACING,
        action: "the character is SEATED on a chair or stool behind a small desk, slumped forward and ASLEEP with the head and folded arms resting flat on the desktop as a pillow; distributed evenly across the 8 frames, only the shoulders, back, and head-on-arms rise by about 1-2% of body height on inhale (frames 1-4) and fall back on exhale (frames 5-8); the eyes stay closed; the arms, hands, torso, hips, and legs are motionless aside from that tiny breathing rise-and-fall; optionally add a single small drifting Zzz-like head sway or a slow one-frame twitch of one fingertip; each frame differs from its neighbor by only a tiny continuous increment of the breath cycle; frame 8 flows seamlessly back into frame 1",
        requirements: "The character MUST be sitting behind a desk with head and arms resting on the desktop, eyes closed, asleep, in every single frame — no standing, sitting upright, working, or greeting gesture, and do not open the eyes. Keep the desk and chair identical across all 8 frames. No Zzz text symbols, sleep bubbles, dream clouds, papers, laptops, keyboards, or floating props. Camera is fixed; the character and furniture stay absolutely centered and do not translate horizontally or vertically between frames.",
    },
    StateDefinition {
        key: "acting_cute",
        label: "撒娇",
        delay_ms: 110,
        frame_variation: FrameVariation::Animated,
        facing: CANONICAL_FACING,
        action: "a cute, affectionate 8-frame cycle with both hands held close to the face or chest; the head and upper body sway gently left and right in tiny continuous increments, with one brief shy blink on a single frame; keep the character planted in place and finish in a seamless loop",
        requirements: "Both hands stay close to the face or chest in every frame. The motion is small, continuous, and centered: no greeting gesture, large arm lift, jumping, translation, head turn, or change of facing direction. No hearts, text, symbols, motion lines, sparkles, particles, glow, speech bubbles, or other detached effects. Preserve the same character identity, scale, baseline, camera, and background across all 8 frames.",
    },
    StateDefinition {
        key: "working",
        label: "工作",
        delay_ms: 120,
        frame_variation: FrameVariation::Animated,
        facing: CANONICAL_FACING,
        action: "the character is SEATED behind a compact desk in every frame, with one complete open laptop computer centered on the desk; the laptop has a clearly visible hinged screen panel and keyboard deck and rests flat on the tabletop; both hands type on the laptop keyboard in every frame; the desk/table is fixed lower-third at elbow height and has the same desk geometry in all frames; distributed evenly across the 8 frames, only localized arm, wrist, hand, and finger typing motion may change between frames while the elbows, shoulders, torso, head, and lower body stay essentially still; there is no desk/chair/laptop drift or replacement; the head angle, body orientation, gaze direction, and camera relationship are locked and unchanged; each frame differs from its neighbor by only a small continuous increment; frame 8 flows seamlessly back into frame 1",
        requirements: "The character MUST be SEATED behind a desk with one complete open laptop computer in every single frame, awake with eyes OPEN and head UP. The laptop has a clearly visible hinged screen panel and keyboard deck, rests flat on the tabletop, and both hands type on the laptop keyboard. The desk/table is fixed lower-third at elbow height with the same desk geometry in all frames. Explicitly forbid a standalone keyboard, keyboard-only output, tablet, closed laptop, laptop on the character's lap (lap-held), lap-held laptop, held device, or independent monitor substitution. Only localized arm, wrist, hand, and finger motion is allowed; head angle, body orientation, gaze direction, and camera relationship are locked and unchanged. No desk/chair/laptop drift or replacement, no UI, screen content, code, papers, symbols, floating props, or detached effects. Camera is fixed; the character and the furniture stay absolutely centered and do not translate horizontally or vertically between frames.",
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
    #[serde(default)]
    pub source_style: SourceStyle,
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
        source_style: SourceStyle,
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
            source_style,
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
            vec!["idle", "sleeping", "acting_cute", "working"]
        );
        assert_eq!(state_definition("sleeping").unwrap().delay_ms, 200);
        assert_eq!(state_definition("acting_cute").unwrap().delay_ms, 110);
        assert_eq!(state_definition("working").unwrap().delay_ms, 120);
        assert_eq!(state_definition("idle").unwrap().frame_variation, FrameVariation::Static);
        assert_eq!(
            state_definition("sleeping").unwrap().frame_variation,
            FrameVariation::Animated
        );
        assert_eq!(
            state_definition("acting_cute").unwrap().frame_variation,
            FrameVariation::Animated
        );
        assert_eq!(
            state_definition("working").unwrap().frame_variation,
            FrameVariation::Animated
        );

        for state in state_definitions() {
            assert_eq!(state.facing, CANONICAL_FACING, "non-canonical facing: {}", state.key);
        }
    }

    #[test]
    fn manifest_round_trip_keeps_retryable_statuses() {
        let manifest = GenerationRunManifest::new(
            "run-1".into(),
            "siliconflow".into(),
            8,
            "#FF00FF".into(),
            "anime chibi girl".into(),
            SourceStyle::Realistic,
        );
        let json = serde_json::to_string(&manifest).unwrap();
        let decoded: GenerationRunManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.base.status, ArtifactStatus::Pending);
        assert_eq!(decoded.states["idle"].attempts, 0);
        assert_eq!(decoded.frame_w, 128);
        assert_eq!(decoded.frame_h, 128);
        assert_eq!(decoded.source_style, SourceStyle::Realistic);
        assert!(json.contains("\"sourceStyle\":\"realistic\""));
    }

    #[test]
    fn source_style_defaults_to_stylized_and_old_manifests_remain_loadable() {
        assert_eq!(SourceStyle::default(), SourceStyle::Stylized);
        assert_eq!(serde_json::to_string(&SourceStyle::Realistic).unwrap(), "\"realistic\"");
        assert_eq!(serde_json::to_string(&SourceStyle::Stylized).unwrap(), "\"stylized\"");

        let manifest = GenerationRunManifest::new(
            "run-legacy".into(),
            "siliconflow".into(),
            8,
            "#FF00FF".into(),
            "pixel-art fox".into(),
            SourceStyle::Stylized,
        );
        let mut old_value = serde_json::to_value(manifest).unwrap();
        old_value
            .as_object_mut()
            .expect("manifest should serialize as an object")
            .remove("sourceStyle");

        let decoded: GenerationRunManifest = serde_json::from_value(old_value).unwrap();

        assert_eq!(decoded.source_style, SourceStyle::Stylized);
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
