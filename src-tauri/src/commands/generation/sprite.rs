use super::types::{
    state_definitions, API_FRAME_H, API_FRAME_W, DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W,
};
use base64::Engine as _;
use image::{imageops::FilterType, DynamicImage, Rgba, RgbaImage};
use std::collections::VecDeque;
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

fn for_each_neighbor<F>(x: u32, y: u32, width: u32, height: u32, mut visit: F)
where
    F: FnMut(u32, u32),
{
    if width == 0 || height == 0 {
        return;
    }
    let min_x = x.saturating_sub(1);
    let max_x = x.saturating_add(1).min(width - 1);
    let min_y = y.saturating_sub(1);
    let max_y = y.saturating_add(1).min(height - 1);
    for neighbor_y in min_y..=max_y {
        for neighbor_x in min_x..=max_x {
            if neighbor_x != x || neighbor_y != y {
                visit(neighbor_x, neighbor_y);
            }
        }
    }
}

fn enqueue_if_unseen(
    queue: &mut VecDeque<(u32, u32)>,
    queued: &mut [bool],
    x: u32,
    y: u32,
    width: u32,
) {
    let idx = (y * width + x) as usize;
    if !queued[idx] {
        queued[idx] = true;
        queue.push_back((x, y));
    }
}

fn push_neighbors(
    queue: &mut VecDeque<(u32, u32)>,
    queued: &mut [bool],
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) {
    for_each_neighbor(x, y, width, height, |neighbor_x, neighbor_y| {
        enqueue_if_unseen(queue, queued, neighbor_x, neighbor_y, width);
    });
}

fn background_alpha(
    pixel: &Rgba<u8>,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> Option<u8> {
    if pixel[3] == 0 {
        return Some(0);
    }

    let distance = (squared_rgb_distance([pixel[0], pixel[1], pixel[2]], actual_bg) as f32).sqrt();
    if distance > ramp_end {
        return None;
    }
    if distance <= threshold {
        return Some(0);
    }

    let ratio = (distance - threshold) / (ramp_end - threshold);
    Some((f32::from(pixel[3]) * ratio).round() as u8)
}

fn apply_background_alpha(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> bool {
    let pixel = *image.get_pixel(x, y);
    let Some(alpha) = background_alpha(&pixel, actual_bg, threshold, ramp_end) else {
        return false;
    };

    if alpha == 0 {
        image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
    } else {
        image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], alpha]));
    }
    true
}

fn is_background_like(pixel: &Rgba<u8>, actual_bg: [u8; 3], threshold: f32, ramp_end: f32) -> bool {
    pixel[3] > 0 && background_alpha(pixel, actual_bg, threshold, ramp_end).is_some()
}

/// Samples the actual background colour from an outer border strip and returns
/// the per-channel median. Median is robust to a character occasionally
/// reaching into the border strip; only >50% intrusion would poison it.
fn sample_border_color(image: &RgbaImage) -> Option<[u8; 3]> {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return None;
    }
    const STRIP: u32 = 4;
    let strip = STRIP.min(width).min(height);
    if strip == 0 {
        return None;
    }

    let mut reds: Vec<u8> = Vec::new();
    let mut greens: Vec<u8> = Vec::new();
    let mut blues: Vec<u8> = Vec::new();

    let collect = |x: u32, y: u32, buffer: &RgbaImage,
                   reds: &mut Vec<u8>,
                   greens: &mut Vec<u8>,
                   blues: &mut Vec<u8>| {
        let [r, g, b, a] = buffer.get_pixel(x, y).0;
        if a > 0 {
            reds.push(r);
            greens.push(g);
            blues.push(b);
        }
    };

    for y in 0..strip {
        for x in 0..width {
            collect(x, y, image, &mut reds, &mut greens, &mut blues);
        }
    }
    let bottom_start = height.saturating_sub(strip);
    for y in bottom_start.max(strip)..height {
        for x in 0..width {
            collect(x, y, image, &mut reds, &mut greens, &mut blues);
        }
    }
    let side_y_range = strip..height.saturating_sub(strip);
    for y in side_y_range {
        for x in 0..strip {
            collect(x, y, image, &mut reds, &mut greens, &mut blues);
        }
        let right_start = width.saturating_sub(strip);
        for x in right_start.max(strip)..width {
            collect(x, y, image, &mut reds, &mut greens, &mut blues);
        }
    }

    if reds.is_empty() {
        return None;
    }
    reds.sort_unstable();
    greens.sort_unstable();
    blues.sort_unstable();
    let mid = reds.len() / 2;
    Some([reds[mid], greens[mid], blues[mid]])
}

/// Removes the chroma-key background via 8-connected BFS from the image edges,
/// then cleans up small enclosed near-background holes.
///
/// The actual background colour is sampled from a border strip using per-channel
/// median (robust to a character intruding into the strip). BFS then removes
/// only pixels connected to the border that are close to that sampled colour,
/// while enclosed cleanup removes only small components.
pub fn remove_chroma_background(image: &mut RgbaImage, key: &ChromaKey) {
    const FILL_THRESHOLD: f32 = 110.0;
    const RAMP_WIDTH: f32 = 30.0;
    let ramp_end = FILL_THRESHOLD + RAMP_WIDTH;
    let width = image.width();
    let height = image.height();

    if width == 0 || height == 0 {
        return;
    }

    // Trust the sampled border colour unconditionally: AI models routinely
    // ignore the prompted chroma-key hue and produce off-hue (white / plain /
    // natural) backgrounds instead, and matching the ideal key would then
    // remove nothing. BFS from the edges still protects any interior pixel not
    // connected to the border, so a character colour that happens to be near
    // the sampled background is preserved unless it touches the frame edge.
    let actual_bg = sample_border_color(image).unwrap_or(key.rgb);

    let mut processed = vec![false; (width * height) as usize];
    let mut queued = vec![false; (width * height) as usize];
    let mut queue: VecDeque<(u32, u32)> = VecDeque::new();

    for x in 0..width {
        enqueue_if_unseen(&mut queue, &mut queued, x, 0, width);
        if height > 1 {
            enqueue_if_unseen(&mut queue, &mut queued, x, height - 1, width);
        }
    }
    for y in 1..height.saturating_sub(1) {
        enqueue_if_unseen(&mut queue, &mut queued, 0, y, width);
        if width > 1 {
            enqueue_if_unseen(&mut queue, &mut queued, width - 1, y, width);
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        let idx = (y * width + x) as usize;
        if processed[idx] {
            continue;
        }
        processed[idx] = true;

        if apply_background_alpha(image, x, y, actual_bg, FILL_THRESHOLD, ramp_end) {
            push_neighbors(&mut queue, &mut queued, x, y, width, height);
        }
    }

    remove_interior_background_holes(image, actual_bg, &processed, FILL_THRESHOLD, ramp_end);

    for pixel in image.pixels_mut() {
        if pixel[3] == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
}

fn remove_interior_background_holes(
    image: &mut RgbaImage,
    actual_bg: [u8; 3],
    edge_processed: &[bool],
    threshold: f32,
    ramp_end: f32,
) {
    const MAX_COMPONENT_SIZE: usize = 4096;
    let width = image.width();
    let height = image.height();
    let mut visited = vec![false; (width * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let start_idx = (y * width + x) as usize;
            if edge_processed[start_idx]
                || visited[start_idx]
                || !is_background_like(image.get_pixel(x, y), actual_bg, threshold, ramp_end)
            {
                continue;
            }

            let mut region = Vec::new();
            let mut queue = VecDeque::new();
            let mut touches_edge = false;
            let mut oversized = false;
            visited[start_idx] = true;
            queue.push_back((x, y));

            while let Some((current_x, current_y)) = queue.pop_front() {
                let current_idx = (current_y * width + current_x) as usize;
                if edge_processed[current_idx]
                    || !is_background_like(
                        image.get_pixel(current_x, current_y),
                        actual_bg,
                        threshold,
                        ramp_end,
                    )
                {
                    continue;
                }

                touches_edge |= current_x == 0
                    || current_y == 0
                    || current_x == width - 1
                    || current_y == height - 1;
                if region.len() < MAX_COMPONENT_SIZE {
                    region.push((current_x, current_y));
                } else {
                    oversized = true;
                }

                for_each_neighbor(current_x, current_y, width, height, |neighbor_x, neighbor_y| {
                    let neighbor_idx = (neighbor_y * width + neighbor_x) as usize;
                    if edge_processed[neighbor_idx] || visited[neighbor_idx] {
                        return;
                    }
                    visited[neighbor_idx] = true;
                    if is_background_like(
                        image.get_pixel(neighbor_x, neighbor_y),
                        actual_bg,
                        threshold,
                        ramp_end,
                    ) {
                        queue.push_back((neighbor_x, neighbor_y));
                    }
                });
            }

            if !touches_edge && !oversized {
                for (region_x, region_y) in region {
                    apply_background_alpha(
                        image, region_x, region_y, actual_bg, threshold, ramp_end,
                    );
                }
            }
        }
    }
}

pub fn normalize_horizontal_row(bytes: &[u8], key: &ChromaKey) -> Result<RgbaImage, String> {
    let decoded =
        image::load_from_memory(bytes).map_err(|error| format!("decode image: {error}"))?;
    let mut source = decoded.to_rgba8();
    remove_chroma_background(&mut source, key);
    Ok(slice_row_into_frames(&source, DEFAULT_FRAME_COUNT))
}

/// Splits a raw row image (background already removed) into `frame_count`
/// output frames of `FRAME_W`×`FRAME_H` each. All slots use a shared crop,
/// scale, and bottom-aligned baseline transform so local furniture coordinates
/// are preserved across frames.
fn slice_row_into_frames(source: &RgbaImage, frame_count: u32) -> RgbaImage {
    let dst_width = FRAME_W * frame_count;
    let mut dst = RgbaImage::new(dst_width, FRAME_H);
    if frame_count == 0 {
        return dst;
    }

    let Some((x_min, y_min, x_max, y_max)) = find_visible_bounds(source) else {
        return dst;
    };

    let content_w = x_max.saturating_sub(x_min).saturating_add(1);
    let content_h = y_max.saturating_sub(y_min).saturating_add(1);
    let segment_w = content_w
        .saturating_add(frame_count.saturating_sub(1))
        .checked_div(frame_count)
        .unwrap_or(1)
        .max(1);

    // Shared vertical scale so every character has the same on-screen height.
    // The horizontal cap keeps a wide-outlier frame reaching outward
    // from breaking the 128-wide destination frame.
    let scale_v = f64::from(FRAME_H) / f64::from(content_h);
    let scale_h = f64::from(FRAME_W) / f64::from(segment_w);
    let global_scale = scale_v.min(scale_h);
    let content_end = x_min.saturating_add(content_w);

    for i in 0..frame_count {
        let seg_x_start = x_min.saturating_add(i.saturating_mul(segment_w));
        if seg_x_start >= content_end {
            // Blank segment: leave the destination frame transparent so the
            // caller's validate_sprite_row surfaces a specific empty-frame error.
            continue;
        }

        let available_w = (content_end - seg_x_start).min(segment_w);
        let cropped = image::imageops::crop_imm(source, seg_x_start, y_min, available_w, content_h)
            .to_image();
        let mut padded = RgbaImage::new(segment_w, content_h);
        for y in 0..content_h {
            for x in 0..available_w {
                padded.put_pixel(x, y, *cropped.get_pixel(x, y));
            }
        }

        let scaled_w = ((f64::from(segment_w) * global_scale).round() as u32)
            .max(1)
            .min(FRAME_W);
        let scaled_h = ((f64::from(content_h) * global_scale).round() as u32)
            .max(1)
            .min(FRAME_H);
        let scaled = DynamicImage::ImageRgba8(padded)
            .resize_exact(scaled_w, scaled_h, FilterType::Lanczos3)
            .to_rgba8();

        let dst_frame_x = i.saturating_mul(FRAME_W);
        let dst_x_offset = (FRAME_W - scaled_w) / 2;
        let dst_y_offset = FRAME_H - scaled_h;
        image::imageops::overlay(
            &mut dst,
            &scaled,
            i64::from(dst_frame_x.saturating_add(dst_x_offset)),
            i64::from(dst_y_offset),
        );
    }

    dst
}

fn find_visible_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut x_min = image.width();
    let mut x_max = 0u32;
    let mut y_min = image.height();
    let mut y_max = 0u32;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        found = true;
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }

    found.then_some((x_min, y_min, x_max, y_max))
}

/// Wanxiang / DashScope image-edit models require each input dimension to be in
/// the range 512..=4096 pixels. Two strategies handle the shape mismatch:
///
///   * If a uniform integer upscale can hit MIN on the short side without
///     blowing past MAX on the long side, upscale (nearest-neighbour keeps
///     tiled art crisp).
///   * Otherwise pad the short side with a solid chroma-key background so the
///     reference reaches MIN without stretching the tiled characters. The row
///     slicer downstream ignores the padding via `find_visible_bounds`.
pub fn ensure_wanxiang_reference_size(image: &RgbaImage, key: &ChromaKey) -> RgbaImage {
    const MIN_DIM: u32 = 512;
    const MAX_DIM: u32 = 4096;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return image.clone();
    }

    let min_dim = width.min(height);
    if min_dim < MIN_DIM {
        let scale = (MIN_DIM + min_dim - 1) / min_dim;
        let scaled_w = width.saturating_mul(scale);
        let scaled_h = height.saturating_mul(scale);
        if scaled_w <= MAX_DIM && scaled_h <= MAX_DIM {
            return DynamicImage::ImageRgba8(image.clone())
                .resize_exact(scaled_w, scaled_h, FilterType::Nearest)
                .to_rgba8();
        }
    } else if width <= MAX_DIM && height <= MAX_DIM {
        return image.clone();
    }

    let target_w = width.max(MIN_DIM).min(MAX_DIM);
    let target_h = height.max(MIN_DIM).min(MAX_DIM);
    if width > target_w || height > target_h {
        // Source already exceeds MAX on some axis — nothing we can do without
        // cropping. Return unchanged and let wanxiang surface the exact error.
        return image.clone();
    }

    let background = Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]);
    let mut padded = RgbaImage::from_pixel(target_w, target_h, background);
    let offset_x = ((target_w - width) / 2) as i64;
    let offset_y = ((target_h - height) / 2) as i64;
    image::imageops::overlay(&mut padded, image, offset_x, offset_y);
    padded
}

pub fn build_row_reference(base: &RgbaImage) -> RgbaImage {
    // Base is already at API_FRAME_W × API_FRAME_H after normalize_base_image;
    // just tile it 8 times side-by-side, no upscaling.
    let mut reference = RgbaImage::new(API_FRAME_W * DEFAULT_FRAME_COUNT, API_FRAME_H);

    for frame_index in 0..DEFAULT_FRAME_COUNT {
        for y in 0..API_FRAME_H {
            for x in 0..API_FRAME_W {
                reference.put_pixel(
                    frame_index * API_FRAME_W + x,
                    y,
                    *base.get_pixel(x, y),
                );
            }
        }
    }

    reference
}

pub fn normalize_base_image(bytes: &[u8], key: &ChromaKey) -> Result<RgbaImage, String> {
    let decoded =
        image::load_from_memory(bytes).map_err(|error| format!("decode image: {error}"))?;
    // Remove background at source resolution first so Lanczos3 doesn't blend
    // background colour into character edge pixels before keying.
    let mut source = decoded.to_rgba8();
    remove_chroma_background(&mut source, key);
    if !source.pixels().any(|pixel| pixel[3] != 0) {
        return Err("canonical base image is empty after chroma keying".to_string());
    }
    // Store at API-resolution (256×256) so the row-reference sheet doesn't have
    // to upscale a downsampled base again — preserves identity detail.
    let base = DynamicImage::ImageRgba8(source)
        .resize_exact(API_FRAME_W, API_FRAME_H, FilterType::Lanczos3)
        .to_rgba8();
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
        apply_chroma_key, assemble_rows, build_row_reference, choose_chroma_key,
        chroma_key_from_hex, image_to_data_url, normalize_base_image, normalize_horizontal_row,
        enqueue_if_unseen, for_each_neighbor, remove_chroma_background, slice_row_into_frames,
        validate_sprite_row,
        ChromaKey,
        CHROMA_KEY_CANDIDATES,
    };
    use crate::commands::generation::types::{
        API_FRAME_H, API_FRAME_W, DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W,
    };
    use base64::Engine;
    use image::{GenericImageView, ImageFormat, Rgba, RgbaImage};
    use std::collections::VecDeque;
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
    fn builds_a_wide_reference_with_one_base_pose_per_api_slot() {
        let base = RgbaImage::from_pixel(API_FRAME_W, API_FRAME_H, Rgba([20, 30, 40, 255]));

        let reference = build_row_reference(&base);

        assert_eq!(
            reference.dimensions(),
            (API_FRAME_W * DEFAULT_FRAME_COUNT, API_FRAME_H)
        );
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let center_x = frame_index * API_FRAME_W + API_FRAME_W / 2;
            assert_eq!(
                reference.get_pixel(center_x, API_FRAME_H / 2).0,
                [20, 30, 40, 255]
            );
        }
    }

    #[test]
    fn crops_vertical_letterboxing_before_normalizing_a_wide_row() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut source = RgbaImage::from_pixel(
            API_FRAME_W * DEFAULT_FRAME_COUNT,
            1024,
            Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
        );
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            for x in (frame_index * 256 + 64)..(frame_index * 256 + 192) {
                for y in 384..640 {
                    source.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
        }

        let normalized = normalize_horizontal_row(&png_bytes(&source), &key).unwrap();

        assert_eq!(normalized.dimensions(), (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H));
        assert!(normalized.get_pixel(FRAME_W / 2, 0)[3] > 0);
        validate_sprite_row(&normalized, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT).unwrap();
    }

    #[test]
    fn normalizes_a_canonical_base_to_one_nonempty_frame() {
        let mut source = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 255, 255]));
        source.put_pixel(8, 8, Rgba([20, 30, 40, 255]));
        let key = CHROMA_KEY_CANDIDATES[0];

        let normalized = normalize_base_image(&png_bytes(&source), &key).unwrap();

        assert_eq!(normalized.dimensions(), (API_FRAME_W, API_FRAME_H));
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
    fn enumerates_each_bounded_eight_connected_neighbor_once() {
        let mut neighbors = Vec::new();

        for_each_neighbor(1, 1, 3, 3, |x, y| neighbors.push((x, y)));

        assert_eq!(
            neighbors,
            vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (0, 1),
                (2, 1),
                (0, 2),
                (1, 2),
                (2, 2),
            ]
        );
    }

    #[test]
    fn enqueues_each_edge_coordinate_only_once() {
        let mut queue = VecDeque::new();
        let mut queued = vec![false; 9];

        enqueue_if_unseen(&mut queue, &mut queued, 1, 1, 3);
        enqueue_if_unseen(&mut queue, &mut queued, 1, 1, 3);

        assert_eq!(queue.into_iter().collect::<Vec<_>>(), vec![(1, 1)]);
    }

    #[test]
    fn removes_a_diagonal_background_gap() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let foreground = Rgba([20, 30, 40, 255]);
        let key_pixel = Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]);
        let mut image = RgbaImage::from_pixel(70, 70, key_pixel);
        for y in 1..=68 {
            for x in 1..=68 {
                if x == 1 || x == 68 || y == 1 || y == 68 {
                    image.put_pixel(x, y, foreground);
                }
            }
        }
        image.put_pixel(1, 1, key_pixel);
        for (x, y) in [(0, 1), (1, 0), (1, 2), (2, 1), (0, 2), (2, 0)] {
            image.put_pixel(x, y, foreground);
        }

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(34, 34).0, [0, 0, 0, 0]);
    }

    #[test]
    fn removes_a_small_enclosed_background_hole_inside_a_foreground_ring() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let foreground = Rgba([20, 30, 40, 255]);
        let mut image =
            RgbaImage::from_pixel(9, 9, Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]));
        for y in 3..=5 {
            for x in 3..=5 {
                if x == 3 || x == 5 || y == 3 || y == 5 {
                    image.put_pixel(x, y, foreground);
                }
            }
        }

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(4, 4).0, [0, 0, 0, 0]);
    }

    #[test]
    fn preserves_a_large_enclosed_background_component() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let foreground = Rgba([20, 30, 40, 255]);
        let key_pixel = Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]);
        let mut image = RgbaImage::from_pixel(70, 70, key_pixel);
        for y in 1..=68 {
            for x in 1..=68 {
                if x == 1 || x == 68 || y == 1 || y == 68 {
                    image.put_pixel(x, y, foreground);
                }
            }
        }

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(34, 34).0, key_pixel.0);
    }

    #[test]
    fn zeroes_rgb_for_preexisting_transparent_pixels() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut image =
            RgbaImage::from_pixel(6, 6, Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]));
        for y in 1..=4 {
            for x in 1..=4 {
                image.put_pixel(x, y, Rgba([20, 30, 40, 255]));
            }
        }
        image.put_pixel(3, 3, Rgba([123, 45, 67, 0]));

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(3, 3).0, [0, 0, 0, 0]);
    }

    #[test]
    fn rejects_wrong_dimensions_and_accepts_a_valid_nonempty_row() {
        let wrong_width =
            RgbaImage::from_pixel(FRAME_W * DEFAULT_FRAME_COUNT - 1, FRAME_H, Rgba([1, 2, 3, 255]));
        let valid = opaque_row([1, 2, 3, 255]);

        let error = validate_sprite_row(&wrong_width, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT)
            .unwrap_err();
        assert!(error.contains("dimensions"));
        validate_sprite_row(&valid, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT).unwrap();
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

        assert_eq!(
            combined.dimensions(),
            (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H * 4)
        );
        assert_eq!(combined.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(combined.get_pixel(0, FRAME_H).0, [0, 255, 0, 255]);
        assert_eq!(combined.get_pixel(0, FRAME_H * 3).0, [255, 255, 0, 255]);
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
            RgbaImage::from_pixel(FRAME_W * DEFAULT_FRAME_COUNT - 1, FRAME_H, Rgba([1, 2, 3, 255])),
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

    #[test]
    fn shared_transform_preserves_frame_local_scene_coordinates() {
        let mut source = RgbaImage::new(API_FRAME_W * DEFAULT_FRAME_COUNT, API_FRAME_H);
        let foreground = Rgba([20, 30, 40, 255]);
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let frame_x = frame_index * API_FRAME_W;
            let desk_start = if frame_index == 3 { 64 } else { 32 };
            let body_start = if frame_index == 3 { 112 } else { 80 };
            for x in desk_start..desk_start + 192 {
                for y in 160..=168 {
                    source.put_pixel(frame_x + x, y, foreground);
                }
            }
            for x in body_start..body_start + 64 {
                for y in 32..=160 {
                    source.put_pixel(frame_x + x, y, foreground);
                }
            }
        }
        source.put_pixel(0, 0, foreground);
        source.put_pixel(source.width() - 1, 0, foreground);

        let normalized = slice_row_into_frames(&source, DEFAULT_FRAME_COUNT);

        assert_eq!(
            normalized.dimensions(),
            (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H)
        );

        // Lanczos3 preserves a low-alpha fringe outside hard source edges;
        // measure the near-opaque core to validate the shared transform.
        let is_solid_core = |pixel: &Rgba<u8>| pixel[3] >= 200;
        let desk_left_edge = |frame_index: u32| {
            let frame_x = frame_index * FRAME_W;
            let (densest_y, _) = (96..FRAME_H)
                .map(|y| {
                    let opaque_pixels = (frame_x..frame_x + FRAME_W)
                        .filter(|&x| is_solid_core(normalized.get_pixel(x, y)))
                        .count();
                    (y, opaque_pixels)
                })
                .max_by_key(|&(_, opaque_pixels)| opaque_pixels)
                .expect("desk search range should not be empty");
            (frame_x..frame_x + FRAME_W)
                .find(|&x| is_solid_core(normalized.get_pixel(x, densest_y)))
                .map(|x| x - frame_x)
                .expect("desk row should have visible pixels")
        };

        assert_eq!(desk_left_edge(0), 16);
        assert_eq!(desk_left_edge(3), 32);

        let body_center = |frame_index: u32| {
            let frame_x = frame_index * FRAME_W;
            let mut min_x = FRAME_W;
            let mut max_x = 0;
            for y in 56..112 {
                for x in frame_x..frame_x + FRAME_W {
                    if is_solid_core(normalized.get_pixel(x, y)) {
                        let local_x = x - frame_x;
                        min_x = min_x.min(local_x);
                        max_x = max_x.max(local_x);
                    }
                }
            }
            (min_x + max_x) / 2
        };

        assert!(body_center(3) > body_center(0) + 8);
    }

    #[test]
    fn per_frame_slicing_uses_a_shared_baseline_across_frames_of_different_heights() {
        // Two "tall" characters (arm raised, occupying 200 rows) and six shorter
        // ones (100 rows) in the same row. All should sit on the same ground
        // line — the bottom-most opaque pixel of every frame must match.
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut source = RgbaImage::from_pixel(
            API_FRAME_W * DEFAULT_FRAME_COUNT,
            API_FRAME_H,
            Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
        );
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let base_x = frame_index * 256 + 96;
            let tall = frame_index % 4 == 0;
            let y_start = if tall { 40 } else { 140 };
            for x in base_x..(base_x + 64) {
                for y in y_start..240 {
                    source.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
        }

        let normalized =
            normalize_horizontal_row(&png_bytes(&source), &key).unwrap();

        let mut per_frame_bottom = Vec::new();
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let start_x = frame_index * FRAME_W;
            let mut bottom = None;
            for y in (0..FRAME_H).rev() {
                if (start_x..start_x + FRAME_W)
                    .any(|x| normalized.get_pixel(x, y)[3] > 0)
                {
                    bottom = Some(y);
                    break;
                }
            }
            per_frame_bottom.push(bottom.expect("frame should have visible pixels"));
        }

        assert_eq!(per_frame_bottom[0], FRAME_H - 1);
        for value in &per_frame_bottom {
            assert_eq!(
                *value, per_frame_bottom[0],
                "every frame must share the same baseline row"
            );
        }
    }

    #[test]
    fn border_sampling_survives_a_character_intruding_into_one_corner() {
        // Solid green background with a small dark blob in the top-left corner.
        // Corner-mean would pull the sampled bg toward the blob; the median
        // survives because <50% of border pixels are the intruder.
        let key = ChromaKey {
            name: "green",
            hex: "#00FF00",
            rgb: [0, 255, 0],
        };
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 255, 0, 255]));
        for y in 0..3 {
            for x in 0..3 {
                image.put_pixel(x, y, Rgba([20, 30, 40, 255]));
            }
        }

        remove_chroma_background(&mut image, &key);

        // Every green pixel should be gone; the intruder should be preserved.
        for y in 8..56 {
            for x in 8..56 {
                assert_eq!(
                    image.get_pixel(x, y).0,
                    [0, 0, 0, 0],
                    "background pixel at ({x},{y}) should be removed"
                );
            }
        }
        assert_eq!(image.get_pixel(1, 1).0, [20, 30, 40, 255]);
    }
}
