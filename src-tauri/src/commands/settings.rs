use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

fn settings_path(base: &PathBuf) -> PathBuf {
    base.join("settings.json")
}

fn write_position(base: &PathBuf, pos: &WindowPosition) -> Result<(), String> {
    fs::create_dir_all(base).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(pos).map_err(|error| error.to_string())?;
    fs::write(settings_path(base), json).map_err(|error| error.to_string())?;
    Ok(())
}

fn read_position(base: &PathBuf) -> Result<Option<WindowPosition>, String> {
    let path = settings_path(base);
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let position: WindowPosition = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    Ok(Some(position))
}

#[tauri::command]
pub fn save_window_position(app: AppHandle, position: WindowPosition) -> Result<(), String> {
    let base = app.path().app_data_dir().map_err(|error| error.to_string())?;
    write_position(&base, &position)
}

#[tauri::command]
pub fn load_window_position(app: AppHandle) -> Result<Option<WindowPosition>, String> {
    let base = app.path().app_data_dir().map_err(|error| error.to_string())?;
    read_position(&base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn saves_and_loads_position() {
        let dir = TempDir::new().unwrap();
        let pos = WindowPosition { x: 120, y: 340 };
        write_position(&dir.path().to_path_buf(), &pos).unwrap();
        let loaded = read_position(&dir.path().to_path_buf()).unwrap().unwrap();
        assert_eq!(loaded.x, 120);
        assert_eq!(loaded.y, 340);
    }

    #[test]
    fn returns_none_when_settings_missing() {
        let dir = TempDir::new().unwrap();
        let result = read_position(&dir.path().to_path_buf()).unwrap();
        assert!(result.is_none());
    }
}
