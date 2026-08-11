use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpriteStateInfo {
    pub cols: usize,
    pub rows: usize,
    pub frame_count: usize,
    pub frame_w: u32,
    pub frame_h: u32,
    pub delay_ms: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pet {
    pub id: String,
    pub name: String,
    pub states: HashMap<String, SpriteStateInfo>,
    pub created_at: String,
    pub prompt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sprite_state() -> SpriteStateInfo {
        SpriteStateInfo { cols: 2, rows: 2, frame_count: 4, frame_w: 128, frame_h: 128, delay_ms: 200 }
    }

    fn make_pet() -> Pet {
        let mut states = HashMap::new();
        for s in &["idle", "sleeping", "waving", "working"] {
            states.insert(s.to_string(), make_sprite_state());
        }
        Pet {
            id: "test-id".to_string(),
            name: "My Pet".to_string(),
            states,
            created_at: "2026-08-03T10:00:00Z".to_string(),
            prompt: "anime chibi girl".to_string(),
        }
    }

    #[test]
    fn pet_round_trips_through_json() {
        let pet = make_pet();
        let json = serde_json::to_string(&pet).unwrap();
        let result: Pet = serde_json::from_str(&json).unwrap();
        assert_eq!(pet, result);
    }

    #[test]
    fn pet_has_all_four_states() {
        let pet = make_pet();
        assert!(pet.states.contains_key("idle"));
        assert!(pet.states.contains_key("sleeping"));
        assert!(pet.states.contains_key("waving"));
        assert!(pet.states.contains_key("working"));
    }

    #[test]
    fn sprite_state_info_serializes_camel_case() {
        let info = make_sprite_state();
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("frameCount"));
        assert!(json.contains("frameW"));
        assert!(json.contains("frameH"));
        assert!(json.contains("delayMs"));
    }
}
