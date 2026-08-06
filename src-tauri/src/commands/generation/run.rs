use super::sprite::CHROMA_KEY_CANDIDATES;
use super::types::{
    state_definition, state_definitions, ArtifactRecord, ArtifactStatus, GenerationRunManifest,
    DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W,
};
use std::fs;
use std::path::{Path, PathBuf};

const RUNS_DIR: &str = "runs";
const MANIFEST_FILE: &str = "manifest.json";
const BASE_PATH: &str = "base.png";

fn validate_safe_id(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(format!("{label} contains an invalid path component"));
    }
    Ok(())
}

pub fn validate_run_id(run_id: &str) -> Result<(), String> {
    validate_safe_id(run_id, "generation run id")
}

pub fn validate_pet_id(pet_id: &str) -> Result<(), String> {
    validate_safe_id(pet_id, "pet id")
}

pub fn runs_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(RUNS_DIR)
}

pub fn run_dir(app_data_dir: &Path, run_id: &str) -> Result<PathBuf, String> {
    validate_run_id(run_id)?;
    let root = runs_dir(app_data_dir);
    let candidate = root.join(run_id);
    if !candidate.starts_with(&root) {
        return Err("generation run path escaped the runs directory".to_string());
    }
    Ok(candidate)
}

pub fn manifest_path(app_data_dir: &Path, run_id: &str) -> Result<PathBuf, String> {
    Ok(run_dir(app_data_dir, run_id)?.join(MANIFEST_FILE))
}

pub fn pet_dir_at(app_data_dir: &Path, pet_id: &str) -> Result<PathBuf, String> {
    validate_pet_id(pet_id)?;
    let root = app_data_dir.join("pets");
    let candidate = root.join(pet_id);
    if !candidate.starts_with(&root) {
        return Err("pet path escaped the pets directory".to_string());
    }
    Ok(candidate)
}

fn row_path(app_data_dir: &Path, run_id: &str, state: &str) -> Result<PathBuf, String> {
    if state_definition(state).is_none() {
        return Err(format!("unknown generation state: {state}"));
    }
    Ok(run_dir(app_data_dir, run_id)?
        .join("rows")
        .join(format!("{state}.png")))
}

fn expected_state_path(state: &str) -> String {
    format!("rows/{state}.png")
}

fn validate_manifest(
    manifest: &GenerationRunManifest,
    requested_run_id: &str,
) -> Result<(), String> {
    validate_run_id(requested_run_id)?;
    if manifest.run_id != requested_run_id {
        return Err("generation run manifest id does not match its path".to_string());
    }
    if manifest.base.path != BASE_PATH {
        return Err("generation run manifest contains an invalid base path".to_string());
    }
    if manifest.frame_w != FRAME_W
        || manifest.frame_h != FRAME_H
        || manifest.frame_count != DEFAULT_FRAME_COUNT
    {
        return Err("generation run manifest has unsupported sprite dimensions".to_string());
    }
    for state in state_definitions() {
        let artifact = manifest
            .states
            .get(state.key)
            .ok_or_else(|| format!("generation run manifest is missing state: {}", state.key))?;
        if artifact.path != expected_state_path(state.key) {
            return Err(format!(
                "generation run manifest contains an invalid path for {}",
                state.key
            ));
        }
    }
    if manifest
        .states
        .keys()
        .any(|key| state_definition(key).is_none())
    {
        return Err("generation run manifest contains an unknown state".to_string());
    }
    Ok(())
}

pub fn create_run_at(
    app_data_dir: &Path,
    run_id: impl Into<String>,
    provider: impl Into<String>,
    base_prompt: impl Into<String>,
) -> Result<GenerationRunManifest, String> {
    let run_id = run_id.into();
    let provider = provider.into();
    let base_prompt = base_prompt.into();
    let run = run_dir(app_data_dir, &run_id)?;
    fs::create_dir_all(run.join("rows"))
        .map_err(|error| format!("create generation run: {error}"))?;

    let manifest_file = run.join(MANIFEST_FILE);
    if manifest_file.is_file() {
        return load_manifest(app_data_dir, &run_id);
    }

    let manifest = GenerationRunManifest::new(
        run_id,
        provider,
        DEFAULT_FRAME_COUNT,
        CHROMA_KEY_CANDIDATES[0].hex.to_string(),
        base_prompt,
    );
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn load_manifest(app_data_dir: &Path, run_id: &str) -> Result<GenerationRunManifest, String> {
    let path = manifest_path(app_data_dir, run_id)?;
    let bytes =
        fs::read(&path).map_err(|error| format!("read generation run manifest: {error}"))?;
    let manifest: GenerationRunManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse generation run manifest: {error}"))?;
    validate_manifest(&manifest, run_id)?;
    Ok(manifest)
}

pub fn save_manifest(app_data_dir: &Path, manifest: &GenerationRunManifest) -> Result<(), String> {
    validate_manifest(manifest, &manifest.run_id)?;
    let run = run_dir(app_data_dir, &manifest.run_id)?;
    fs::create_dir_all(run.join("rows"))
        .map_err(|error| format!("create manifest directory: {error}"))?;

    let path = run.join(MANIFEST_FILE);
    let temp_path = run.join(format!(".{MANIFEST_FILE}.tmp-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(manifest)
            .map_err(|error| format!("serialize generation run manifest: {error}"))?;
        fs::write(&temp_path, bytes)
            .map_err(|error| format!("write generation run manifest: {error}"))?;
        if let Err(error) = fs::rename(&temp_path, &path) {
            if path.exists() {
                fs::remove_file(&path).map_err(|remove_error| {
                    format!("replace generation run manifest: {remove_error}")
                })?;
                fs::rename(&temp_path, &path).map_err(|rename_error| {
                    format!("replace generation run manifest: {rename_error}")
                })?;
            } else {
                return Err(format!("commit generation run manifest: {error}"));
            }
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn pending_artifact(path: String) -> ArtifactRecord {
    ArtifactRecord {
        status: ArtifactStatus::Pending,
        path,
        attempts: 0,
        error: None,
    }
}

fn reset_state_artifact(app_data_dir: &Path, run_id: &str, state: &str) -> Result<(), String> {
    let path = row_path(app_data_dir, run_id, state)?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| format!("remove previous {state} row: {error}"))?;
    }
    Ok(())
}

fn reset_rows_in_manifest(
    app_data_dir: &Path,
    manifest: &mut GenerationRunManifest,
) -> Result<(), String> {
    for state in state_definitions() {
        reset_state_artifact(app_data_dir, &manifest.run_id, state.key)?;
        manifest.states.insert(
            state.key.to_string(),
            pending_artifact(expected_state_path(state.key)),
        );
    }
    Ok(())
}

pub fn reset_rows_after_base_retry(
    app_data_dir: &Path,
    run_id: &str,
) -> Result<GenerationRunManifest, String> {
    let mut manifest = load_manifest(app_data_dir, run_id)?;
    reset_rows_in_manifest(app_data_dir, &mut manifest)?;
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn mark_base_generating(
    app_data_dir: &Path,
    run_id: &str,
) -> Result<GenerationRunManifest, String> {
    let mut manifest = load_manifest(app_data_dir, run_id)?;
    let is_retry = manifest.base.attempts > 0
        || manifest.base.status != ArtifactStatus::Pending
        || manifest
            .states
            .values()
            .any(|artifact| artifact.attempts > 0 || artifact.status != ArtifactStatus::Pending);
    if is_retry {
        reset_rows_in_manifest(app_data_dir, &mut manifest)?;
    }
    manifest.base.attempts = manifest
        .base
        .attempts
        .checked_add(1)
        .ok_or_else(|| "base generation attempt count overflowed".to_string())?;
    manifest.base.status = ArtifactStatus::Generating;
    manifest.base.error = None;
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn mark_base_complete(
    app_data_dir: &Path,
    run_id: &str,
) -> Result<GenerationRunManifest, String> {
    let mut manifest = load_manifest(app_data_dir, run_id)?;
    manifest.base.status = ArtifactStatus::Complete;
    manifest.base.error = None;
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn mark_state_generating(
    app_data_dir: &Path,
    run_id: &str,
    state: &str,
) -> Result<GenerationRunManifest, String> {
    let mut manifest = load_manifest(app_data_dir, run_id)?;
    if manifest.base.status != ArtifactStatus::Complete {
        return Err("canonical base must be complete before generating a state".to_string());
    }
    reset_state_artifact(app_data_dir, run_id, state)?;
    let artifact = manifest
        .states
        .get_mut(state)
        .ok_or_else(|| format!("unknown generation state: {state}"))?;
    artifact.attempts = artifact
        .attempts
        .checked_add(1)
        .ok_or_else(|| format!("{state} generation attempt count overflowed"))?;
    artifact.status = ArtifactStatus::Generating;
    artifact.error = None;
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn mark_state_complete(
    app_data_dir: &Path,
    run_id: &str,
    state: &str,
) -> Result<GenerationRunManifest, String> {
    let mut manifest = load_manifest(app_data_dir, run_id)?;
    let artifact = manifest
        .states
        .get_mut(state)
        .ok_or_else(|| format!("unknown generation state: {state}"))?;
    artifact.status = ArtifactStatus::Complete;
    artifact.error = None;
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn mark_failed(
    app_data_dir: &Path,
    run_id: &str,
    artifact_name: &str,
    error: &str,
) -> Result<GenerationRunManifest, String> {
    let mut manifest = load_manifest(app_data_dir, run_id)?;
    let artifact = if artifact_name == "base" {
        &mut manifest.base
    } else {
        manifest
            .states
            .get_mut(artifact_name)
            .ok_or_else(|| format!("unknown generation state: {artifact_name}"))?
    };
    artifact.status = ArtifactStatus::Failed;
    artifact.error = Some(error.to_string());
    save_manifest(app_data_dir, &manifest)?;
    Ok(manifest)
}

pub fn discard_run_at(app_data_dir: &Path, run_id: &str) -> Result<(), String> {
    let run = run_dir(app_data_dir, run_id)?;
    if run.exists() {
        fs::remove_dir_all(run).map_err(|error| format!("discard generation run: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_run(temp: &TempDir) -> GenerationRunManifest {
        create_run_at(
            temp.path(),
            "run-1".to_string(),
            "siliconflow".to_string(),
            "a small orange fox".to_string(),
        )
        .expect("create test run")
    }

    #[test]
    fn creates_manifest_and_expected_run_artifact_paths() {
        let temp = TempDir::new().unwrap();

        let manifest = create_test_run(&temp);

        let run = run_dir(temp.path(), "run-1").unwrap();
        assert_eq!(run, temp.path().join("runs").join("run-1"));
        assert_eq!(
            manifest_path(temp.path(), "run-1").unwrap(),
            run.join("manifest.json")
        );
        assert!(run.join("rows").is_dir());
        assert!(run.join("manifest.json").is_file());
        assert_eq!(manifest.base.path, "base.png");
        assert_eq!(manifest.states["idle"].path, "rows/idle.png");
        assert_eq!(manifest.states["working"].path, "rows/working.png");

        let loaded = load_manifest(temp.path(), "run-1").unwrap();
        assert_eq!(loaded, manifest);

        let manifest_json = fs::read_to_string(run.join("manifest.json")).unwrap();
        assert!(!manifest_json.contains("apiKey"));
        assert!(!manifest_json.contains("secret"));
    }

    #[test]
    fn rejects_run_id_path_traversal_and_non_single_component_paths() {
        let temp = TempDir::new().unwrap();

        for run_id in [
            "",
            ".",
            "..",
            "../escape",
            "nested/run",
            r"nested\run",
            "run..1",
        ] {
            assert!(
                create_run_at(
                    temp.path(),
                    run_id.to_string(),
                    "localsd".to_string(),
                    "pet".to_string(),
                )
                .is_err(),
                "run id should be rejected: {run_id:?}"
            );
        }
    }

    #[test]
    fn tracks_base_and_state_attempts_and_statuses_independently() {
        let temp = TempDir::new().unwrap();
        create_test_run(&temp);

        let manifest = mark_base_generating(temp.path(), "run-1").unwrap();
        assert_eq!(manifest.base.status, ArtifactStatus::Generating);
        assert_eq!(manifest.base.attempts, 1);

        mark_base_complete(temp.path(), "run-1").unwrap();
        let manifest = mark_state_generating(temp.path(), "run-1", "idle").unwrap();
        assert_eq!(manifest.states["idle"].status, ArtifactStatus::Generating);
        assert_eq!(manifest.states["idle"].attempts, 1);
        assert_eq!(manifest.states["walking"].attempts, 0);

        mark_state_complete(temp.path(), "run-1", "idle").unwrap();
        let manifest = mark_state_generating(temp.path(), "run-1", "walking").unwrap();
        assert_eq!(manifest.states["idle"].status, ArtifactStatus::Complete);
        assert_eq!(manifest.states["idle"].attempts, 1);
        assert_eq!(
            manifest.states["walking"].status,
            ArtifactStatus::Generating
        );
        assert_eq!(manifest.states["walking"].attempts, 1);

        let manifest = mark_failed(temp.path(), "run-1", "walking", "row failed").unwrap();
        assert_eq!(manifest.states["walking"].status, ArtifactStatus::Failed);
        assert_eq!(
            manifest.states["walking"].error.as_deref(),
            Some("row failed")
        );
        assert_eq!(manifest.states["idle"].status, ArtifactStatus::Complete);
    }

    #[test]
    fn base_retry_resets_rows_without_touching_pets() {
        let temp = TempDir::new().unwrap();
        create_test_run(&temp);
        mark_base_generating(temp.path(), "run-1").unwrap();
        mark_base_complete(temp.path(), "run-1").unwrap();
        mark_state_generating(temp.path(), "run-1", "idle").unwrap();
        mark_state_complete(temp.path(), "run-1", "idle").unwrap();
        mark_state_generating(temp.path(), "run-1", "walking").unwrap();
        mark_state_complete(temp.path(), "run-1", "walking").unwrap();

        let run = run_dir(temp.path(), "run-1").unwrap();
        fs::write(run.join("base.png"), b"old base").unwrap();
        fs::write(run.join("rows/idle.png"), b"old idle").unwrap();
        fs::write(run.join("rows/walking.png"), b"old walking").unwrap();
        let pets = temp.path().join("pets").join("existing-pet");
        fs::create_dir_all(&pets).unwrap();
        fs::write(pets.join("idle.png"), b"pet data").unwrap();

        let manifest = mark_base_generating(temp.path(), "run-1").unwrap();

        assert_eq!(manifest.base.status, ArtifactStatus::Generating);
        assert_eq!(manifest.base.attempts, 2);
        for state in ["idle", "walking", "waving", "working"] {
            assert_eq!(manifest.states[state].status, ArtifactStatus::Pending);
            assert_eq!(manifest.states[state].attempts, 0);
            assert!(!run.join(format!("rows/{state}.png")).exists());
        }
        assert!(run.join("base.png").exists());
        assert_eq!(fs::read(pets.join("idle.png")).unwrap(), b"pet data");
    }

    #[test]
    fn state_retry_removes_only_requested_row_and_preserves_completed_rows() {
        let temp = TempDir::new().unwrap();
        create_test_run(&temp);
        mark_base_generating(temp.path(), "run-1").unwrap();
        mark_base_complete(temp.path(), "run-1").unwrap();
        mark_state_generating(temp.path(), "run-1", "idle").unwrap();
        mark_state_complete(temp.path(), "run-1", "idle").unwrap();
        mark_state_generating(temp.path(), "run-1", "walking").unwrap();
        mark_state_complete(temp.path(), "run-1", "walking").unwrap();

        let run = run_dir(temp.path(), "run-1").unwrap();
        fs::write(run.join("rows/idle.png"), b"old idle").unwrap();
        fs::write(run.join("rows/walking.png"), b"completed walking").unwrap();

        let manifest = mark_state_generating(temp.path(), "run-1", "idle").unwrap();

        assert_eq!(manifest.base.status, ArtifactStatus::Complete);
        assert_eq!(manifest.base.attempts, 1);
        assert_eq!(manifest.states["idle"].status, ArtifactStatus::Generating);
        assert_eq!(manifest.states["idle"].attempts, 2);
        assert_eq!(manifest.states["walking"].status, ArtifactStatus::Complete);
        assert_eq!(manifest.states["walking"].attempts, 1);
        assert!(!run.join("rows/idle.png").exists());
        assert_eq!(
            fs::read(run.join("rows/walking.png")).unwrap(),
            b"completed walking"
        );
    }

    #[test]
    fn discard_deletes_only_the_requested_run() {
        let temp = TempDir::new().unwrap();
        create_test_run(&temp);
        create_run_at(
            temp.path(),
            "run-2".to_string(),
            "localsd".to_string(),
            "another pet".to_string(),
        )
        .unwrap();
        let pets = temp.path().join("pets").join("keep-me");
        fs::create_dir_all(&pets).unwrap();
        fs::write(pets.join("idle.png"), b"pet data").unwrap();

        discard_run_at(temp.path(), "run-1").unwrap();

        assert!(!temp.path().join("runs/run-1").exists());
        assert!(temp.path().join("runs/run-2/manifest.json").exists());
        assert_eq!(fs::read(pets.join("idle.png")).unwrap(), b"pet data");
        assert!(temp.path().join("runs").is_dir());
    }

    #[test]
    fn retrying_unknown_state_is_rejected_without_mutating_the_manifest() {
        let temp = TempDir::new().unwrap();
        let original = create_test_run(&temp);

        assert!(mark_state_generating(temp.path(), "run-1", "jumping").is_err());
        assert_eq!(load_manifest(temp.path(), "run-1").unwrap(), original);
    }

    #[test]
    fn pet_ids_reject_empty_traversal_and_rooted_forms_but_accept_existing_style_ids() {
        for pet_id in [
            "",
            ".",
            "..",
            "../outside",
            "nested/pet",
            r"nested\pet",
            r"C:\outside",
            "/outside",
        ] {
            assert!(
                validate_pet_id(pet_id).is_err(),
                "pet id should be rejected: {pet_id:?}"
            );
        }

        assert!(validate_pet_id("pet-1234_abcd").is_ok());
    }

    #[test]
    fn pet_dir_at_keeps_valid_ids_inside_the_pets_directory() {
        let temp = TempDir::new().unwrap();

        let path = pet_dir_at(temp.path(), "pet-1234").unwrap();

        assert_eq!(path, temp.path().join("pets/pet-1234"));
        assert!(path.starts_with(temp.path().join("pets")));
    }
}
