use std::path::PathBuf;

/// Lists all .js filenames (sorted) in a plugins directory.
pub fn list_plugins_in_dir(plugins_dir: &PathBuf) -> Result<Vec<String>, String> {
    if !plugins_dir.exists() {
        return Ok(vec![]);
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(plugins_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("js") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Reads the content of a single plugin file by name.
pub fn read_plugin_in_dir(plugins_dir: &PathBuf, name: &str) -> Result<String, String> {
    let path = plugins_dir.join(name);
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_plugins(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    use tauri::Manager;
    let plugins_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("plugins");
    list_plugins_in_dir(&plugins_dir)
}

#[tauri::command]
pub async fn read_plugin_file(app: tauri::AppHandle, name: String) -> Result<String, String> {
    use tauri::Manager;
    let plugins_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("plugins");
    read_plugin_in_dir(&plugins_dir, &name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_temp_plugins(files: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }
        dir
    }

    #[test]
    fn list_returns_only_js_files() {
        let dir = make_temp_plugins(&[
            ("foo.js", ""),
            ("bar.js", ""),
            ("readme.txt", ""),
        ]);
        let names = list_plugins_in_dir(&dir.path().to_path_buf()).unwrap();
        assert_eq!(names, vec!["bar.js", "foo.js"]); // sorted
    }

    #[test]
    fn list_returns_empty_when_dir_missing() {
        let dir = PathBuf::from("/tmp/nonexistent-desktop-pet-plugins-xyz");
        let names = list_plugins_in_dir(&dir).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn read_returns_file_content() {
        let dir = make_temp_plugins(&[("myplugin.js", "pet.setState('idle');")]);
        let content = read_plugin_in_dir(&dir.path().to_path_buf(), "myplugin.js").unwrap();
        assert_eq!(content, "pet.setState('idle');");
    }

    #[test]
    fn read_errors_on_missing_file() {
        let dir = make_temp_plugins(&[]);
        let result = read_plugin_in_dir(&dir.path().to_path_buf(), "missing.js");
        assert!(result.is_err());
    }
}
