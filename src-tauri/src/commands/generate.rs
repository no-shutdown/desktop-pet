use image::{RgbaImage, imageops};
use std::collections::HashMap;
use crate::models::SpriteStateInfo;

pub const FRAME_COUNTS: &[(&str, usize)] = &[
    ("idle",    8),
    ("walking", 8),
    ("waving",  8),
    ("working", 8),
];

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePreviewResult {
    pub data_url: String,
    pub pet_id: String,
    pub frame_w: u32,
    pub frame_h: u32,
    pub idle_frames: u32,
    pub walking_frames: u32,
    pub waving_frames: u32,
    pub working_frames: u32,
}

pub const TOTAL_STATES: usize = FRAME_COUNTS.len();

// Stored cell size per frame in the sprite sheet PNG.
const CELL_SIZE: u32 = 128;
// Generation size per cell — fetched at 2× then downsampled for quality.
const API_CELL_SIZE: u32 = 256;

fn state_animation_desc(state: &str, frame_count: usize, cols: usize, rows: usize) -> String {
    let action = state;
    let motion = match state {
        "idle" =>
            "standing still with gentle breathing, subtle body sway, small pose variations, relaxed resting pose loop",
        "walking" =>
            "walking to the right, complete walk cycle, alternating leg steps with opposite arm swing, smooth sequential gait",
        "waving" =>
            "waving right hand, arm raises and hand moves side to side, big cheerful smile, friendly wave motion",
        "working" =>
            "typing at keyboard, fingers moving on keys, slight head tilt and bob, focused expression, desk work motion",
        _ => "animation loop",
    };
    format!(
        "pixel art sprite sheet, {cols}x{rows} grid, {n} sequential frames, character {action}: {motion}, pure white background, same character design every frame",
        cols   = cols,
        rows   = rows,
        n      = frame_count,
        action = action,
        motion = motion,
    )
}

pub struct StateSpec {
    pub state: String,
    pub frame_count: usize,
    pub cols: usize,
    pub rows: usize,
}

impl StateSpec {
    pub fn img_width(&self) -> u32  { self.cols as u32 * CELL_SIZE }
    pub fn img_height(&self) -> u32 { self.rows as u32 * CELL_SIZE }
}

pub fn build_state_specs() -> Vec<StateSpec> {
    build_state_specs_with_count(FRAME_COUNTS[0].1)
}

pub fn build_state_specs_with_count(frame_count: usize) -> Vec<StateSpec> {
    let cols = 4usize;
    let rows = (frame_count + cols - 1) / cols;
    FRAME_COUNTS.iter().map(|(state, _)| {
        StateSpec { state: state.to_string(), frame_count, cols, rows }
    }).collect()
}

pub fn build_sprite_prompt(base_prompt: &str, spec: &StateSpec) -> String {
    let truncated = if base_prompt.len() > 300 {
        let mut end = 300;
        while !base_prompt.is_char_boundary(end) { end -= 1; }
        &base_prompt[..end]
    } else {
        base_prompt
    };
    let anim_desc = state_animation_desc(&spec.state, spec.frame_count, spec.cols, spec.rows);
    // Order: animation description → character description → quality tags.
    // Animation layout info comes first so it is preserved even if URL truncates.
    format!(
        "{}, {}, no text, no watermark",
        anim_desc, truncated
    )
}

pub fn build_pollinations_url(prompt: &str, width: u32, height: u32) -> String {
    let truncated = if prompt.len() > 700 {
        let mut end = 700;
        while !prompt.is_char_boundary(end) { end -= 1; }
        &prompt[..end]
    } else {
        prompt
    };
    let encoded = urlencoding::encode(truncated);
    format!(
        "https://image.pollinations.ai/prompt/{}?width={}&height={}&nologo=true&model=flux",
        encoded, width, height
    )
}

pub fn apply_chroma_key(img: &mut RgbaImage, threshold: u8) {
    let w = img.width();
    let h = img.height();

    let corner_coords = [
        (0, 0),
        (w.saturating_sub(1), 0),
        (0, h.saturating_sub(1)),
        (w.saturating_sub(1), h.saturating_sub(1)),
    ];

    // Sample corners to detect whether the background is dark or light.
    let avg_brightness: u32 = corner_coords
        .iter()
        .map(|&(x, y)| {
            let p = img.get_pixel(x, y);
            (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3
        })
        .sum::<u32>()
        / corner_coords.len() as u32;

    let dark_bg = avg_brightness < 128;

    for pixel in img.pixels_mut() {
        let [r, g, b, _] = pixel.0;
        let remove = if dark_bg {
            r < threshold && g < threshold && b < threshold
        } else {
            r > 255 - threshold && g > 255 - threshold && b > 255 - threshold
        };
        if remove {
            pixel.0 = [0, 0, 0, 0];
        }
    }
}

/// Takes a multi-row grid image and rearranges all frames into a single horizontal row.
/// Each frame is `frame_w × frame_h`. The source image has `cols` columns and `rows` rows.
pub fn flatten_to_single_row(img: &RgbaImage, frame_w: u32, frame_h: u32, cols: u32, rows: u32) -> RgbaImage {
    let total_frames = cols * rows;
    let mut out = RgbaImage::new(frame_w * total_frames, frame_h);
    for frame_idx in 0..total_frames {
        let src_col = frame_idx % cols;
        let src_row = frame_idx / cols;
        let src_x = src_col * frame_w;
        let src_y = src_row * frame_h;
        let dst_x = frame_idx * frame_w;
        let frame = imageops::crop_imm(img, src_x, src_y, frame_w, frame_h).to_image();
        imageops::replace(&mut out, &frame, dst_x as i64, 0);
    }
    out
}

pub fn decode_sprite_sheet(bytes: &[u8], width: u32, height: u32) -> Result<RgbaImage, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let resized = img.resize_exact(width, height, imageops::FilterType::Lanczos3);
    Ok(resized.to_rgba8())
}

pub fn save_sprite_sheet_png(
    pets_dir: &std::path::PathBuf,
    pet_id: &str,
    state: &str,
    sheet: &RgbaImage,
) -> Result<(), String> {
    let dir = pets_dir.join(pet_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    sheet
        .save(dir.join(format!("{}.png", state)))
        .map_err(|e| e.to_string())
}

pub async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut last_err = String::new();
    for attempt in 0..3u64 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(5 * attempt)).await;
        }
        let resp = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => { last_err = e.to_string(); continue; }
        };
        if resp.status().is_success() {
            return resp.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string());
        }
        last_err = format!("HTTP {} downloading image", resp.status());
    }
    Err(last_err)
}

async fn fetch_image_siliconflow(
    prompt: &str, api_key: &str, model: &str, width: u32, height: u32,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "image_size": format!("{}x{}", width, height),
        "num_inference_steps": 20,
        "num_images": 1,
    });
    let resp = client
        .post("https://api.siliconflow.cn/v1/images/generations")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("SiliconFlow API error {}: {}", status, body_text));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let image_url = data["images"][0]["url"]
        .as_str()
        .ok_or_else(|| "SiliconFlow: missing image URL in response".to_string())?;
    download_image(image_url).await
}

async fn fetch_image_localsd(
    prompt: &str, sd_url: &str, width: u32, height: u32,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "prompt": prompt,
        "negative_prompt": "ugly, blurry, watermark, multiple characters",
        "steps": 20,
        "width": width,
        "height": height,
        "batch_size": 1,
    });
    let endpoint = format!("{}/sdapi/v1/txt2img", sd_url.trim_end_matches('/'));
    let resp = client.post(&endpoint).json(&body).send().await
        .map_err(|e| format!("Local SD connection failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Local SD API error: {}", resp.status()));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let b64 = data["images"][0].as_str()
        .ok_or_else(|| "Local SD: missing image in response".to_string())?;
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("Local SD base64 decode error: {e}"))
}

#[tauri::command]
pub async fn generate_and_assemble(
    app: tauri::AppHandle,
    base_prompt: String,
    frame_count_per_state: Option<u32>,
    image_provider: Option<String>,
    image_api_key: Option<String>,
    image_model: Option<String>,
    local_sd_url: Option<String>,
) -> Result<GeneratePreviewResult, String> {
    use tauri::{Emitter, Manager};

    let provider = image_provider.as_deref().unwrap_or("pollinations");
    let count = frame_count_per_state.unwrap_or(8).max(1) as usize;
    let state_specs = build_state_specs_with_count(count);
    let total = state_specs.len() as u32;

    let gen_pet_id = uuid::Uuid::new_v4().to_string();
    let pets_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("pets");
    let raw_dir = pets_dir.join(&gen_pet_id).join("raw");
    std::fs::create_dir_all(&raw_dir).map_err(|e| e.to_string())?;

    // Collect single-row strips for each state to stack vertically at the end.
    let mut row_strips: Vec<RgbaImage> = Vec::new();

    for (idx, spec) in state_specs.iter().enumerate() {
        let prompt = build_sprite_prompt(&base_prompt, spec);

        // Generate the full sprite sheet at API_CELL_SIZE per cell, then resize down.
        let api_w = spec.cols as u32 * API_CELL_SIZE;
        let api_h = spec.rows as u32 * API_CELL_SIZE;

        let fetch_result = match provider {
            "siliconflow" => {
                let key = image_api_key.as_deref()
                    .ok_or_else(|| "SiliconFlow requires an API key (configure in Settings)".to_string())?;
                let model = image_model.as_deref().unwrap_or("Tongyi-MAI/Z-Image-Turbo");
                fetch_image_siliconflow(&prompt, key, model, api_w, api_h).await
            }
            "localsd" => {
                let url = local_sd_url.as_deref().unwrap_or("http://localhost:7860");
                fetch_image_localsd(&prompt, url, api_w, api_h).await
            }
            _ => {
                let url = build_pollinations_url(&prompt, api_w, api_h);
                download_image(&url).await
            }
        };

        let bytes = fetch_result
            .map_err(|e| format!("生成「{}」动画失败: {}", spec.state, e))?;

        // Save raw image before any processing.
        let _ = std::fs::write(raw_dir.join(format!("{}.png", spec.state)), &bytes);

        let store_w = spec.img_width();
        let store_h = spec.img_height();
        let mut sheet = decode_sprite_sheet(&bytes, store_w, store_h)
            .map_err(|e| format!("解码「{}」精灵图失败: {}", spec.state, e))?;

        apply_chroma_key(&mut sheet, 30);

        // Rearrange multi-row grid into a single horizontal strip.
        let single_row = flatten_to_single_row(
            &sheet,
            CELL_SIZE,
            CELL_SIZE,
            spec.cols as u32,
            spec.rows as u32,
        );
        row_strips.push(single_row);

        let _ = app.emit("generation-progress", serde_json::json!({
            "current": idx as u32 + 1,
            "total": total,
        }));
    }

    // Stack all single-row strips vertically into one combined image.
    let combined_w = row_strips[0].width();
    let combined_h = CELL_SIZE * total;
    let mut combined = RgbaImage::new(combined_w, combined_h);
    for (row_idx, strip) in row_strips.iter().enumerate() {
        imageops::replace(&mut combined, strip, 0, (row_idx as u32 * CELL_SIZE) as i64);
    }

    // Encode combined image as PNG and base64.
    let mut png_bytes: Vec<u8> = Vec::new();
    combined
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("PNG编码失败: {e}"))?;

    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    let data_url = format!("data:image/png;base64,{}", b64);

    Ok(GeneratePreviewResult {
        data_url,
        pet_id: gen_pet_id,
        frame_w: CELL_SIZE,
        frame_h: CELL_SIZE,
        idle_frames:    count as u32,
        walking_frames: count as u32,
        waving_frames:  count as u32,
        working_frames: count as u32,
    })
}

/// Import a combined sprite sheet where each row is one animation state.
/// Row order: idle, walking, waving, working.
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
    use tauri::Manager;

    let bytes = decode_data_url(&data_url)?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();

    let pets_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("pets");

    let rows: &[(&str, u32, u32)] = &[
        ("idle",    idle_frames,    150),
        ("walking", walking_frames, 100),
        ("waving",  waving_frames,  110),
        ("working", working_frames, 120),
    ];

    let mut result: HashMap<String, SpriteStateInfo> = HashMap::new();

    for (row_idx, (state, frame_count, delay_ms)) in rows.iter().enumerate() {
        let y_start = row_idx as u32 * (frame_h + row_gap);
        let row_w   = frame_w * frame_count;

        if y_start + frame_h > rgba.height() {
            return Err(format!(
                "图片高度不足：「{}」行起始 y={} + 帧高 {} 超出图片高度 {}",
                state, y_start, frame_h, rgba.height()
            ));
        }
        if row_w > rgba.width() {
            return Err(format!(
                "图片宽度不足：「{}」需要 {} px（{}帧 × {}px），图片宽度为 {}",
                state, row_w, frame_count, frame_w, rgba.width()
            ));
        }

        let row_sheet = imageops::crop_imm(&rgba, 0, y_start, row_w, frame_h).to_image();
        save_sprite_sheet_png(&pets_dir, &pet_id, state, &row_sheet)?;

        result.insert(state.to_string(), SpriteStateInfo {
            cols: *frame_count as usize,
            rows: 1,
            frame_count: *frame_count as usize,
            frame_w,
            frame_h,
            delay_ms: *delay_ms,
        });
    }

    Ok(result)
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let comma_pos = data_url.find(',').ok_or_else(|| "invalid data URL: missing comma".to_string())?;
    let base64_str = &data_url[comma_pos + 1..];
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| format!("base64 decode error: {e}"))
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
    use tauri::Manager;

    let bytes = decode_data_url(&data_url)?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();

    let pets_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("pets");

    // Each entry: (state_name, cells, delay_ms)
    let state_entries: [(&str, &Vec<FrameCell>, u32); 4] = [
        ("idle",    &idle_cells,    150),
        ("walking", &walking_cells, 100),
        ("waving",  &waving_cells,  110),
        ("working", &working_cells, 120),
    ];

    let mut result: HashMap<String, SpriteStateInfo> = HashMap::new();

    for (state, cells, delay_ms) in &state_entries {
        if cells.is_empty() {
            return Err(format!("「{}」动作没有选择任何帧", state));
        }

        let frame_count = cells.len() as u32;
        let mut sheet = RgbaImage::new(frame_w * frame_count, frame_h);

        for (i, cell) in cells.iter().enumerate() {
            let src_x = cell.col * (frame_w + col_gap);
            let src_y = cell.row * (frame_h + row_gap);

            if src_x + frame_w > rgba.width() || src_y + frame_h > rgba.height() {
                return Err(format!(
                    "「{}」第 {} 帧超出图片边界 (x={}, y={}, 图片 {}×{})",
                    state,
                    i + 1,
                    src_x,
                    src_y,
                    rgba.width(),
                    rgba.height()
                ));
            }

            let frame = imageops::crop_imm(&rgba, src_x, src_y, frame_w, frame_h).to_image();
            imageops::replace(&mut sheet, &frame, (i as i64) * (frame_w as i64), 0);
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
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn build_state_specs_counts() {
        let specs = build_state_specs();
        let map: std::collections::HashMap<_, _> =
            specs.iter().map(|s| (s.state.as_str(), s.frame_count)).collect();
        assert_eq!(map["idle"],    8);
        assert_eq!(map["walking"], 8);
        assert_eq!(map["waving"],  8);
        assert_eq!(map["working"], 8);
    }

    #[test]
    fn state_spec_grid_layout() {
        let specs = build_state_specs();
        for spec in &specs {
            assert_eq!(spec.cols, 4);
            assert_eq!(spec.rows, (spec.frame_count + 3) / 4);
            assert_eq!(spec.img_width(),  spec.cols as u32 * CELL_SIZE);
            assert_eq!(spec.img_height(), spec.rows as u32 * CELL_SIZE);
        }
    }

    #[test]
    fn build_pollinations_url_encodes_spaces() {
        let url = build_pollinations_url("anime chibi girl", 256, 256);
        assert!(url.starts_with("https://image.pollinations.ai/prompt/"));
        assert!(url.contains("anime%20chibi%20girl") || url.contains("anime+chibi+girl"));
        assert!(url.contains("width=256"));
        assert!(url.contains("height=256"));
    }

    #[test]
    fn apply_chroma_key_removes_white_pixels() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
        img.put_pixel(1, 0, Rgba([100, 50, 200, 255]));
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(1, 0)[3], 255);
    }

    #[test]
    fn apply_chroma_key_keeps_near_white_below_threshold() {
        let mut img = RgbaImage::new(1, 1);
        img.put_pixel(0, 0, Rgba([200, 200, 200, 255]));
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn apply_chroma_key_removes_dark_background() {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 0, 0, 255]));
        img.put_pixel(0, 1, Rgba([0, 0, 0, 255]));
        img.put_pixel(1, 1, Rgba([100, 150, 200, 255]));
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 0, "black corner should be removed");
        assert_eq!(img.get_pixel(1, 0)[3], 0, "black pixel should be removed");
        assert_eq!(img.get_pixel(0, 1)[3], 0, "black pixel should be removed");
        assert_eq!(img.get_pixel(1, 1)[3], 255, "colored pixel should be kept");
    }

    #[test]
    fn save_sprite_sheet_png_writes_correct_dimensions() {
        let dir = tempfile::TempDir::new().unwrap();
        let sheet = RgbaImage::new(256, 256);
        save_sprite_sheet_png(&dir.path().to_path_buf(), "pet-1", "idle", &sheet).unwrap();
        let path = dir.path().join("pet-1").join("idle.png");
        assert!(path.exists());
        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 256);
        assert_eq!(loaded.height(), 256);
    }

    #[test]
    fn build_sprite_prompt_contains_base_and_key_terms() {
        let specs = build_state_specs();
        let walking_spec = specs.iter().find(|s| s.state == "walking").unwrap();
        let prompt = build_sprite_prompt("anime cat girl", walking_spec);
        assert!(prompt.contains("anime cat girl"), "should contain character desc");
        assert!(prompt.contains("sprite sheet"),   "should contain sprite sheet keyword");
        assert!(prompt.contains("walk"),           "should contain action keyword");
        assert!(prompt.contains("4x2"),            "should contain grid dimensions");
        assert!(prompt.contains("same character"), "should emphasize consistency");
    }

    #[test]
    fn build_sprite_prompt_sprite_info_comes_before_character() {
        let specs = build_state_specs();
        let spec = specs.iter().find(|s| s.state == "idle").unwrap();
        let prompt = build_sprite_prompt("a wizard cat", spec);
        // Sprite sheet info must appear before character description so it
        // is not lost if the URL truncates from the end.
        let sprite_pos = prompt.find("sprite sheet").unwrap();
        let char_pos   = prompt.find("wizard cat").unwrap();
        assert!(sprite_pos < char_pos, "sprite sheet info should precede character desc");
    }

    #[test]
    fn flatten_to_single_row_produces_correct_dimensions() {
        // Build a 4×2 grid (4 cols, 2 rows) with 8 frames of 16×16 each.
        let frame_w = 16u32;
        let frame_h = 16u32;
        let cols = 4u32;
        let rows = 2u32;
        let grid = RgbaImage::new(frame_w * cols, frame_h * rows);
        let result = flatten_to_single_row(&grid, frame_w, frame_h, cols, rows);
        assert_eq!(result.width(),  frame_w * cols * rows, "width should hold all frames");
        assert_eq!(result.height(), frame_h,               "height should be one frame tall");
    }
}
