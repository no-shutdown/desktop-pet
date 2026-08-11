pub mod prompts;
pub mod providers;
pub mod run;
pub mod sprite;
pub mod types;

pub use run::{
    create_run_at, discard_run_at, load_manifest, manifest_path, mark_base_complete,
    mark_base_generating, mark_failed, mark_state_complete, mark_state_generating, pet_dir_at,
    reset_rows_after_base_retry, run_dir, runs_dir, save_manifest, validate_pet_id,
    validate_run_id,
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
    assemble_rows, build_row_reference, choose_chroma_key, chroma_key_from_hex,
    ensure_wanxiang_reference_size, image_to_data_url, normalize_base_image,
    normalize_horizontal_row, validate_sprite_row, ChromaKey,
};
use self::types::{
    state_definition, state_definitions, ArtifactStatus, GenerationRunManifest, ProviderConfig,
    StateDefinition, API_FRAME_H, API_FRAME_W, DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W,
};

const DEFAULT_BASE_MODEL: &str = "Tongyi-MAI/Z-Image-Turbo";
const DEFAULT_REFERENCE_MODEL: &str = "Qwen/Qwen-Image-Edit-2509";
const DEFAULT_LOCAL_SD_URL: &str = "http://localhost:7860";
const DEFAULT_DENOISING_STRENGTH: f32 = 0.55;
const MAX_RUN_ERROR_BYTES: usize = 512;

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
        "siliconflow" | "wanxiang" | "localsd" | "local_sd" => Ok(provider.to_string()),
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

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn bounded_run_error(error: &str, secret: Option<&str>) -> String {
    truncate_utf8(&redact_secret(error, secret), MAX_RUN_ERROR_BYTES)
}

fn mark_command_failed(
    app_data_dir: &Path,
    run_id: &str,
    artifact: &str,
    error: &str,
    api_key: Option<&str>,
) -> String {
    let safe_error = bounded_run_error(error, api_key);
    let _ = fail_run_artifact(app_data_dir, run_id, artifact, &safe_error);
    safe_error
}

pub(crate) fn chroma_key_for_manifest(
    manifest: &GenerationRunManifest,
) -> Result<ChromaKey, String> {
    chroma_key_from_hex(&manifest.chroma_key)
}

pub(crate) fn generation_progress_payload(
    run_id: &str,
    phase: &str,
    state: Option<&str>,
    current: u32,
    total: u32,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "runId": run_id,
        "phase": phase,
        "current": current,
        "total": total,
    });
    if let Some(state) = state {
        payload["state"] = serde_json::Value::String(state.to_string());
    }
    payload
}

fn emit_progress(app: &tauri::AppHandle, payload: serde_json::Value) -> Result<(), String> {
    app.emit("generation-progress", payload)
        .map_err(|error| format!("emit generation progress: {error}"))
}

fn assemble_preview_rows(rows: &[RgbaImage]) -> Result<RgbaImage, String> {
    assemble_rows(rows, FRAME_W, FRAME_H)
}

pub(crate) fn finish_base_result_at(
    app_data_dir: &Path,
    run_id: &str,
    selected_key: &ChromaKey,
    provider_result: Result<Vec<u8>, String>,
    api_key: Option<&str>,
) -> Result<RgbaImage, String> {
    let generated_bytes = match provider_result {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(mark_command_failed(
                app_data_dir,
                run_id,
                "base",
                &error,
                api_key,
            ));
        }
    };
    let base = match normalize_base_image(&generated_bytes, selected_key) {
        Ok(base) => base,
        Err(error) => {
            return Err(mark_command_failed(
                app_data_dir,
                run_id,
                "base",
                &error,
                api_key,
            ));
        }
    };
    let base_path = run_dir(app_data_dir, run_id)?.join("base.png");
    if let Err(error) = write_png(&base_path, &base) {
        return Err(mark_command_failed(
            app_data_dir,
            run_id,
            "base",
            &error,
            api_key,
        ));
    }
    if let Err(error) = complete_base(app_data_dir, run_id) {
        return Err(mark_command_failed(
            app_data_dir,
            run_id,
            "base",
            &error,
            api_key,
        ));
    }
    Ok(base)
}

pub(crate) fn finish_state_result_at(
    app_data_dir: &Path,
    run_id: &str,
    state: &str,
    selected_key: &ChromaKey,
    provider_result: Result<Vec<u8>, String>,
    api_key: Option<&str>,
) -> Result<RgbaImage, String> {
    let generated_bytes = match provider_result {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(mark_command_failed(
                app_data_dir,
                run_id,
                state,
                &error,
                api_key,
            ));
        }
    };
    let row = match normalize_horizontal_row(&generated_bytes, selected_key) {
        Ok(row) => row,
        Err(error) => {
            return Err(mark_command_failed(
                app_data_dir,
                run_id,
                state,
                &error,
                api_key,
            ));
        }
    };
    if let Err(error) = validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT) {
        return Err(mark_command_failed(
            app_data_dir,
            run_id,
            state,
            &error,
            api_key,
        ));
    }
    let row_path = run_dir(app_data_dir, run_id)?.join(format!("rows/{state}.png"));
    if let Err(error) = write_png(&row_path, &row) {
        return Err(mark_command_failed(
            app_data_dir,
            run_id,
            state,
            &error,
            api_key,
        ));
    }
    if let Err(error) = complete_state(app_data_dir, run_id, state) {
        return Err(mark_command_failed(
            app_data_dir,
            run_id,
            state,
            &error,
            api_key,
        ));
    }
    Ok(row)
}

pub(crate) fn assemble_run_preview_at(
    app_data_dir: &Path,
    run_id: &str,
) -> Result<AssembleRunPreviewResult, String> {
    let manifest = load_run_manifest(app_data_dir, run_id)?;
    require_preview_ready(&manifest)?;
    let mut rows = Vec::with_capacity(state_definitions().len());
    for state in state_definitions() {
        let row = read_png(
            &run_dir(app_data_dir, run_id)?.join(format!("rows/{}.png", state.key)),
            &format!("{} row", state.key),
        )?;
        validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT)?;
        rows.push(row);
    }
    let preview = assemble_preview_rows(&rows)?;
    Ok(AssembleRunPreviewResult {
        run_id: run_id.to_string(),
        data_url: image_to_data_url(&preview)?,
        frame_w: FRAME_W,
        frame_h: FRAME_H,
        frame_count: DEFAULT_FRAME_COUNT,
        row_gap: 0,
    })
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
    let base = finish_base_result_at(
        &data_dir,
        &run_id,
        &selected_key,
        generate_base(&config, &prompt).await,
        image_api_key.as_deref(),
    )?;
    emit_progress(
        &app,
        generation_progress_payload(&run_id, "base", None, 1, 1),
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
    // Rows may use a different provider than the base (e.g. base via Wanxiang, rows via
    // SiliconFlow) — do not enforce a provider match here. The base's provider stays
    // locked to whatever created the base.png.
    require_base_complete(&manifest)?;
    let selected_key = chroma_key_for_manifest(&manifest)?;
    let base_path = run_dir(&data_dir, &run_id)?.join("base.png");
    let base = read_png(&base_path, "canonical base image")?;
    if base.dimensions() != (API_FRAME_W, API_FRAME_H) {
        return Err("canonical base image has invalid dimensions".to_string());
    }
    let row_reference = build_row_reference(&base);
    let row_reference_sized = if provider == "wanxiang" {
        ensure_wanxiang_reference_size(&row_reference, &selected_key)
    } else {
        row_reference
    };
    let row_reference_data_url = image_to_data_url(&row_reference_sized)?;
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
    let row = finish_state_result_at(
        &data_dir,
        &run_id,
        &state,
        &selected_key,
        generate_row(&config, &prompt, &row_reference_data_url).await,
        image_api_key.as_deref(),
    )?;
    emit_progress(
        &app,
        generation_progress_payload(&run_id, "state", Some(&state), 1, 1),
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
    assemble_run_preview_at(&data_dir, &run_id)
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

fn save_sprite_sheet_png(pet_dir: &Path, state: &str, sheet: &RgbaImage) -> Result<(), String> {
    std::fs::create_dir_all(pet_dir).map_err(|error| error.to_string())?;
    sheet
        .save(pet_dir.join(format!("{state}.png")))
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
    sleeping_frames: u32,
    waving_frames: u32,
    working_frames: u32,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let bytes = decode_data_url(&data_url)?;
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let rgba = image.to_rgba8();
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let pet_dir = pet_dir_at(&data_dir, &pet_id)?;
    let rows: [(&str, u32, u32); 4] = [
        ("idle", idle_frames, 150),
        ("sleeping", sleeping_frames, 200),
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
        save_sprite_sheet_png(&pet_dir, state, &row_sheet)?;
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

fn write_frame_selections_to_dir(
    destination_dir: &Path,
    data_url: &str,
    frame_w: u32,
    frame_h: u32,
    col_gap: u32,
    row_gap: u32,
    idle_cells: &[FrameCell],
    sleeping_cells: &[FrameCell],
    waving_cells: &[FrameCell],
    working_cells: &[FrameCell],
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let bytes = decode_data_url(data_url)?;
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let rgba = image.to_rgba8();
    let state_entries: [(&str, &[FrameCell], u32); 4] = [
        ("idle", idle_cells, 150),
        ("sleeping", sleeping_cells, 200),
        ("waving", waving_cells, 110),
        ("working", working_cells, 120),
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
        save_sprite_sheet_png(destination_dir, state, &sheet)?;
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

pub(crate) fn stage_frame_selections_at(
    app_data_dir: &Path,
    run_id: &str,
    data_url: &str,
    frame_w: u32,
    frame_h: u32,
    col_gap: u32,
    row_gap: u32,
    idle_cells: Vec<FrameCell>,
    sleeping_cells: Vec<FrameCell>,
    waving_cells: Vec<FrameCell>,
    working_cells: Vec<FrameCell>,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let selected_dir = run_dir(app_data_dir, run_id)?.join("selected");
    write_frame_selections_to_dir(
        &selected_dir,
        data_url,
        frame_w,
        frame_h,
        col_gap,
        row_gap,
        &idle_cells,
        &sleeping_cells,
        &waving_cells,
        &working_cells,
    )
}

#[tauri::command]
pub async fn stage_frame_selections(
    app: tauri::AppHandle,
    run_id: String,
    data_url: String,
    frame_w: u32,
    frame_h: u32,
    col_gap: u32,
    row_gap: u32,
    idle_cells: Vec<FrameCell>,
    sleeping_cells: Vec<FrameCell>,
    waving_cells: Vec<FrameCell>,
    working_cells: Vec<FrameCell>,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    stage_frame_selections_at(
        &data_dir,
        &run_id,
        &data_url,
        frame_w,
        frame_h,
        col_gap,
        row_gap,
        idle_cells,
        sleeping_cells,
        waving_cells,
        working_cells,
    )
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
    sleeping_cells: Vec<FrameCell>,
    waving_cells: Vec<FrameCell>,
    working_cells: Vec<FrameCell>,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let pet_dir = pet_dir_at(&data_dir, &pet_id)?;
    write_frame_selections_to_dir(
        &pet_dir,
        &data_url,
        frame_w,
        frame_h,
        col_gap,
        row_gap,
        &idle_cells,
        &sleeping_cells,
        &waving_cells,
        &working_cells,
    )
}

#[cfg(test)]
mod command_tests {
    use super::run::create_run_at;
    use super::run_dir;
    use super::sprite::CHROMA_KEY_CANDIDATES;
    use super::types::{
        state_definitions, ArtifactStatus, GenerationRunManifest, DEFAULT_FRAME_COUNT, FRAME_H,
        FRAME_W,
    };
    use super::{
        assemble_preview_rows, assemble_run_preview_at, chroma_key_for_manifest,
        finish_base_result_at, finish_state_result_at, generation_progress_payload,
        require_base_complete, require_preview_ready, stage_frame_selections_at,
        validate_state_name, FrameCell,
    };
    use base64::Engine as _;
    use image::{GenericImageView, ImageFormat, Rgba, RgbaImage};
    use std::fs;
    use std::io::Cursor;
    use tempfile::TempDir;

    fn png_bytes(image: &RgbaImage) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image.clone())
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn opaque_row(color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H, Rgba(color))
    }

    fn keyed_row(key: super::sprite::ChromaKey) -> RgbaImage {
        // A raw AI-style row: DEFAULT_FRAME_COUNT chroma-keyed columns each with
        // a small solid character blob. Blobs are large enough (16×64) that
        // per-frame slicing preserves visible pixels in every output frame.
        let mut row = RgbaImage::from_pixel(
            FRAME_W * DEFAULT_FRAME_COUNT,
            FRAME_H,
            Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
        );
        for frame in 0..DEFAULT_FRAME_COUNT {
            let x_start = frame * FRAME_W + 56;
            for x in x_start..(x_start + 16) {
                for y in 32..96 {
                    row.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
        }
        row
    }

    fn data_url(image: &RgbaImage) -> String {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png_bytes(image))
        )
    }

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
            opaque_row([255, 0, 0, 255]),
            opaque_row([0, 255, 0, 255]),
            opaque_row([0, 0, 255, 255]),
            opaque_row([255, 255, 0, 255]),
        ];

        let preview = assemble_preview_rows(&rows).unwrap();

        assert_eq!(
            preview.dimensions(),
            (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H * 4)
        );
        assert_eq!(preview.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(preview.get_pixel(0, FRAME_H).0, [0, 255, 0, 255]);
        assert_eq!(preview.get_pixel(0, FRAME_H * 3).0, [255, 255, 0, 255]);
    }

    #[test]
    fn stages_selected_frames_under_the_run_without_creating_pets() {
        let temp = TempDir::new().unwrap();
        let source = RgbaImage::from_pixel(2, 2, Rgba([20, 30, 40, 255]));
        let cell = || vec![FrameCell { col: 0, row: 0 }];

        let result = stage_frame_selections_at(
            temp.path(),
            "manual-run",
            &data_url(&source),
            2,
            2,
            0,
            0,
            cell(),
            cell(),
            cell(),
            cell(),
        )
        .unwrap();

        assert_eq!(result["idle"].frame_count, 1);
        for state in ["idle", "sleeping", "waving", "working"] {
            assert!(
                run_dir(temp.path(), "manual-run")
                    .unwrap()
                    .join(format!("selected/{state}.png"))
                    .is_file()
            );
        }
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn discards_manifestless_external_staging_runs() {
        let temp = TempDir::new().unwrap();
        let run = run_dir(temp.path(), "external-run").unwrap();
        fs::create_dir_all(run.join("selected")).unwrap();
        fs::write(run.join("selected/idle.png"), b"staged").unwrap();

        super::run::discard_run_at(temp.path(), "external-run").unwrap();

        assert!(!run.exists());
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn base_provider_failure_marks_only_base_failed_with_bounded_error_and_no_pets() {
        let temp = TempDir::new().unwrap();
        create_run_at(temp.path(), "run-1", "siliconflow", "pet").unwrap();
        super::mark_base_generating(temp.path(), "run-1").unwrap();
        let provider_error = format!("{}secret-key", "provider failure ".repeat(80));

        let error = finish_base_result_at(
            temp.path(),
            "run-1",
            &CHROMA_KEY_CANDIDATES[0],
            Err(provider_error),
            Some("secret-key"),
        )
        .unwrap_err();

        let manifest = super::load_manifest(temp.path(), "run-1").unwrap();
        assert_eq!(manifest.base.status, ArtifactStatus::Failed);
        assert!(manifest.base.error.as_ref().unwrap().len() <= 512);
        assert!(!error.contains("secret-key"));
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn row_provider_failure_marks_only_requested_state_and_keeps_completed_rows() {
        let temp = TempDir::new().unwrap();
        create_run_at(temp.path(), "run-1", "siliconflow", "pet").unwrap();
        super::mark_base_generating(temp.path(), "run-1").unwrap();
        super::mark_base_complete(temp.path(), "run-1").unwrap();
        super::mark_state_generating(temp.path(), "run-1", "idle").unwrap();
        super::mark_state_complete(temp.path(), "run-1", "idle").unwrap();
        super::mark_state_generating(temp.path(), "run-1", "sleeping").unwrap();

        finish_state_result_at(
            temp.path(),
            "run-1",
            "sleeping",
            &CHROMA_KEY_CANDIDATES[0],
            Err("sleeping provider failed".to_string()),
            None,
        )
        .unwrap_err();

        let manifest = super::load_manifest(temp.path(), "run-1").unwrap();
        assert_eq!(manifest.states["idle"].status, ArtifactStatus::Complete);
        assert_eq!(manifest.states["sleeping"].status, ArtifactStatus::Failed);
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn row_processing_uses_manifest_chroma_key_and_writes_only_the_run_row() {
        let temp = TempDir::new().unwrap();
        let mut manifest = create_run_at(temp.path(), "run-1", "siliconflow", "pet").unwrap();
        manifest.chroma_key = "#00FFFF".to_string();
        super::save_manifest(temp.path(), &manifest).unwrap();
        super::mark_base_generating(temp.path(), "run-1").unwrap();
        super::mark_base_complete(temp.path(), "run-1").unwrap();
        super::mark_state_generating(temp.path(), "run-1", "idle").unwrap();
        let selected_key =
            chroma_key_for_manifest(&super::load_manifest(temp.path(), "run-1").unwrap()).unwrap();

        let row = finish_state_result_at(
            temp.path(),
            "run-1",
            "idle",
            &selected_key,
            Ok(png_bytes(&keyed_row(selected_key))),
            None,
        )
        .unwrap();

        assert_eq!(row.dimensions(), (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H));
        assert_eq!(row.get_pixel(0, 0)[3], 0);
        for frame in 0..DEFAULT_FRAME_COUNT {
            let start_x = frame * FRAME_W;
            let has_visible = (start_x..start_x + FRAME_W)
                .any(|x| (0..FRAME_H).any(|y| row.get_pixel(x, y)[3] > 0));
            assert!(has_visible, "frame {frame} lost its character after slicing");
        }
        assert!(run_dir(temp.path(), "run-1")
            .unwrap()
            .join("rows/idle.png")
            .exists());
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn assembly_at_loads_four_complete_rows_without_creating_pets() {
        let temp = TempDir::new().unwrap();
        create_run_at(temp.path(), "run-1", "siliconflow", "pet").unwrap();
        super::mark_base_generating(temp.path(), "run-1").unwrap();
        super::mark_base_complete(temp.path(), "run-1").unwrap();
        for (index, state) in state_definitions().iter().enumerate() {
            super::mark_state_generating(temp.path(), "run-1", state.key).unwrap();
            fs::write(
                run_dir(temp.path(), "run-1")
                    .unwrap()
                    .join(format!("rows/{}.png", state.key)),
                png_bytes(&opaque_row([index as u8 + 1, 2, 3, 255])),
            )
            .unwrap();
            super::mark_state_complete(temp.path(), "run-1", state.key).unwrap();
        }

        let preview = assemble_run_preview_at(temp.path(), "run-1").unwrap();
        let encoded = preview
            .data_url
            .strip_prefix("data:image/png;base64,")
            .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let image = image::load_from_memory(&bytes).unwrap();

        assert_eq!(preview.frame_w, FRAME_W);
        assert_eq!(preview.frame_h, FRAME_H);
        assert_eq!(preview.frame_count, DEFAULT_FRAME_COUNT);
        assert_eq!(preview.row_gap, 0);
        assert_eq!(image.dimensions(), (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H * 4));
        assert!(!temp.path().join("pets").exists());
    }

    #[test]
    fn progress_payload_contains_run_phase_state_and_counters() {
        let payload = generation_progress_payload("run-1", "state", Some("sleeping"), 1, 1);

        assert_eq!(payload["runId"], "run-1");
        assert_eq!(payload["phase"], "state");
        assert_eq!(payload["state"], "sleeping");
        assert_eq!(payload["current"], 1);
        assert_eq!(payload["total"], 1);
    }
}
