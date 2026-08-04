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

/// Saves GIF bytes to <app_data>/pets/<pet_id>/<state>.gif.
fn save_gif(pets_dir: &std::path::PathBuf, pet_id: &str, state: &str, gif_bytes: &[u8]) -> Result<(), String> {
    let dir = pets_dir.join(pet_id);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    std::fs::write(dir.join(format!("{}.gif", state)), gif_bytes).map_err(|error| error.to_string())?;
    Ok(())
}

/// Generates all 4 animated GIFs for a pet and saves them to AppData.
/// Emits "generation-progress" events: { current: u32, total: u32 }.
#[tauri::command]
pub async fn generate_and_assemble(
    app: tauri::AppHandle,
    pet_id: String,
    base_prompt: String,
) -> Result<(), String> {
    use tauri::Manager;
    use tauri::Emitter;

    let pets_dir = app.path().app_data_dir().map_err(|error| error.to_string())?.join("pets");
    let all_prompts = build_frame_prompts(&base_prompt);
    let mut current: u32 = 0;

    for (state, frame_prompts) in &all_prompts {
        let mut rgba_frames: Vec<RgbaImage> = Vec::new();

        for prompt in frame_prompts {
            let url = build_pollinations_url(prompt);
            let bytes = download_image(&url).await?;
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
