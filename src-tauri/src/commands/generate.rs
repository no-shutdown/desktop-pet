use image::{RgbaImage, imageops};
use std::collections::HashMap;
use crate::models::SpriteStateInfo;

pub const FRAME_COUNTS: &[(&str, usize)] = &[
    ("idle",    8),
    ("walking", 8),
    ("waving",  8),
    ("working", 6),
];

pub const TOTAL_STATES: usize = FRAME_COUNTS.len();

const CELL_SIZE: u32 = 128;

fn state_action_prompt(state: &str) -> &str {
    match state {
        "idle"    => "standing still, relaxed natural pose, slight smile, subtle breathing motion",
        "walking" => "walking cycle, legs alternating steps, arms swinging naturally at sides",
        "waving"  => "waving hand high with cheerful big smile, arm raised above head",
        "working" => "focused working pose, leaning forward slightly, thinking or typing expression",
        _         => "neutral pose",
    }
}

fn frame_phase(state: &str, frame_idx: usize) -> &'static str {
    const IDLE: &[&str] = &[
        "relaxed neutral standing, arms at sides, looking forward",
        "very slight exhale, shoulders gently settled, peaceful expression",
        "head very slightly tilted left, arms at sides, content look",
        "weight subtly shifting, natural breathing, soft smile",
        "neutral stance returning, arms at sides, calm expression",
        "gentle inhale, shoulders barely raised, serene face",
        "head very slightly tilted right, arms relaxed, gentle smile",
        "fully upright neutral, grounded stance, soft forward gaze",
    ];
    const WALKING: &[&str] = &[
        "left foot stepping forward, right arm swinging forward, striding",
        "feet close mid-stride, arms passing center, weight shifting",
        "right foot stepping forward, left arm swinging forward, in motion",
        "feet close mid-stride, arms at sides, transferring weight",
        "left foot forward again, right arm forward, confident step",
        "mid-stride bounce, arms in motion, energetic walk",
        "right foot stepping forward, left arm swinging, natural gait",
        "mid-stride transition, arms settling, smooth walking rhythm",
    ];
    const WAVING: &[&str] = &[
        "right arm raised, hand tilted to the left, big cheerful smile",
        "right arm raised high, hand at top center, bright happy expression",
        "right arm raised, hand tilted to the right, joyful waving",
        "right arm raised, hand centered at peak, enthusiastic smile",
        "arm raised, hand sweeping left, beaming grin",
        "arm raised high, hand at center, delighted expression",
        "arm raised, hand sweeping right, warm cheerful wave",
        "arm at full height, hand centered, very happy expression",
    ];
    const WORKING: &[&str] = &[
        "leaning slightly forward, fingers near keyboard, focused expression",
        "typing intently, fingers on keys, very concentrated look",
        "pausing to think, hand near chin, thoughtful gaze at screen",
        "slight nod of understanding, pen near chin, pondering",
        "typing quickly, leaning in, determined focused expression",
        "leaning back slightly, reviewing work, satisfied thoughtful look",
    ];

    match state {
        "idle"    => IDLE[frame_idx % IDLE.len()],
        "walking" => WALKING[frame_idx % WALKING.len()],
        "waving"  => WAVING[frame_idx % WAVING.len()],
        "working" => WORKING[frame_idx % WORKING.len()],
        _         => "natural pose",
    }
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
    FRAME_COUNTS.iter().map(|(state, count)| {
        let cols = 2usize;
        let rows = (count + cols - 1) / cols;
        StateSpec { state: state.to_string(), frame_count: *count, cols, rows }
    }).collect()
}

pub fn build_frame_prompt(base_prompt: &str, state: &str, frame_idx: usize) -> String {
    let truncated = if base_prompt.len() > 300 {
        let mut end = 300;
        while !base_prompt.is_char_boundary(end) { end -= 1; }
        &base_prompt[..end]
    } else {
        base_prompt
    };
    let action = state_action_prompt(state);
    let phase = frame_phase(state, frame_idx);
    format!(
        "{}, {}, {}, chibi pixel art, simple flat colors, pure white background, full body character, single animation frame, centered character, square image",
        truncated, action, phase
    )
}

// Kept for tests / external callers that still reference the sprite-sheet prompt style.
pub fn build_sprite_prompt(base_prompt: &str, spec: &StateSpec) -> String {
    let truncated = if base_prompt.len() > 300 {
        let mut end = 300;
        while !base_prompt.is_char_boundary(end) { end -= 1; }
        &base_prompt[..end]
    } else {
        base_prompt
    };
    let action = state_action_prompt(&spec.state);
    format!(
        "{}, {}, chibi pixel art, simple flat colors, pure white background, full body character, sprite sheet {}x{} grid layout, {} sequential animation frames, same consistent character in every cell",
        truncated, action, spec.cols, spec.rows, spec.frame_count
    )
}

pub fn build_pollinations_url(prompt: &str, width: u32, height: u32) -> String {
    let truncated = if prompt.len() > 450 {
        let mut end = 450;
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
    for pixel in img.pixels_mut() {
        let [r, g, b, _] = pixel.0;
        if r > 255 - threshold && g > 255 - threshold && b > 255 - threshold {
            pixel.0 = [0, 0, 0, 0];
        }
    }
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
    pet_id: String,
    base_prompt: String,
    image_provider: Option<String>,
    image_api_key: Option<String>,
    image_model: Option<String>,
    local_sd_url: Option<String>,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    use tauri::Manager;
    use tauri::Emitter;

    let provider = image_provider.as_deref().unwrap_or("pollinations");
    let pets_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("pets");
    let state_specs = build_state_specs();

    // Total progress steps = total number of individual frames across all states.
    let total: u32 = state_specs.iter().map(|s| s.frame_count as u32).sum();
    let mut current: u32 = 0;
    let mut call_count: u32 = 0;
    let mut states: HashMap<String, SpriteStateInfo> = HashMap::new();

    for spec in &state_specs {
        let mut sheet = RgbaImage::new(spec.img_width(), spec.img_height());

        for frame_idx in 0..spec.frame_count {
            let prompt = build_frame_prompt(&base_prompt, &spec.state, frame_idx);

            // Some providers (SiliconFlow, LocalSD) require width > 256.
            // We fetch at a larger size and let decode_sprite_sheet resize down to CELL_SIZE.
            const API_SIZE: u32 = 512;

            let fetch_result = match provider {
                "siliconflow" => {
                    let key = image_api_key.as_deref()
                        .ok_or_else(|| "SiliconFlow requires an API key (configure in Settings)".to_string())?;
                    let model = image_model.as_deref().unwrap_or("Tongyi-MAI/Z-Image-Turbo");
                    fetch_image_siliconflow(&prompt, key, model, API_SIZE, API_SIZE).await
                }
                "localsd" => {
                    let url = local_sd_url.as_deref().unwrap_or("http://localhost:7860");
                    fetch_image_localsd(&prompt, url, API_SIZE, API_SIZE).await
                }
                _ => {
                    // Rate-limit free Pollinations API: 1 s between calls.
                    if call_count > 0 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    let url = build_pollinations_url(&prompt, CELL_SIZE, CELL_SIZE);
                    download_image(&url).await
                }
            };

            let mut frame = fetch_result
                .and_then(|b| decode_sprite_sheet(&b, CELL_SIZE, CELL_SIZE))
                .map_err(|e| format!("生成「{}」第{}帧失败: {}", spec.state, frame_idx + 1, e))?;

            apply_chroma_key(&mut frame, 30);

            // Paste frame into the sprite sheet at (col, row) grid position.
            let col = (frame_idx % spec.cols) as u32;
            let row = (frame_idx / spec.cols) as u32;
            imageops::replace(&mut sheet, &frame, (col * CELL_SIZE) as i64, (row * CELL_SIZE) as i64);

            call_count += 1;
            current += 1;
            let _ = app.emit("generation-progress", serde_json::json!({
                "current": current,
                "total": total,
            }));
        }

        save_sprite_sheet_png(&pets_dir, &pet_id, &spec.state, &sheet)?;

        let delay_ms: u32 = match spec.state.as_str() {
            "walking" | "waving" => 100,
            _ => 120,
        };

        states.insert(spec.state.clone(), SpriteStateInfo {
            cols: spec.cols,
            rows: spec.rows,
            frame_count: spec.frame_count,
            frame_w: CELL_SIZE,
            frame_h: CELL_SIZE,
            delay_ms,
        });
    }

    Ok(states)
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let comma_pos = data_url.find(',').ok_or_else(|| "invalid data URL: missing comma".to_string())?;
    let base64_str = &data_url[comma_pos + 1..];
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| format!("base64 decode error: {e}"))
}

#[tauri::command]
pub async fn save_custom_frames(
    app: tauri::AppHandle,
    pet_id: String,
    idle: String,
    walking: String,
    waving: String,
    working: String,
) -> Result<HashMap<String, SpriteStateInfo>, String> {
    use tauri::Manager;

    let pets_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("pets");

    let entries = [
        ("idle",    &idle),
        ("walking", &walking),
        ("waving",  &waving),
        ("working", &working),
    ];

    let mut states: HashMap<String, SpriteStateInfo> = HashMap::new();

    for (state, data_url) in &entries {
        let bytes = decode_data_url(data_url)?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let resized = imageops::resize(&img.to_rgba8(), 128, 128, imageops::FilterType::Lanczos3);
        save_sprite_sheet_png(&pets_dir, pet_id.as_str(), state, &resized)?;
        states.insert(state.to_string(), SpriteStateInfo {
            cols: 1,
            rows: 1,
            frame_count: 1,
            frame_w: 128,
            frame_h: 128,
            delay_ms: 200,
        });
    }

    Ok(states)
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
        assert_eq!(map["working"], 6);
    }

    #[test]
    fn state_spec_grid_layout() {
        let specs = build_state_specs();
        for spec in &specs {
            assert_eq!(spec.cols, 2);
            assert_eq!(spec.rows, (spec.frame_count + 1) / 2);
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
    fn frame_phase_covers_all_states() {
        for state in &["idle", "walking", "waving", "working"] {
            for i in 0..8 {
                let phase = frame_phase(state, i);
                assert!(!phase.is_empty());
            }
        }
    }

    #[test]
    fn build_frame_prompt_contains_base_and_phase() {
        let prompt = build_frame_prompt("anime cat girl", "idle", 0);
        assert!(prompt.contains("anime cat girl"));
        assert!(prompt.contains("chibi pixel art"));
        assert!(prompt.contains("single animation frame"));
    }
}
