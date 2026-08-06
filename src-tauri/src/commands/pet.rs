use crate::commands::generation::{pet_dir_at, run_dir};
use crate::models::Pet;
use std::fs;
use std::path::{Path, PathBuf};
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

fn read_selected_pngs(
    app_data_dir: &Path,
    run_id: &str,
) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let selected_dir = run_dir(app_data_dir, run_id)?.join("selected");
    ["idle", "walking", "waving", "working"]
        .into_iter()
        .map(|state| {
            let path = selected_dir.join(format!("{state}.png"));
            let bytes = fs::read(&path)
                .map_err(|error| format!("read selected {state} PNG: {error}"))?;
            image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                .map_err(|error| format!("decode selected {state} PNG: {error}"))?;
            Ok((state, bytes))
        })
        .collect()
}

pub(crate) fn save_pet_from_run_at(
    app_data_dir: &Path,
    run_id: &str,
    pet: &Pet,
) -> Result<(), String> {
    let pet_dir = pet_dir_at(app_data_dir, &pet.id)?;
    let selected_pngs = read_selected_pngs(app_data_dir, run_id)?;
    if pet_dir.exists() {
        return Err(format!("pet already exists: {}", pet.id));
    }
    let pet_json = serde_json::to_vec_pretty(pet).map_err(|error| error.to_string())?;
    let pets_root = pet_dir
        .parent()
        .ok_or_else(|| "pet path has no parent directory".to_string())?;
    let pets_root_existed = pets_root.exists();
    fs::create_dir_all(pets_root).map_err(|error| format!("create pets directory: {error}"))?;
    let staging_dir = pets_root.join(format!(".{}-tmp-{}", pet.id, uuid::Uuid::new_v4()));

    let result = (|| {
        fs::create_dir(&staging_dir)
            .map_err(|error| format!("create temporary pet directory: {error}"))?;
        for (state, bytes) in &selected_pngs {
            fs::write(staging_dir.join(format!("{state}.png")), bytes)
                .map_err(|error| format!("stage selected {state} PNG: {error}"))?;
        }
        fs::write(staging_dir.join("pet.json"), pet_json)
            .map_err(|error| format!("stage pet metadata: {error}"))?;
        if pet_dir.exists() {
            return Err(format!("pet already exists: {}", pet.id));
        }
        fs::rename(&staging_dir, &pet_dir)
            .map_err(|error| format!("commit pet directory: {error}"))?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_dir);
        if !pets_root_existed
            && pets_root.is_dir()
            && fs::read_dir(pets_root)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false)
        {
            let _ = fs::remove_dir(&pets_root);
        }
    }
    result
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
pub fn save_pet_from_run(app: AppHandle, run_id: String, pet: Pet) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_pet_from_run_at(&data_dir, &run_id, &pet)?;
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
    use crate::commands::generation::run_dir;
    use crate::models::SpriteStateInfo;
    use std::collections::HashMap;
    use std::fs;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;
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

    fn write_selected_frames(base_dir: &std::path::Path, run_id: &str) {
        let selected_dir = run_dir(base_dir, run_id).unwrap().join("selected");
        fs::create_dir_all(&selected_dir).unwrap();
        let frame = RgbaImage::from_pixel(2, 2, Rgba([20, 30, 40, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(frame)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        for state in ["idle", "walking", "waving", "working"] {
            fs::write(selected_dir.join(format!("{state}.png")), bytes.get_ref()).unwrap();
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

    #[test]
    fn finalizes_selected_frames_and_pet_metadata_into_one_pet_directory() {
        let temp = TempDir::new().unwrap();
        write_selected_frames(temp.path(), "run-1");
        let pet = make_pet("pet-001");

        save_pet_from_run_at(temp.path(), "run-1", &pet).unwrap();

        let pet_dir = temp.path().join("pets/pet-001");
        assert!(pet_dir.join("pet.json").is_file());
        for state in ["idle", "walking", "waving", "working"] {
            assert!(pet_dir.join(format!("{state}.png")).is_file());
        }
        assert_eq!(read_pets_from_dir(&temp.path().join("pets")).unwrap(), vec![pet]);
    }

    #[test]
    fn missing_selected_frame_does_not_create_a_formal_pet() {
        let temp = TempDir::new().unwrap();
        write_selected_frames(temp.path(), "run-1");
        fs::remove_file(
            run_dir(temp.path(), "run-1")
                .unwrap()
                .join("selected/working.png"),
        )
        .unwrap();

        assert!(save_pet_from_run_at(temp.path(), "run-1", &make_pet("pet-001")).is_err());
        assert!(!temp.path().join("pets/pet-001").exists());
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn invalid_selected_png_does_not_create_a_formal_pet() {
        let temp = TempDir::new().unwrap();
        write_selected_frames(temp.path(), "run-1");
        fs::write(
            run_dir(temp.path(), "run-1")
                .unwrap()
                .join("selected/idle.png"),
            b"not a PNG",
        )
        .unwrap();

        assert!(save_pet_from_run_at(temp.path(), "run-1", &make_pet("pet-001")).is_err());
        assert!(!temp.path().join("pets/pet-001").exists());
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn invalid_run_or_pet_ids_are_rejected_before_finalization() {
        let temp = TempDir::new().unwrap();
        let pet = make_pet("pet-001");

        assert!(save_pet_from_run_at(temp.path(), "../run", &pet).is_err());
        assert!(save_pet_from_run_at(temp.path(), "run-1", &make_pet("../pet")).is_err());
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn atomic_finalization_failure_leaves_no_temporary_pet_directory() {
        let temp = TempDir::new().unwrap();
        write_selected_frames(temp.path(), "run-1");
        fs::create_dir_all(temp.path().join("pets/pet-001")).unwrap();
        fs::write(temp.path().join("pets/pet-001/pet.json"), "old").unwrap();

        assert!(save_pet_from_run_at(temp.path(), "run-1", &make_pet("pet-001")).is_err());
        assert_eq!(fs::read_to_string(temp.path().join("pets/pet-001/pet.json")).unwrap(), "old");
        assert_eq!(
            fs::read_dir(temp.path().join("pets"))
                .unwrap()
                .filter_map(Result::ok)
                .count(),
            1
        );
    }
}
