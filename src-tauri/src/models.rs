use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PetFrames {
    pub idle: String,
    pub walking: String,
    pub waving: String,
    pub working: String,
}

// rename_all = "camelCase" makes created_at serialize as createdAt,
// matching the TypeScript Pet interface
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pet {
    pub id: String,
    pub name: String,
    pub frames: PetFrames,
    pub created_at: String,
    pub prompt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pet() -> Pet {
        Pet {
            id: "test-id".to_string(),
            name: "My Pet".to_string(),
            frames: PetFrames {
                idle: "idle.gif".to_string(),
                walking: "walking.gif".to_string(),
                waving: "waving.gif".to_string(),
                working: "working.gif".to_string(),
            },
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
    fn pet_frames_has_all_four_states() {
        let pet = make_pet();
        assert!(!pet.frames.idle.is_empty());
        assert!(!pet.frames.walking.is_empty());
        assert!(!pet.frames.waving.is_empty());
        assert!(!pet.frames.working.is_empty());
    }
}
