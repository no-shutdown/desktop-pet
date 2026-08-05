use crate::models::Pet;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn pets_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(app_dir.join("pets"))
}

fn write_pet_to_dir(base_dir: &PathBuf, pet: &Pet) -> Result<(), String> {
    let dir = base_dir.join(&pet.id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(pet).map_err(|e| e.to_string())?;
    fs::write(dir.join("pet.json"), json).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_pets_from_dir(base_dir: &PathBuf) -> Result<Vec<Pet>, String> {
    if !base_dir.exists() {
        return Ok(vec![]);
    }
    let mut pets = Vec::new();
    for entry in fs::read_dir(base_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let pet_json = entry.path().join("pet.json");
        if pet_json.exists() {
            let json = fs::read_to_string(&pet_json).map_err(|e| e.to_string())?;
            let pet: Pet = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            pets.push(pet);
        }
    }
    Ok(pets)
}

fn delete_pet_from_dir(base_dir: &PathBuf, pet_id: &str) -> Result<(), String> {
    let dir = base_dir.join(pet_id);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_pet(app: AppHandle, pet: Pet) -> Result<(), String> {
    let dir = pets_dir(&app)?;
    write_pet_to_dir(&dir, &pet)?;
    use tauri::Emitter;
    let _ = app.emit("pet-saved", &pet);
    Ok(())
}

#[tauri::command]
pub fn list_pets(app: AppHandle) -> Result<Vec<Pet>, String> {
    let dir = pets_dir(&app)?;
    read_pets_from_dir(&dir)
}

#[tauri::command]
pub fn delete_pet(app: AppHandle, pet_id: String) -> Result<(), String> {
    let dir = pets_dir(&app)?;
    delete_pet_from_dir(&dir, &pet_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SpriteStateInfo;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_pet(id: &str) -> Pet {
        let mut states = HashMap::new();
        for s in &["idle", "walking", "waving", "working"] {
            states.insert(s.to_string(), SpriteStateInfo {
                cols: 2, rows: 2, frame_count: 4, frame_w: 128, frame_h: 128, delay_ms: 200,
            });
        }
        Pet {
            id: id.to_string(),
            name: "Test Pet".to_string(),
            states,
            created_at: "2026-08-03T10:00:00Z".to_string(),
            prompt: "anime chibi".to_string(),
        }
    }

    #[test]
    fn saves_and_loads_pet() {
        let dir = TempDir::new().unwrap();
        let pet = make_pet("pet-001");
        write_pet_to_dir(&dir.path().to_path_buf(), &pet).unwrap();
        let loaded = read_pets_from_dir(&dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "pet-001");
        assert_eq!(loaded[0].name, "Test Pet");
    }

    #[test]
    fn lists_multiple_pets() {
        let dir = TempDir::new().unwrap();
        write_pet_to_dir(&dir.path().to_path_buf(), &make_pet("pet-a")).unwrap();
        write_pet_to_dir(&dir.path().to_path_buf(), &make_pet("pet-b")).unwrap();
        let loaded = read_pets_from_dir(&dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn returns_empty_when_dir_missing() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nonexistent");
        let loaded = read_pets_from_dir(&missing).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn delete_removes_pet() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().to_path_buf();
        write_pet_to_dir(&base, &make_pet("to-delete")).unwrap();
        delete_pet_from_dir(&base, "to-delete").unwrap();
        let loaded = read_pets_from_dir(&base).unwrap();
        assert!(loaded.is_empty());
    }
}
