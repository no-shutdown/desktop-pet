pub mod prompts;
pub mod providers;
pub mod run;
pub mod sprite;
pub mod types;

pub use run::{
    create_run_at, discard_run_at, load_manifest, manifest_path, mark_base_complete,
    mark_base_generating, mark_failed, mark_state_complete, mark_state_generating,
    reset_rows_after_base_retry, run_dir, runs_dir, save_manifest, validate_run_id,
};
pub use types::{AssembleRunPreviewResult, BasePreviewResult, StateRowResult};

use crate::models::SpriteStateInfo;
use base64::Engine as _;
use image::{imageops, DynamicImage, ImageFormat, RgbaImage};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};

use self::prompts::{build_base_prompt, build_row_prompt};
use self::providers::{clamp_denoising_strength, generate_base, generate_row};
use self::run::{
    load_manifest as load_run_manifest, mark_base_complete as complete_base,
    mark_base_generating as begin_base, mark_failed as fail_run_artifact,
    mark_state_complete as complete_state, mark_state_generating as begin_state,
};
use self::sprite::{
    assemble_rows, choose_chroma_key, chroma_key_from_hex, image_to_data_url, normalize_base_image,
    normalize_horizontal_row, validate_sprite_row,
};
use self::types::{
    state_definition, state_definitions, ArtifactStatus, GenerationRunManifest, ProviderConfig,
    StateDefinition, DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W,
};

const DEFAULT_BASE_MODEL: &str = "Tongyi-MAI/Z-Image-Turbo";
const DEFAULT_REFERENCE_MODEL: &str = "Qwen/Qwen-Image-Edit-2509";
const DEFAULT_LOCAL_SD_URL: &str = "http://localhost:7860";
const DEFAULT_DENOISING_STRENGTH: f32 = 0.55;

pub fn validate_state_name(state: &str) -> Result<&'static StateDefinition, String> {
    state_definition(state).ok_or_else(|| format!("unknown generation state: {state}"))
}

pub fn require_base_complete(manifest: &GenerationRunManifest) -> Result<(), String> {
    if manifest.base.status == ArtifactStatus::Complete {
        Ok(())
    } else {
        Err("canonical base must be complete before generating a state".to_string())
    }
}

pub fn require_preview_ready(manifest: &GenerationRunManifest) -> Result<(), String> {
    require_base_complete(manifest)?;
    for state in state_definitions() {
        let artifact = manifest
            .states
            .get(state.key)
            .ok_or_else(|| format!("generation run is missing state: {}", state.key))?;
        if artifact.status != ArtifactStatus::Complete {
            return Err(format!("state {} is not complete", state.key));
        }
    }
    Ok(())
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("resolve application data directory: {error}"))
}

fn provider_name(provider: &str) -> Result<String, String> {
    let provider = provider.trim();
    if provider.is_empty() {
        return Err("image provider is required".to_string());
    }
    match provider {
        "siliconflow" | "localsd" | "local_sd" => Ok(provider.to_string()),
        _ => Err(format!("unsupported image provider: {provider}")),
    }
}

fn option_or_default(value: Option<String>, default: &str) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn provider_config(
    provider: String,
    api_key: Option<String>,
    base_model: Option<String>,
    reference_model: Option<String>,
    local_sd_url: Option<String>,
    denoising_strength: Option<f32>,
) -> ProviderConfig {
    ProviderConfig {
        provider,
        api_key,
        base_model: option_or_default(base_model, DEFAULT_BASE_MODEL),
        reference_model: option_or_default(reference_model, DEFAULT_REFERENCE_MODEL),
        local_sd_url: option_or_default(local_sd_url, DEFAULT_LOCAL_SD_URL),
        denoising_strength: clamp_denoising_strength(
            denoising_strength.unwrap_or(DEFAULT_DENOISING_STRENGTH),
        ),
    }
}

fn decode_image_data_url(value: &str, context: &str) -> Result<Vec<u8>, String> {
    let value = value.trim();
    let (metadata, payload) = value
        .split_once(',')
        .ok_or_else(|| format!("{context} must be a complete data URL"))?;
    let metadata = metadata.to_ascii_lowercase();
    if !metadata.starts_with("data:image/") || !metadata.ends_with(";base64") {
        return Err(format!("{context} must be a base64 image data URL"));
    }
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(format!("{context} has an empty payload"));
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|error| format!("decode {context}: {error}"))
}

fn decode_reference_image(value: &str) -> Result<image::RgbaImage, String> {
    let bytes = decode_image_data_url(value, "reference image")?;
    image::load_from_memory(&bytes)
        .map(|image| image.to_rgba8())
        .map_err(|error| format!("decode reference image: {error}"))
}

fn write_png(path: &Path, image: &RgbaImage) -> Result<(), String> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|error| format!("encode PNG: {error}"))?;
    std::fs::write(path, bytes.into_inner()).map_err(|error| format!("write PNG: {error}"))
}

fn read_png(path: &Path, context: &str) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {context}: {error}"))?;
    image::load_from_memory(&bytes)
        .map(|image| image.to_rgba8())
        .map_err(|error| format!("decode {context}: {error}"))
}

fn redact_secret(error: &str, secret: Option<&str>) -> String {
    secret
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
        .map(|secret| error.replace(secret, "[redacted-secret]"))
        .unwrap_or_else(|| error.to_string())
}

fn mark_command_failed(
    app_data_dir: &Path,
    run_id: &str,
    artifact: &str,
    error: &str,
    api_key: Option<&str>,
) -> String {
    let safe_error = redact_secret(error, api_key);
    let _ = fail_run_artifact(app_data_dir, run_id, artifact, &safe_error);
    safe_error
}

fn emit_progress(app: &tauri::AppHandle, payload: serde_json::Value) -> Result<(), String> {
    app.emit("generation-progress", payload)
        .map_err(|error| format!("emit generation progress: {error}"))
}

fn assemble_preview_rows(rows: &[RgbaImage]) -> Result<RgbaImage, String> {
    assemble_rows(rows, FRAME_W, FRAME_H)
}

#[tauri::command]
pub async fn generate_base_preview(
    app: tauri::AppHandle,
    run_id: Option<String>,
    base_prompt: String,
    reference_data_url: Option<String>,
    image_provider: String,
    image_api_key: Option<String>,
    base_model: Option<String>,
    reference_model: Option<String>,
    local_sd_url: Option<String>,
    denoising_strength: Option<f32>,
) -> Result<BasePreviewResult, String> {
    let data_dir = app_data_dir(&app)?;
    let provider = provider_name(&image_provider)?;
    let reference = reference_data_url
        .as_deref()
        .map(decode_reference_image)
        .transpose()?;
    let selected_key = choose_chroma_key(reference.as_ref());
    let run_id = run_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut manifest = create_run_at(
        &data_dir,
        run_id.clone(),
        provider.clone(),
        base_prompt.clone(),
    )?;
    if manifest.provider != provider {
        return Err("generation run provider cannot be changed during a retry".to_string());
    }
    manifest.base_prompt = base_prompt;
    manifest.chroma_key = selected_key.hex.to_string();
    save_manifest(&data_dir, &manifest)?;
    begin_base(&data_dir, &run_id)?;

    let prompt = build_base_prompt(&manifest.base_prompt, selected_key.hex, selected_key.name);
    let config = provider_config(
        provider,
        image_api_key.clone(),
        base_model,
        reference_model,
        local_sd_url,
        denoising_strength,
    );
    let generated_bytes = match generate_base(&config, &prompt).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(mark_command_failed(
                &data_dir,
                &run_id,
                "base",
                &error,
                image_api_key.as_deref(),
            ));
        }
    };
    let base = match normalize_base_image(&generated_bytes, &selected_key) {
        Ok(base) => base,
        Err(error) => {
            return Err(mark_command_failed(
                &data_dir,
                &run_id,
                "base",
                &error,
                image_api_key.as_deref(),
            ));
        }
    };
    let base_path = run_dir(&data_dir, &run_id)?.join("base.png");
    if let Err(error) = write_png(&base_path, &base) {
        return Err(mark_command_failed(
            &data_dir,
            &run_id,
            "base",
            &error,
            image_api_key.as_deref(),
        ));
    }
    complete_base(&data_dir, &run_id)?;
    emit_progress(
        &app,
        serde_json::json!({
            "runId": run_id,
            "phase": "base",
            "current": 1,
            "total": 1,
        }),
    )?;

    Ok(BasePreviewResult {
        run_id,
        data_url: image_to_data_url(&base)?,
        chroma_key: selected_key.hex.to_string(),
    })
}

#[tauri::command]
pub async fn generate_state_row(
    app: tauri::AppHandle,
    run_id: String,
    state: String,
    image_provider: String,
    image_api_key: Option<String>,
    reference_model: Option<String>,
    local_sd_url: Option<String>,
    denoising_strength: Option<f32>,
) -> Result<StateRowResult, String> {
    let data_dir = app_data_dir(&app)?;
    let state_definition = validate_state_name(&state)?;
    let provider = provider_name(&image_provider)?;
    let manifest = load_run_manifest(&data_dir, &run_id)?;
    if manifest.provider != provider {
        return Err("generation run provider cannot be changed during a retry".to_string());
    }
    require_base_complete(&manifest)?;
    let selected_key = chroma_key_from_hex(&manifest.chroma_key)?;
    let base_path = run_dir(&data_dir, &run_id)?.join("base.png");
    let base = read_png(&base_path, "canonical base image")?;
    if base.dimensions() != (FRAME_W, FRAME_H) {
        return Err("canonical base image has invalid dimensions".to_string());
    }
    let base_data_url = image_to_data_url(&base)?;
    begin_state(&data_dir, &run_id, &state)?;

    let prompt = build_row_prompt(
        &manifest.base_prompt,
        selected_key.hex,
        selected_key.name,
        state_definition,
    );
    let config = provider_config(
        provider,
        image_api_key.clone(),
        None,
        reference_model,
        local_sd_url,
        denoising_strength,
    );
    let generated_bytes = match generate_row(&config, &prompt, &base_data_url).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(mark_command_failed(
                &data_dir,
                &run_id,
                &state,
                &error,
                image_api_key.as_deref(),
            ));
        }
    };
    let row = match normalize_horizontal_row(&generated_bytes, &selected_key) {
        Ok(row) => row,
        Err(error) => {
            return Err(mark_command_failed(
                &data_dir,
                &run_id,
                &state,
                &error,
                image_api_key.as_deref(),
            ));
        }
    };
    if let Err(error) = validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT) {
        return Err(mark_command_failed(
            &data_dir,
            &run_id,
            &state,
            &error,
            image_api_key.as_deref(),
        ));
    }
    let row_path = run_dir(&data_dir, &run_id)?.join(format!("rows/{state}.png"));
    if let Err(error) = write_png(&row_path, &row) {
        return Err(mark_command_failed(
            &data_dir,
            &run_id,
            &state,
            &error,
            image_api_key.as_deref(),
        ));
    }
    complete_state(&data_dir, &run_id, &state)?;
    emit_progress(
        &app,
        serde_json::json!({
            "runId": run_id,
            "phase": "state",
            "state": state,
            "current": 1,
            "total": 1,
        }),
    )?;

    Ok(StateRowResult {
        run_id,
        state,
        data_url: image_to_data_url(&row)?,
        frame_w: FRAME_W,
        frame_h: FRAME_H,
        frame_count: DEFAULT_FRAME_COUNT,
    })
}

#[tauri::command]
pub fn assemble_run_preview(
    app: tauri::AppHandle,
    run_id: String,
) -> Result<AssembleRunPreviewResult, String> {
    let data_dir = app_data_dir(&app)?;
    let manifest = load_run_manifest(&data_dir, &run_id)?;
    require_preview_ready(&manifest)?;
    let mut rows = Vec::with_capacity(state_definitions().len());
    for state in state_definitions() {
        let row = read_png(
            &run_dir(&data_dir, &run_id)?.join(format!("rows/{}.png", state.key)),
            &format!("{} row", state.key),
        )?;
        validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT)?;
        rows.push(row);
    }
    let preview = assemble_preview_rows(&rows)?;
    Ok(AssembleRunPreviewResult {
        run_id,
        data_url: image_to_data_url(&preview)?,
        frame_w: FRAME_W,
        frame_h: FRAME_H,
        frame_count: DEFAULT_FRAME_COUNT,
        row_gap: 0,
    })
}

#[tauri::command]
pub fn discard_generation_run(app: tauri::AppHandle, run_id: String) -> Result<(), String> {
    let data_dir = app_data_dir(&app)?;
    discard_run_at(&data_dir, &run_id)
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let comma_pos = data_url
        .find(',')
        .ok_or_else(|| "invalid data URL: missing comma".to_string())?;
    base64::engine::general_purpose::STANDARD
        .decode(&data_url[comma_pos + 1..])
        .map_err(|error| format!("base64 decode error: {error}"))
}

fn save_sprite_sheet_png(
    pets_dir: &Path,
    pet_id: &str,
    state: &str,
    sheet: &RgbaImage,
) -> Result<(), String> {
    let dir = pets_dir.join(pet_id);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    sheet
        .save(dir.join(format!("{state}.png")))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn save_combined_sprite_sheet(
    app: tauri::AppHandle,
    pet_id: String,
    data_url: String,
    frame_w: u32,
    frame_h: u32,
    row_gap: u32,
    idle_frames: u32,
    walking_frames: u32,
    waving_frames: u32,
    working_frames: u32,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let bytes = decode_data_url(&data_url)?;
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let rgba = image.to_rgba8();
    let pets_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("pets");
    let rows: [(&str, u32, u32); 4] = [
        ("idle", idle_frames, 150),
        ("walking", walking_frames, 100),
        ("waving", waving_frames, 110),
        ("working", working_frames, 120),
    ];
    let mut result = HashMap::new();
    for (row_index, (state, frame_count, delay_ms)) in rows.iter().enumerate() {
        let y_start = row_index as u32 * (frame_h + row_gap);
        let row_width = frame_w
            .checked_mul(*frame_count)
            .ok_or_else(|| format!("{state} frame width overflow"))?;
        if y_start.checked_add(frame_h).is_none() || y_start + frame_h > rgba.height() {
            return Err(format!("image height is insufficient for {state} row"));
        }
        if row_width > rgba.width() {
            return Err(format!("image width is insufficient for {state} row"));
        }
        let row_sheet = imageops::crop_imm(&rgba, 0, y_start, row_width, frame_h).to_image();
        save_sprite_sheet_png(&pets_dir, &pet_id, state, &row_sheet)?;
        result.insert(
            state.to_string(),
            SpriteStateInfo {
                cols: *frame_count as usize,
                rows: 1,
                frame_count: *frame_count as usize,
                frame_w,
                frame_h,
                delay_ms: *delay_ms,
            },
        );
    }
    Ok(result)
}

#[derive(serde::Deserialize)]
pub struct FrameCell {
    pub col: u32,
    pub row: u32,
}

#[tauri::command]
pub async fn save_frame_selections(
    app: tauri::AppHandle,
    pet_id: String,
    data_url: String,
    frame_w: u32,
    frame_h: u32,
    col_gap: u32,
    row_gap: u32,
    idle_cells: Vec<FrameCell>,
    walking_cells: Vec<FrameCell>,
    waving_cells: Vec<FrameCell>,
    working_cells: Vec<FrameCell>,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let bytes = decode_data_url(&data_url)?;
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let rgba = image.to_rgba8();
    let pets_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("pets");
    let state_entries: [(&str, &Vec<FrameCell>, u32); 4] = [
        ("idle", &idle_cells, 150),
        ("walking", &walking_cells, 100),
        ("waving", &waving_cells, 110),
        ("working", &working_cells, 120),
    ];
    let mut result = HashMap::new();
    for (state, cells, delay_ms) in &state_entries {
        if cells.is_empty() {
            return Err(format!("{state} state has no selected frames"));
        }
        let frame_count = cells.len() as u32;
        let width = frame_w
            .checked_mul(frame_count)
            .ok_or_else(|| format!("{state} frame width overflow"))?;
        let mut sheet = RgbaImage::new(width, frame_h);
        for (index, cell) in cells.iter().enumerate() {
            let x_step = frame_w
                .checked_add(col_gap)
                .ok_or_else(|| format!("{state} frame x step overflow"))?;
            let y_step = frame_h
                .checked_add(row_gap)
                .ok_or_else(|| format!("{state} frame y step overflow"))?;
            let src_x = cell
                .col
                .checked_mul(x_step)
                .ok_or_else(|| format!("{state} frame x overflow"))?;
            let src_y = cell
                .row
                .checked_mul(y_step)
                .ok_or_else(|| format!("{state} frame y overflow"))?;
            if src_x.checked_add(frame_w).is_none()
                || src_y.checked_add(frame_h).is_none()
                || src_x + frame_w > rgba.width()
                || src_y + frame_h > rgba.height()
            {
                return Err(format!(
                    "{state} frame {} is outside the source image",
                    index + 1
                ));
            }
            let frame = imageops::crop_imm(&rgba, src_x, src_y, frame_w, frame_h).to_image();
            imageops::replace(&mut sheet, &frame, index as i64 * frame_w as i64, 0);
        }
        save_sprite_sheet_png(&pets_dir, &pet_id, state, &sheet)?;
        result.insert(
            state.to_string(),
            SpriteStateInfo {
                cols: frame_count as usize,
                rows: 1,
                frame_count: frame_count as usize,
                frame_w,
                frame_h,
                delay_ms: *delay_ms,
            },
        );
    }
    Ok(result)
}

#[cfg(test)]
mod command_tests {
    use super::run::create_run_at;
    use super::types::{state_definitions, ArtifactStatus, GenerationRunManifest};
    use super::{
        assemble_preview_rows, require_base_complete, require_preview_ready, validate_state_name,
    };
    use image::{Rgba, RgbaImage};
    use tempfile::TempDir;

    fn test_manifest() -> (TempDir, GenerationRunManifest) {
        let temp = TempDir::new().unwrap();
        let manifest = create_run_at(
            temp.path(),
            "run-1".to_string(),
            "siliconflow".to_string(),
            "a small orange fox".to_string(),
        )
        .unwrap();
        (temp, manifest)
    }

    #[test]
    fn validates_only_catalog_states() {
        assert_eq!(validate_state_name("idle").unwrap().key, "idle");
        assert!(validate_state_name("jumping").is_err());
    }

    #[test]
    fn requires_a_completed_canonical_base_before_state_generation() {
        let (_temp, mut manifest) = test_manifest();
        assert!(require_base_complete(&manifest).is_err());
        manifest.base.status = ArtifactStatus::Complete;
        assert!(require_base_complete(&manifest).is_ok());
    }

    #[test]
    fn preview_requires_all_four_completed_state_rows() {
        let (_temp, mut manifest) = test_manifest();
        manifest.base.status = ArtifactStatus::Complete;
        assert!(require_preview_ready(&manifest).is_err());
        for state in state_definitions() {
            manifest.states.get_mut(state.key).unwrap().status = ArtifactStatus::Complete;
        }
        assert!(require_preview_ready(&manifest).is_ok());
    }

    #[test]
    fn preview_assembly_keeps_catalog_order_and_zero_row_gap() {
        let rows = vec![
            RgbaImage::from_pixel(1024, 128, Rgba([255, 0, 0, 255])),
            RgbaImage::from_pixel(1024, 128, Rgba([0, 255, 0, 255])),
            RgbaImage::from_pixel(1024, 128, Rgba([0, 0, 255, 255])),
            RgbaImage::from_pixel(1024, 128, Rgba([255, 255, 0, 255])),
        ];

        let preview = assemble_preview_rows(&rows).unwrap();

        assert_eq!(preview.dimensions(), (1024, 512));
        assert_eq!(preview.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(preview.get_pixel(0, 128).0, [0, 255, 0, 255]);
        assert_eq!(preview.get_pixel(0, 384).0, [255, 255, 0, 255]);
    }
}
