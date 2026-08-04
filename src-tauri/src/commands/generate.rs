use image::RgbaImage;

// Animation state frame counts: idle=4, walking=6, waving=4, working=4
pub const FRAME_COUNTS: &[(&str, usize)] = &[
    ("idle", 4),
    ("walking", 6),
    ("waving", 4),
    ("working", 4),
];

pub const TOTAL_FRAMES: usize = 18;

fn state_action_prompt(state: &str) -> &str {
    match state {
        "idle"    => "standing still, relaxed natural pose, slight smile",
        "walking" => "walking, dynamic stride, arms swinging",
        "waving"  => "waving hand cheerfully, big smile, arm raised high",
        "working" => "focused expression, leaning forward slightly, thinking pose",
        _         => "neutral pose",
    }
}

/// Builds the full Pollinations URL for one frame.
pub fn build_pollinations_url(prompt: &str) -> String {
    let encoded = urlencoding::encode(prompt);
    format!(
        "https://image.pollinations.ai/prompt/{}?width=128&height=128&nologo=true&model=flux",
        encoded
    )
}

/// Returns all 18 prompts grouped by state: [(state, [prompt1, prompt2, ...]), ...]
pub fn build_frame_prompts(base_prompt: &str) -> Vec<(String, Vec<String>)> {
    FRAME_COUNTS.iter().map(|(state, count)| {
        let action = state_action_prompt(state);
        let prompts = (0..*count).map(|i| {
            format!(
                "{}, {}, Q-version chibi style, pixel art, simple colors, white background, full body character, centered, frame {} of {}",
                base_prompt, action, i + 1, count
            )
        }).collect();
        (state.to_string(), prompts)
    }).collect()
}

/// Removes near-white pixels by setting their alpha to 0 (in place).
pub fn apply_chroma_key(img: &mut RgbaImage, threshold: u8) {
    for pixel in img.pixels_mut() {
        let [r, g, b, _] = pixel.0;
        if r > 255 - threshold && g > 255 - threshold && b > 255 - threshold {
            pixel.0 = [0, 0, 0, 0];
        }
    }
}

/// Decodes JPEG bytes into an RGBA image.
pub fn decode_jpeg(bytes: &[u8]) -> Result<RgbaImage, String> {
    let img = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    Ok(img.to_rgba8())
}

/// Encodes a list of RGBA frames into an animated GIF (returned as bytes).
/// delay_cs: frame delay in centiseconds (10 = 100ms = 10fps).
pub fn assemble_gif_bytes(frames: Vec<RgbaImage>, delay_cs: u16) -> Result<Vec<u8>, String> {
    if frames.is_empty() {
        return Err("no frames provided".to_string());
    }
    let width = frames[0].width() as u16;
    let height = frames[0].height() as u16;

    let mut output: Vec<u8> = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut output, width, height, &[])
            .map_err(|error| error.to_string())?;
        encoder.set_repeat(gif::Repeat::Infinite).map_err(|error| error.to_string())?;

        for frame_img in frames {
            let mut pixels = frame_img.into_raw();
            let mut frame = gif::Frame::from_rgba(width, height, &mut pixels);
            frame.delay = delay_cs;
            encoder.write_frame(&frame).map_err(|error| error.to_string())?;
        }
    }
    Ok(output)
}

/// Downloads a single image from a URL, returns raw bytes.
pub async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let response = reqwest::get(url).await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {} downloading image", response.status()));
    }
    response.bytes().await.map(|bytes| bytes.to_vec()).map_err(|error| error.to_string())
}

/// Fetches image bytes for a single frame from SiliconFlow.
/// Returns the image bytes after downloading from the signed URL in the response.
async fn fetch_image_siliconflow(prompt: &str, api_key: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "black-forest-labs/FLUX.1-schnell",
        "prompt": prompt,
        "image_size": "128x128",
        "num_inference_steps": 20,
        "num_images": 1,
    });
    let response = client
        .post("https://api.siliconflow.cn/v1/images/generations")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("SiliconFlow API error: {}", response.status()));
    }
    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let image_url = data["images"][0]["url"]
        .as_str()
        .ok_or_else(|| "SiliconFlow: missing image URL in response".to_string())?;
    download_image(image_url).await
}

/// Fetches image bytes for a single frame from a local AUTOMATIC1111 SD WebUI.
/// Returns PNG bytes decoded from base64.
async fn fetch_image_localsd(prompt: &str, sd_url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "prompt": prompt,
        "negative_prompt": "ugly, blurry, watermark",
        "steps": 20,
        "width": 128,
        "height": 128,
        "batch_size": 1,
    });
    let endpoint = format!("{}/sdapi/v1/txt2img", sd_url.trim_end_matches('/'));
    let response = client
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Local SD connection failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Local SD API error: {}", response.status()));
    }
    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let b64 = data["images"][0]
        .as_str()
        .ok_or_else(|| "Local SD: missing image in response".to_string())?;
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("Local SD base64 decode error: {e}"))
}

/// Saves GIF bytes to <app_data>/pets/<pet_id>/<state>.gif.
fn save_gif(pets_dir: &std::path::PathBuf, pet_id: &str, state: &str, gif_bytes: &[u8]) -> Result<(), String> {
    let dir = pets_dir.join(pet_id);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    std::fs::write(dir.join(format!("{}.gif", state)), gif_bytes).map_err(|error| error.to_string())?;
    Ok(())
}

/// Generates all 4 animated GIFs for a pet and saves them to AppData.
/// Emits "generation-progress" events: { current: u32, total: u32 }.
/// image_provider: "pollinations" | "siliconflow" | "localsd"
#[tauri::command]
pub async fn generate_and_assemble(
    app: tauri::AppHandle,
    pet_id: String,
    base_prompt: String,
    image_provider: Option<String>,
    image_api_key: Option<String>,
    local_sd_url: Option<String>,
) -> Result<(), String> {
    use tauri::Manager;
    use tauri::Emitter;

    let provider = image_provider.as_deref().unwrap_or("pollinations");
    let pets_dir = app.path().app_data_dir().map_err(|error| error.to_string())?.join("pets");
    let all_prompts = build_frame_prompts(&base_prompt);
    let mut current: u32 = 0;

    for (state, frame_prompts) in &all_prompts {
        let mut rgba_frames: Vec<RgbaImage> = Vec::new();

        for prompt in frame_prompts {
            let bytes = match provider {
                "siliconflow" => {
                    let key = image_api_key.as_deref()
                        .ok_or_else(|| "SiliconFlow requires an API key (configure in Settings)".to_string())?;
                    fetch_image_siliconflow(prompt, key).await?
                }
                "localsd" => {
                    let url = local_sd_url.as_deref().unwrap_or("http://localhost:7860");
                    fetch_image_localsd(prompt, url).await?
                }
                _ => {
                    let url = build_pollinations_url(prompt);
                    download_image(&url).await?
                }
            };

            let mut rgba = decode_jpeg(&bytes)?;
            apply_chroma_key(&mut rgba, 30);
            rgba_frames.push(rgba);

            current += 1;
            let _ = app.emit("generation-progress", serde_json::json!({
                "current": current,
                "total": TOTAL_FRAMES as u32,
            }));
        }

        let gif_bytes = assemble_gif_bytes(rgba_frames, 10)?;
        save_gif(&pets_dir, &pet_id, state, &gif_bytes)?;
    }

    Ok(())
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>, String> {
    let comma_pos = data_url.find(',').ok_or_else(|| "invalid data URL: missing comma".to_string())?;
    let base64_str = &data_url[comma_pos + 1..];
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| format!("base64 decode error: {e}"))
}

fn save_frame_file(pets_dir: &std::path::PathBuf, pet_id: &str, state: &str, bytes: &[u8]) -> Result<(), String> {
    let dir = pets_dir.join(pet_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.gif", state)), bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Saves user-supplied images/GIFs as the four animation states without AI generation.
#[tauri::command]
pub async fn save_custom_frames(
    app: tauri::AppHandle,
    pet_id: String,
    idle: String,
    walking: String,
    waving: String,
    working: String,
) -> Result<(), String> {
    use tauri::Manager;
    let pets_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("pets");

    let entries = [
        ("idle", &idle),
        ("walking", &walking),
        ("waving", &waving),
        ("working", &working),
    ];
    for (state, data_url) in &entries {
        let bytes = decode_data_url(data_url)?;
        save_frame_file(&pets_dir, &pet_id, state, &bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn build_frame_prompts_returns_correct_counts() {
        let prompts = build_frame_prompts("anime chibi girl");
        let map: std::collections::HashMap<_, _> = prompts.into_iter().collect();
        assert_eq!(map["idle"].len(), 4);
        assert_eq!(map["walking"].len(), 6);
        assert_eq!(map["waving"].len(), 4);
        assert_eq!(map["working"].len(), 4);
        let total: usize = map.values().map(|v| v.len()).sum();
        assert_eq!(total, TOTAL_FRAMES);
    }

    #[test]
    fn build_pollinations_url_encodes_spaces() {
        let url = build_pollinations_url("anime chibi girl");
        assert!(url.starts_with("https://image.pollinations.ai/prompt/"));
        assert!(url.contains("anime%20chibi%20girl") || url.contains("anime+chibi+girl"));
        assert!(url.contains("width=128"));
        assert!(url.contains("height=128"));
        assert!(url.contains("nologo=true"));
    }

    #[test]
    fn apply_chroma_key_removes_white_pixels() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([255, 255, 255, 255])); // pure white → transparent
        img.put_pixel(1, 0, Rgba([100, 50, 200, 255]));  // colored → stays
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 0);   // transparent
        assert_eq!(img.get_pixel(1, 0)[3], 255); // opaque
    }

    #[test]
    fn apply_chroma_key_keeps_near_white_below_threshold() {
        let mut img = RgbaImage::new(1, 1);
        img.put_pixel(0, 0, Rgba([200, 200, 200, 255])); // grey, threshold=30 → stays
        apply_chroma_key(&mut img, 30);
        assert_eq!(img.get_pixel(0, 0)[3], 255);
    }

    #[test]
    fn assemble_gif_bytes_returns_valid_gif_header() {
        // Two minimal 2x2 RGBA frames
        let frame1 = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
        let frame2 = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255]));
        let bytes = assemble_gif_bytes(vec![frame1, frame2], 10).unwrap();
        // GIF magic bytes: GIF89a
        assert_eq!(&bytes[0..6], b"GIF89a");
    }

    #[test]
    fn assemble_gif_bytes_errors_on_empty_frames() {
        let result = assemble_gif_bytes(vec![], 10);
        assert!(result.is_err());
    }
}
