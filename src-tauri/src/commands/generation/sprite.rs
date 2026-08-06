use super::types::{state_definitions, DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W};
use base64::Engine as _;
use image::{imageops::FilterType, DynamicImage, Rgba, RgbaImage};
use std::io::Cursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromaKey {
    pub name: &'static str,
    pub hex: &'static str,
    pub rgb: [u8; 3],
}

pub const CHROMA_KEY_CANDIDATES: [ChromaKey; 6] = [
    ChromaKey {
        name: "magenta",
        hex: "#FF00FF",
        rgb: [255, 0, 255],
    },
    ChromaKey {
        name: "cyan",
        hex: "#00FFFF",
        rgb: [0, 255, 255],
    },
    ChromaKey {
        name: "yellow",
        hex: "#FFFF00",
        rgb: [255, 255, 0],
    },
    ChromaKey {
        name: "blue",
        hex: "#0000FF",
        rgb: [0, 0, 255],
    },
    ChromaKey {
        name: "orange",
        hex: "#FF7F00",
        rgb: [255, 127, 0],
    },
    ChromaKey {
        name: "green",
        hex: "#00FF00",
        rgb: [0, 255, 0],
    },
];

pub fn choose_chroma_key(reference: Option<&RgbaImage>) -> ChromaKey {
    let Some(reference) = reference else {
        return CHROMA_KEY_CANDIDATES[0];
    };

    let width = reference.width();
    let height = reference.height();
    if width == 0 || height == 0 {
        return CHROMA_KEY_CANDIDATES[0];
    }

    let mut samples = Vec::with_capacity(25);
    for grid_y in 0..5u32 {
        let y = grid_y * (height - 1) / 4;
        for grid_x in 0..5u32 {
            let x = grid_x * (width - 1) / 4;
            samples.push(reference.get_pixel(x, y).0);
        }
    }

    let mut best = CHROMA_KEY_CANDIDATES[0];
    let mut best_score = 0;
    for candidate in CHROMA_KEY_CANDIDATES {
        let score = samples
            .iter()
            .map(|sample| squared_rgb_distance(candidate.rgb, [sample[0], sample[1], sample[2]]))
            .min()
            .unwrap_or(0);
        if score > best_score {
            best = candidate;
            best_score = score;
        }
    }

    best
}

pub fn chroma_key_from_hex(hex: &str) -> Result<ChromaKey, String> {
    CHROMA_KEY_CANDIDATES
        .iter()
        .copied()
        .find(|candidate| candidate.hex.eq_ignore_ascii_case(hex.trim()))
        .ok_or_else(|| format!("unknown chroma key: {hex}"))
}

pub fn apply_chroma_key(image: &mut RgbaImage, key: &ChromaKey, threshold: u8) {
    const RAMP_WIDTH: f32 = 16.0;
    let threshold = f32::from(threshold);
    let ramp_end = threshold + RAMP_WIDTH;

    for pixel in image.pixels_mut() {
        if pixel[3] == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
            continue;
        }

        let distance =
            (squared_rgb_distance([pixel[0], pixel[1], pixel[2]], key.rgb) as f32).sqrt();

        if distance <= threshold {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if distance < ramp_end {
            let alpha_ratio = (distance - threshold) / RAMP_WIDTH;
            pixel[3] = (f32::from(pixel[3]) * alpha_ratio).round() as u8;
            if pixel[3] == 0 {
                *pixel = Rgba([0, 0, 0, 0]);
            }
        }
    }
}

pub fn normalize_horizontal_row(bytes: &[u8], key: &ChromaKey) -> Result<RgbaImage, String> {
    let decoded =
        image::load_from_memory(bytes).map_err(|error| format!("decode image: {error}"))?;
    let mut row = decoded
        .resize_exact(FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H, FilterType::Lanczos3)
        .to_rgba8();
    apply_chroma_key(&mut row, key, 32);
    Ok(row)
}

pub fn normalize_base_image(bytes: &[u8], key: &ChromaKey) -> Result<RgbaImage, String> {
    let decoded =
        image::load_from_memory(bytes).map_err(|error| format!("decode image: {error}"))?;
    let mut base = decoded
        .resize_exact(FRAME_W, FRAME_H, FilterType::Lanczos3)
        .to_rgba8();
    apply_chroma_key(&mut base, key, 32);
    if !base.pixels().any(|pixel| pixel[3] != 0) {
        return Err("canonical base image is empty after chroma keying".to_string());
    }
    Ok(base)
}

pub fn validate_sprite_row(
    image: &RgbaImage,
    frame_w: u32,
    frame_h: u32,
    frame_count: u32,
) -> Result<(), String> {
    if frame_count == 0 {
        return Err("invalid frame count: must be greater than zero".to_string());
    }

    let expected_width = frame_w
        .checked_mul(frame_count)
        .ok_or_else(|| "invalid dimensions: width overflow".to_string())?;
    if frame_w == 0 || frame_h == 0 || image.width() != expected_width || image.height() != frame_h
    {
        return Err(format!(
            "invalid dimensions: expected {}x{}, got {}x{}",
            expected_width,
            frame_h,
            image.width(),
            image.height()
        ));
    }

    for frame_index in 0..frame_count {
        let start_x = frame_index * frame_w;
        let has_visible_pixel = (start_x..start_x + frame_w)
            .any(|x| (0..frame_h).any(|y| image.get_pixel(x, y)[3] != 0));
        if !has_visible_pixel {
            return Err(format!("empty frame at index {frame_index}"));
        }
    }

    Ok(())
}

pub fn assemble_rows(rows: &[RgbaImage], frame_w: u32, frame_h: u32) -> Result<RgbaImage, String> {
    let expected_row_count = state_definitions().len();
    if rows.len() != expected_row_count {
        return Err(format!(
            "invalid row count: expected {expected_row_count}, got {}",
            rows.len()
        ));
    }

    for row in rows {
        validate_sprite_row(row, frame_w, frame_h, DEFAULT_FRAME_COUNT)?;
    }

    let combined_height = frame_h
        .checked_mul(rows.len() as u32)
        .ok_or_else(|| "invalid dimensions: height overflow".to_string())?;
    let mut combined = RgbaImage::new(frame_w * DEFAULT_FRAME_COUNT, combined_height);
    for (row_index, row) in rows.iter().enumerate() {
        for y in 0..frame_h {
            for x in 0..row.width() {
                combined.put_pixel(x, y + row_index as u32 * frame_h, *row.get_pixel(x, y));
            }
        }
    }

    Ok(combined)
}

pub fn image_to_data_url(image: &RgbaImage) -> Result<String, String> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|error| format!("encode PNG: {error}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes.into_inner());
    Ok(format!("data:image/png;base64,{encoded}"))
}

fn squared_rgb_distance(left: [u8; 3], right: [u8; 3]) -> u32 {
    let red = i32::from(left[0]) - i32::from(right[0]);
    let green = i32::from(left[1]) - i32::from(right[1]);
    let blue = i32::from(left[2]) - i32::from(right[2]);
    (red * red + green * green + blue * blue) as u32
}

#[cfg(test)]
mod tests {
    use super::{
        apply_chroma_key, assemble_rows, choose_chroma_key, chroma_key_from_hex, image_to_data_url,
        normalize_base_image, normalize_horizontal_row, validate_sprite_row, ChromaKey,
        CHROMA_KEY_CANDIDATES,
    };
    use crate::commands::generation::types::{DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W};
    use base64::Engine;
    use image::{GenericImageView, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

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

    #[test]
    fn chroma_candidates_follow_hatch_pet_order() {
        let names = CHROMA_KEY_CANDIDATES
            .iter()
            .map(|candidate| candidate.name)
            .collect::<Vec<_>>();
        let hexes = CHROMA_KEY_CANDIDATES
            .iter()
            .map(|candidate| candidate.hex)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec!["magenta", "cyan", "yellow", "blue", "orange", "green"]
        );
        assert_eq!(
            hexes,
            vec!["#FF00FF", "#00FFFF", "#FFFF00", "#0000FF", "#FF7F00", "#00FF00"]
        );
    }

    #[test]
    fn chooses_the_farthest_candidate_from_a_red_reference() {
        let reference = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));

        assert_eq!(choose_chroma_key(Some(&reference)).name, "cyan");
    }

    #[test]
    fn falls_back_to_magenta_without_a_reference() {
        assert_eq!(choose_chroma_key(None).name, "magenta");
    }

    #[test]
    fn applying_the_actual_key_removes_key_pixels_and_preserves_opaque_pixels() {
        let key = ChromaKey {
            name: "magenta",
            hex: "#FF00FF",
            rgb: [255, 0, 255],
        };
        let mut image = RgbaImage::new(3, 1);
        image.put_pixel(0, 0, Rgba([255, 0, 255, 255]));
        image.put_pixel(1, 0, Rgba([255, 0, 245, 255]));
        image.put_pixel(2, 0, Rgba([20, 30, 40, 255]));

        apply_chroma_key(&mut image, &key, 8);

        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 0]);
        assert!(image.get_pixel(1, 0)[3] > 0 && image.get_pixel(1, 0)[3] < 255);
        assert_eq!(image.get_pixel(2, 0).0, [20, 30, 40, 255]);
    }

    #[test]
    fn normalizes_image_bytes_to_a_default_horizontal_row() {
        let source = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 255, 255]));
        let key = CHROMA_KEY_CANDIDATES[0];

        let normalized = normalize_horizontal_row(&png_bytes(&source), &key).unwrap();

        assert_eq!(
            normalized.dimensions(),
            (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H)
        );
        assert_eq!(normalized.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn normalizes_a_canonical_base_to_one_nonempty_frame() {
        let mut source = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 255, 255]));
        source.put_pixel(8, 8, Rgba([20, 30, 40, 255]));
        let key = CHROMA_KEY_CANDIDATES[0];

        let normalized = normalize_base_image(&png_bytes(&source), &key).unwrap();

        assert_eq!(normalized.dimensions(), (FRAME_W, FRAME_H));
        assert!(normalized.pixels().any(|pixel| pixel[3] != 0));
        assert_eq!(normalized.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn rejects_a_canonical_base_that_is_empty_after_keying() {
        let source = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 255, 255]));

        let error =
            normalize_base_image(&png_bytes(&source), &CHROMA_KEY_CANDIDATES[0]).unwrap_err();

        assert!(error.contains("empty"));
    }

    #[test]
    fn resolves_only_known_manifest_chroma_hexes() {
        assert_eq!(chroma_key_from_hex("#00FFFF").unwrap().name, "cyan");
        assert!(chroma_key_from_hex("#123456").is_err());
    }

    #[test]
    fn rejects_malformed_image_bytes_before_normalizing() {
        let error =
            normalize_horizontal_row(b"not an image", &CHROMA_KEY_CANDIDATES[0]).unwrap_err();

        assert!(error.contains("decode image"));
    }

    #[test]
    fn normalizes_rgb_when_a_near_key_ramp_rounds_alpha_to_zero() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut image = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 245, 1]));

        apply_chroma_key(&mut image, &key, 8);

        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn near_key_alpha_ramp_reaches_zero_at_the_threshold_boundary() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut image = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 247, 255]));

        apply_chroma_key(&mut image, &key, 8);

        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn rejects_wrong_dimensions_and_accepts_a_valid_nonempty_row() {
        let wrong_width = RgbaImage::from_pixel(1023, 128, Rgba([1, 2, 3, 255]));
        let valid = opaque_row([1, 2, 3, 255]);

        let error = validate_sprite_row(&wrong_width, 128, 128, 8).unwrap_err();
        assert!(error.contains("dimensions"));
        validate_sprite_row(&valid, 128, 128, 8).unwrap();
    }

    #[test]
    fn rejects_zero_frame_count_even_when_zero_width_matches() {
        let zero_frame_image = RgbaImage::new(0, FRAME_H);

        let error = validate_sprite_row(&zero_frame_image, FRAME_W, FRAME_H, 0).unwrap_err();

        assert!(error.contains("frame count"));
    }

    #[test]
    fn rejects_a_fully_transparent_frame_but_allows_transparent_pixels_elsewhere() {
        let mut row = opaque_row([1, 2, 3, 255]);
        for y in 0..FRAME_H {
            for x in FRAME_W..(FRAME_W * 2) {
                row.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        row.put_pixel(0, 0, Rgba([0, 0, 0, 0]));

        let error = validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT).unwrap_err();
        assert!(error.contains("empty"));
    }

    #[test]
    fn assembles_four_rows_into_a_zero_gap_four_state_sheet() {
        let rows = vec![
            opaque_row([255, 0, 0, 255]),
            opaque_row([0, 255, 0, 255]),
            opaque_row([0, 0, 255, 255]),
            opaque_row([255, 255, 0, 255]),
        ];

        let combined = assemble_rows(&rows, FRAME_W, FRAME_H).unwrap();

        assert_eq!(combined.dimensions(), (1024, 512));
        assert_eq!(combined.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(combined.get_pixel(0, 128).0, [0, 255, 0, 255]);
        assert_eq!(combined.get_pixel(0, 384).0, [255, 255, 0, 255]);
    }

    #[test]
    fn rejects_wrong_row_count_and_row_size() {
        let rows = vec![opaque_row([1, 2, 3, 255]); 3];
        let error = assemble_rows(&rows, FRAME_W, FRAME_H).unwrap_err();
        assert!(error.contains("row count"));

        let rows = vec![
            opaque_row([1, 2, 3, 255]),
            opaque_row([1, 2, 3, 255]),
            opaque_row([1, 2, 3, 255]),
            RgbaImage::from_pixel(1023, FRAME_H, Rgba([1, 2, 3, 255])),
        ];
        let error = assemble_rows(&rows, FRAME_W, FRAME_H).unwrap_err();
        assert!(error.contains("dimensions"));
    }

    #[test]
    fn encodes_a_png_data_url_that_decodes_back_to_a_png() {
        let image = RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 255]));

        let data_url = image_to_data_url(&image).unwrap();
        assert!(data_url.starts_with("data:image/png;base64,"));

        let encoded = data_url.strip_prefix("data:image/png;base64,").unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.dimensions(), (2, 2));
    }
}
