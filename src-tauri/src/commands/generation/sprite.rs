use super::types::{
    state_definitions, AnimationProbeValidation, API_FRAME_H, API_FRAME_W, DEFAULT_FRAME_COUNT,
    FRAME_H, FRAME_W,
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
    states: &mut [u8],
    x: u32,
    y: u32,
    width: u32,
    state: u8,
) {
    let idx = (y * width + x) as usize;
    if states[idx] == 0 {
        states[idx] = state;
        queue.push_back((x, y));
    }
}

fn push_neighbors(
    queue: &mut VecDeque<(u32, u32)>,
    states: &mut [u8],
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) {
    for_each_neighbor(x, y, width, height, |neighbor_x, neighbor_y| {
        enqueue_if_unseen(queue, states, neighbor_x, neighbor_y, width, EDGE_SEEN);
    });
}

fn background_alpha(
    pixel: &Rgba<u8>,
    color: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> Option<u8> {
    if pixel[3] == 0 {
        return Some(0);
    }

    let distance = (squared_rgb_distance([pixel[0], pixel[1], pixel[2]], color) as f32).sqrt();
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
    color: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> bool {
    let pixel = *image.get_pixel(x, y);
    let Some(alpha) = background_alpha(&pixel, color, threshold, ramp_end) else {
        return false;
    };

    if alpha == 0 {
        image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
    } else {
        image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], alpha]));
    }
    true
}

fn is_background_like(
    pixel: &Rgba<u8>,
    color: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> bool {
    pixel[3] > 0 && background_alpha(pixel, color, threshold, ramp_end).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackgroundKind {
    Key,
    Sampled,
}

const EDGE_SEEN: u8 = 1;
const INTERIOR_SEEN: u8 = 2;
const OVERSIZED_COMPONENT: u8 = 4;
const KEY_THRESHOLD: f32 = 8.0;
const KEY_RAMP_END: f32 = 24.0;
const ENCLOSED_THRESHOLD: f32 = 6.0;
const ENCLOSED_RAMP_END: f32 = 8.0;
const MAX_COMPONENT_SIZE: usize = 4096;
const MAX_KEY_GAP_SIZE: usize = 16;
const MAX_KEY_GAP_WIDTH: u32 = 2;
const MAX_KEY_GAP_HEIGHT: u32 = 2;
const MAX_BACKGROUND_DECORATION_COMPONENT_SIZE: usize = 4096;
const BACKGROUND_DECORATION_SIZE_RATIO: usize = 8;
const KEY_HUE_TOLERANCE_DEGREES: f32 = 28.0;
const MIN_KEY_LIKE_CHROMA: f32 = 40.0;
const MIN_KEY_LIKE_VALUE: f32 = 0.12;

fn rgb_hue_degrees(color: [u8; 3]) -> Option<f32> {
    let red = f32::from(color[0]) / 255.0;
    let green = f32::from(color[1]) / 255.0;
    let blue = f32::from(color[2]) / 255.0;
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let chroma = max - min;

    if chroma <= f32::EPSILON {
        return None;
    }

    let hue = if max == red {
        60.0 * ((green - blue) / chroma).rem_euclid(6.0)
    } else if max == green {
        60.0 * (((blue - red) / chroma) + 2.0)
    } else {
        60.0 * (((red - green) / chroma) + 4.0)
    };

    Some(hue.rem_euclid(360.0))
}

fn is_key_color_like(pixel: &Rgba<u8>, key: &ChromaKey, key_hue: f32) -> bool {
    if pixel[3] == 0 {
        return false;
    }

    let rgb = [pixel[0], pixel[1], pixel[2]];
    let distance = (squared_rgb_distance(rgb, key.rgb) as f32).sqrt();
    if distance <= KEY_RAMP_END {
        return true;
    }

    let max = f32::from(pixel[0].max(pixel[1]).max(pixel[2])) / 255.0;
    let min = f32::from(pixel[0].min(pixel[1]).min(pixel[2])) / 255.0;
    let chroma = max - min;
    if max < MIN_KEY_LIKE_VALUE || chroma * 255.0 < MIN_KEY_LIKE_CHROMA {
        return false;
    }

    let saturation = chroma / max;
    let Some(pixel_hue) = rgb_hue_degrees(rgb) else {
        return false;
    };
    let hue_distance = (pixel_hue - key_hue)
        .abs()
        .min(360.0 - (pixel_hue - key_hue).abs());
    saturation >= 0.45 && hue_distance <= KEY_HUE_TOLERANCE_DEGREES
}

fn background_kind(
    pixel: &Rgba<u8>,
    key: &ChromaKey,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) -> Option<BackgroundKind> {
    if background_alpha(pixel, key.rgb, KEY_THRESHOLD, KEY_RAMP_END).is_some() {
        Some(BackgroundKind::Key)
    } else if is_background_like(pixel, actual_bg, threshold, ramp_end) {
        Some(BackgroundKind::Sampled)
    } else {
        None
    }
}

fn apply_background_kind(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    kind: BackgroundKind,
    key: &ChromaKey,
    actual_bg: [u8; 3],
    threshold: f32,
    ramp_end: f32,
) {
    match kind {
        BackgroundKind::Key => {
            apply_background_alpha(image, x, y, key.rgb, KEY_THRESHOLD, KEY_RAMP_END);
        }
        BackgroundKind::Sampled => {
            apply_background_alpha(image, x, y, actual_bg, threshold, ramp_end);
        }
    }
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
/// so interior character pixels sharing a similar hue are always protected.
pub fn remove_chroma_background(image: &mut RgbaImage, key: &ChromaKey) {
    const FILL_THRESHOLD: f32 = 110.0;
    const RAMP_WIDTH: f32 = 30.0;
    let ramp_end = FILL_THRESHOLD + RAMP_WIDTH;
    let width = image.width();
    let height = image.height();

    if width == 0 || height == 0 {
        return;
    }

    // Sample before mutating the image. AI models routinely
    // ignore the prompted chroma-key hue and produce off-hue (white / plain /
    // natural) backgrounds instead, and matching the ideal key would then
    // remove nothing. BFS from the edges still protects any interior pixel not
    // connected to the border, so a character colour that happens to be near
    // the sampled background is preserved unless it touches the frame edge.
    let actual_bg = sample_border_color(image).unwrap_or(key.rgb);

    // Use one state byte per pixel for both the edge traversal and the
    // enclosed-component scan. Key-colour pixels are accepted here only when
    // they are connected to the border; enclosed key-colour details are
    // handled later by the small-gap rule.
    let mut states = vec![0u8; (width * height) as usize];
    let mut queue: VecDeque<(u32, u32)> = VecDeque::new();

    for x in 0..width {
        enqueue_if_unseen(&mut queue, &mut states, x, 0, width, EDGE_SEEN);
        if height > 1 {
            enqueue_if_unseen(
                &mut queue,
                &mut states,
                x,
                height - 1,
                width,
                EDGE_SEEN,
            );
        }
    }
    for y in 1..height.saturating_sub(1) {
        enqueue_if_unseen(&mut queue, &mut states, 0, y, width, EDGE_SEEN);
        if width > 1 {
            enqueue_if_unseen(
                &mut queue,
                &mut states,
                width - 1,
                y,
                width,
                EDGE_SEEN,
            );
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        let Some(kind) = background_kind(
            image.get_pixel(x, y),
            key,
            actual_bg,
            FILL_THRESHOLD,
            ramp_end,
        ) else {
            continue;
        };

        apply_background_kind(
            image,
            x,
            y,
            kind,
            key,
            actual_bg,
            FILL_THRESHOLD,
            ramp_end,
        );
        push_neighbors(&mut queue, &mut states, x, y, width, height);
    }

    remove_interior_background_holes(image, key, actual_bg, &mut states);
    remove_exposed_key_color_regions(image, key);
    remove_small_background_decoration_islands(image);

    for pixel in image.pixels_mut() {
        if pixel[3] == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
}

fn remove_interior_background_holes(
    image: &mut RgbaImage,
    key: &ChromaKey,
    actual_bg: [u8; 3],
    states: &mut [u8],
) {
    let width = image.width();
    let height = image.height();

    for y in 0..height {
        for x in 0..width {
            let start_idx = (y * width + x) as usize;
            if states[start_idx] != 0 || image.get_pixel(x, y)[3] == 0 {
                continue;
            }
            let Some(start_kind) = background_kind(
                image.get_pixel(x, y),
                key,
                actual_bg,
                ENCLOSED_THRESHOLD,
                ENCLOSED_RAMP_END,
            ) else {
                continue;
            };

            let mut region = Vec::with_capacity(MAX_COMPONENT_SIZE);
            let mut queue = VecDeque::new();
            let mut touches_edge = false;
            let mut oversized = false;
            let mut key_count = usize::from(start_kind == BackgroundKind::Key);
            let mut sampled_count = usize::from(start_kind == BackgroundKind::Sampled);
            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;
            states[start_idx] = INTERIOR_SEEN;
            region.push((x, y));
            queue.push_back((x, y));

            while let Some((current_x, current_y)) = queue.pop_front() {
                touches_edge |= current_x == 0
                    || current_y == 0
                    || current_x == width - 1
                    || current_y == height - 1;
                min_x = min_x.min(current_x);
                max_x = max_x.max(current_x);
                min_y = min_y.min(current_y);
                max_y = max_y.max(current_y);

                if oversized {
                    continue;
                }

                for_each_neighbor(
                    current_x,
                    current_y,
                    width,
                    height,
                    |neighbor_x, neighbor_y| {
                        if oversized {
                            return;
                        }
                        let neighbor_idx = (neighbor_y * width + neighbor_x) as usize;
                        if states[neighbor_idx] & OVERSIZED_COMPONENT != 0 {
                            oversized = true;
                            return;
                        }
                        if states[neighbor_idx] != 0 {
                            return;
                        }
                        let Some(kind) = background_kind(
                            image.get_pixel(neighbor_x, neighbor_y),
                            key,
                            actual_bg,
                            ENCLOSED_THRESHOLD,
                            ENCLOSED_RAMP_END,
                        ) else {
                            return;
                        };
                        if region.len() >= MAX_COMPONENT_SIZE {
                            oversized = true;
                            return;
                        }

                        states[neighbor_idx] = INTERIOR_SEEN;
                        region.push((neighbor_x, neighbor_y));
                        queue.push_back((neighbor_x, neighbor_y));
                        key_count += usize::from(kind == BackgroundKind::Key);
                        sampled_count += usize::from(kind == BackgroundKind::Sampled);
                    },
                );
            }

            if oversized {
                // Stop expanding once the cap is reached. Mark the explored
                // frontier as protected so later scan starts conservatively
                // inherit the oversized decision without another full flood.
                for &(region_x, region_y) in &region {
                    let region_idx = (region_y * width + region_x) as usize;
                    states[region_idx] |= OVERSIZED_COMPONENT;
                }
                continue;
            }

            let key_gap_like = key_count > 0
                && sampled_count == 0
                && region.len() <= MAX_KEY_GAP_SIZE
                && (max_x - min_x + 1 <= MAX_KEY_GAP_WIDTH
                    || max_y - min_y + 1 <= MAX_KEY_GAP_HEIGHT);
            let can_remove = !touches_edge
                && ((key_count == 0 && sampled_count <= MAX_COMPONENT_SIZE) || key_gap_like);

            if can_remove {
                for (region_x, region_y) in region {
                    if let Some(kind) = background_kind(
                        image.get_pixel(region_x, region_y),
                        key,
                        actual_bg,
                        ENCLOSED_THRESHOLD,
                        ENCLOSED_RAMP_END,
                    ) {
                        apply_background_kind(
                            image,
                            region_x,
                            region_y,
                            kind,
                            key,
                            actual_bg,
                            ENCLOSED_THRESHOLD,
                            ENCLOSED_RAMP_END,
                        );
                    }
                }
            }
        }
    }
}

/// Removes noisy variants of the configured chroma-key colour when they are
/// exposed to transparency or the frame edge. Image providers can turn a
/// solid key canvas into a textured, off-hue magenta/blue background whose
/// pixels are too far away for the exact RGB distance check above. Restricting
/// this cleanup to exposed colour regions keeps intentionally key-coloured
/// details inside the character intact.
fn remove_exposed_key_color_regions(image: &mut RgbaImage, key: &ChromaKey) {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return;
    }

    let Some(key_hue) = rgb_hue_degrees(key.rgb) else {
        return;
    };
    let mut visited = vec![false; (width * height) as usize];
    let mut exposed_regions = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let start_idx = (y * width + x) as usize;
            if visited[start_idx]
                || !is_key_color_like(image.get_pixel(x, y), key, key_hue)
            {
                continue;
            }

            let mut region = Vec::new();
            let mut queue = VecDeque::new();
            let mut touches_exposure = x == 0
                || y == 0
                || x == width - 1
                || y == height - 1;
            visited[start_idx] = true;
            region.push((x, y));
            queue.push_back((x, y));

            while let Some((current_x, current_y)) = queue.pop_front() {
                for_each_neighbor(
                    current_x,
                    current_y,
                    width,
                    height,
                    |neighbor_x, neighbor_y| {
                        let neighbor_pixel = image.get_pixel(neighbor_x, neighbor_y);
                        if neighbor_pixel[3] == 0 {
                            touches_exposure = true;
                            return;
                        }

                        let neighbor_idx = (neighbor_y * width + neighbor_x) as usize;
                        if visited[neighbor_idx]
                            || !is_key_color_like(neighbor_pixel, key, key_hue)
                        {
                            return;
                        }

                        visited[neighbor_idx] = true;
                        touches_exposure |= neighbor_x == 0
                            || neighbor_y == 0
                            || neighbor_x == width - 1
                            || neighbor_y == height - 1;
                        region.push((neighbor_x, neighbor_y));
                        queue.push_back((neighbor_x, neighbor_y));
                    },
                );
            }

            if touches_exposure {
                exposed_regions.push(region);
            }
        }
    }

    for region in exposed_regions {
        for (x, y) in region {
            image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        }
    }
}

struct OpaqueComponent {
    size: usize,
    touches_edge: bool,
    touches_transparent: bool,
    pixels: Option<Vec<(u32, u32)>>,
}

/// AI image providers sometimes decorate a chroma canvas with off-colour
/// snowflakes, sparkles, or other small motifs. Once the real background has
/// been removed, each motif is an isolated opaque island in the transparent
/// background and would otherwise be mistaken for character detail.
///
/// Remove only small, edge-exposed islands that are much smaller than the
/// largest remaining opaque component. Character details remain connected to
/// the subject, while large enclosed regions are protected by the size cap and
/// the relative-size check.
fn remove_small_background_decoration_islands(image: &mut RgbaImage) {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return;
    }

    let mut visited = vec![false; (width * height) as usize];
    let mut components = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let start_idx = (y * width + x) as usize;
            if visited[start_idx] || image.get_pixel(x, y)[3] == 0 {
                continue;
            }

            let mut component = OpaqueComponent {
                size: 1,
                touches_edge: x == 0 || y == 0 || x == width - 1 || y == height - 1,
                touches_transparent: false,
                pixels: Some(vec![(x, y)]),
            };
            let mut queue = VecDeque::new();
            visited[start_idx] = true;
            queue.push_back((x, y));

            while let Some((current_x, current_y)) = queue.pop_front() {
                for_each_neighbor(
                    current_x,
                    current_y,
                    width,
                    height,
                    |neighbor_x, neighbor_y| {
                        let neighbor_pixel = image.get_pixel(neighbor_x, neighbor_y);
                        if neighbor_pixel[3] == 0 {
                            component.touches_transparent = true;
                            return;
                        }

                        let neighbor_idx = (neighbor_y * width + neighbor_x) as usize;
                        if visited[neighbor_idx] {
                            return;
                        }

                        visited[neighbor_idx] = true;
                        component.size = component.size.saturating_add(1);
                        component.touches_edge |= neighbor_x == 0
                            || neighbor_y == 0
                            || neighbor_x == width - 1
                            || neighbor_y == height - 1;
                        if let Some(pixels) = component.pixels.as_mut() {
                            if pixels.len() < MAX_BACKGROUND_DECORATION_COMPONENT_SIZE {
                                pixels.push((neighbor_x, neighbor_y));
                            } else {
                                component.pixels = None;
                            }
                        }
                        queue.push_back((neighbor_x, neighbor_y));
                    },
                );
            }

            components.push(component);
        }
    }

    let largest_component_size = components
        .iter()
        .map(|component| component.size)
        .max()
        .unwrap_or(0);
    if largest_component_size == 0 {
        return;
    }

    for component in components {
        let Some(pixels) = component.pixels else {
            continue;
        };
        let is_small_relative_to_subject = component
            .size
            .saturating_mul(BACKGROUND_DECORATION_SIZE_RATIO)
            <= largest_component_size;
        if component.touches_edge
            || !component.touches_transparent
            || component.size > MAX_BACKGROUND_DECORATION_COMPONENT_SIZE
            || !is_small_relative_to_subject
        {
            continue;
        }

        for (x, y) in pixels {
            image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        }
    }
}

pub fn normalize_horizontal_row(bytes: &[u8], key: &ChromaKey) -> Result<RgbaImage, String> {
    let decoded =
        image::load_from_memory(bytes).map_err(|error| format!("decode image: {error}"))?;
    let mut source = decoded.to_rgba8();
    remove_chroma_background(&mut source, key);
    let mut row = slice_row_into_frames(&source, DEFAULT_FRAME_COUNT);
    align_frames_to_shared_anchor(&mut row, DEFAULT_FRAME_COUNT);
    Ok(row)
}

/// Splits a raw row image (background already removed) into `frame_count`
/// output frames of `FRAME_W`×`FRAME_H` each. The visible row is partitioned
/// into fixed source slots, then every slot is resized and placed with one
/// shared scale, horizontal offset, and bottom baseline so shared furniture
/// stays aligned when a character changes width between frames.
fn slice_row_into_frames(source: &RgbaImage, frame_count: u32) -> RgbaImage {
    let Some(dst_width) = FRAME_W.checked_mul(frame_count) else {
        return RgbaImage::new(0, FRAME_H);
    };
    let mut dst = RgbaImage::new(dst_width, FRAME_H);
    if frame_count == 0 {
        return dst;
    }

    let Some((x_min, y_min, x_max, y_max)) = find_visible_bounds(source) else {
        return dst;
    };
    let Some(content_w) = x_max.checked_sub(x_min).and_then(|width| width.checked_add(1)) else {
        return dst;
    };
    let Some(content_h) = y_max.checked_sub(y_min).and_then(|height| height.checked_add(1)) else {
        return dst;
    };
    let Some(segment_w) = content_w
        .checked_add(frame_count.saturating_sub(1))
        .map(|width| width / frame_count)
    else {
        return dst;
    };
    if segment_w == 0 {
        return dst;
    }

    // One shared scale keeps the source slot geometry and all shared scene
    // elements aligned while still fitting the complete content in a frame.
    let scale_v = f64::from(FRAME_H) / f64::from(content_h);
    let scale_h = f64::from(FRAME_W) / f64::from(segment_w);
    let global_scale = scale_v.min(scale_h);
    let scaled_w = (f64::from(segment_w) * global_scale)
        .round()
        .clamp(1.0, f64::from(FRAME_W)) as u32;
    let scaled_h = (f64::from(content_h) * global_scale)
        .round()
        .clamp(1.0, f64::from(FRAME_H)) as u32;
    let dst_x_offset = (FRAME_W - scaled_w) / 2;
    let dst_y_offset = FRAME_H - scaled_h;

    for frame_index in 0..frame_count {
        let Some(slot_offset) = frame_index.checked_mul(segment_w) else {
            continue;
        };
        if slot_offset >= content_w {
            // Leave empty source slots transparent so validate_sprite_row
            // surfaces an actionable empty-frame error to the caller.
            continue;
        }

        let Some(src_x_start) = x_min.checked_add(slot_offset) else {
            continue;
        };
        let copy_w = (content_w - slot_offset).min(segment_w);
        let Some(src_x_end) = src_x_start.checked_add(copy_w) else {
            continue;
        };
        let Some(src_y_end) = y_min.checked_add(content_h) else {
            continue;
        };
        if src_x_end > source.width() || src_y_end > source.height() {
            continue;
        }

        // Keep the complete fixed-width source slot. A short final slot is
        // padded with transparency instead of being re-centered independently.
        let cropped = image::imageops::crop_imm(source, src_x_start, y_min, copy_w, content_h)
            .to_image();
        let mut slot = RgbaImage::new(segment_w, content_h);
        for (x, y, pixel) in cropped.enumerate_pixels() {
            slot.put_pixel(x, y, *pixel);
        }

        let scaled = DynamicImage::ImageRgba8(slot)
            .resize_exact(scaled_w, scaled_h, FilterType::Lanczos3)
            .to_rgba8();

        let Some(dst_frame_x) = frame_index.checked_mul(FRAME_W) else {
            continue;
        };

        for (px, py, pixel) in scaled.enumerate_pixels() {
            if pixel[3] == 0 {
                continue;
            }
            let Some(dx) = dst_frame_x
                .checked_add(dst_x_offset)
                .and_then(|x| x.checked_add(px))
            else {
                continue;
            };
            let Some(dy) = dst_y_offset.checked_add(py) else {
                continue;
            };
            if dx < dst_width && dy < FRAME_H {
                dst.put_pixel(dx, dy, *pixel);
            }
        }
    }

    dst
}

/// Removes whole-frame translation introduced by an image model while keeping
/// the pose inside each frame intact. The bottom support band (feet, desk edge,
/// chair legs) is more stable than a single edge pixel, which can be a drifting
/// chair leg in sleeping scenes, and it ignores upper-body breathing motion.
fn align_frames_to_shared_anchor(row: &mut RgbaImage, frame_count: u32) {
    if frame_count == 0 || row.height() != FRAME_H || row.width() != FRAME_W * frame_count {
        return;
    }

    let mut anchors = Vec::with_capacity(frame_count as usize);
    for frame_index in 0..frame_count {
        let frame = image::imageops::crop_imm(row, frame_index * FRAME_W, 0, FRAME_W, FRAME_H)
            .to_image();
        let Some(anchor) = frame_ground_anchor(&frame) else {
            return;
        };
        anchors.push(anchor);
    }

    let mut centers = anchors.iter().map(|(center, _)| *center).collect::<Vec<_>>();
    let mut baselines = anchors.iter().map(|(_, baseline)| *baseline).collect::<Vec<_>>();
    centers.sort_unstable();
    baselines.sort_unstable();
    let target_center = centers[centers.len() / 2];
    let target_baseline = baselines[baselines.len() / 2];

    for (frame_index, (center, baseline)) in anchors.into_iter().enumerate() {
        let start_x = frame_index as u32 * FRAME_W;
        let frame = image::imageops::crop_imm(row, start_x, 0, FRAME_W, FRAME_H).to_image();
        let Some((min_x, min_y, max_x, max_y)) = find_visible_bounds(&frame) else {
            continue;
        };
        let shift_x = (i64::from(target_center) - i64::from(center))
            .clamp(-i64::from(min_x), i64::from(FRAME_W - 1 - max_x));
        let shift_y = (i64::from(target_baseline) - i64::from(baseline))
            .clamp(-i64::from(min_y), i64::from(FRAME_H - 1 - max_y));
        let mut aligned = RgbaImage::new(FRAME_W, FRAME_H);

        for (x, y, pixel) in frame.enumerate_pixels() {
            if pixel[3] == 0 {
                continue;
            }
            let destination_x = i64::from(x) + shift_x;
            let destination_y = i64::from(y) + shift_y;
            if destination_x >= 0
                && destination_x < i64::from(FRAME_W)
                && destination_y >= 0
                && destination_y < i64::from(FRAME_H)
            {
                aligned.put_pixel(destination_x as u32, destination_y as u32, *pixel);
            }
        }

        image::imageops::replace(row, &aligned, i64::from(start_x), 0);
    }
}

fn frame_ground_anchor(frame: &RgbaImage) -> Option<(u32, u32)> {
    const MIN_ANCHOR_ALPHA: u8 = 32;
    const MIN_SUPPORT_PIXELS: usize = 4;

    // A chroma-removal edge can leave a few barely visible anti-aliased pixels
    // below the actual feet or furniture. Looking only at the lowest alpha>0
    // pixel makes that fringe look like a 10px+ scene translation. Require a
    // small horizontal support run of solid-enough pixels instead, so the
    // anchor represents a real support surface rather than one edge artifact.
    let baseline = (0..frame.height()).rev().find(|&y| {
        let support_pixels = (0..frame.width())
            .filter(|&x| frame.get_pixel(x, y)[3] >= MIN_ANCHOR_ALPHA)
            .count();
        support_pixels >= MIN_SUPPORT_PIXELS
    })?;

    let support_band_start = baseline.saturating_sub(5);
    let support_band_end = baseline
        .saturating_add(5)
        .min(frame.height().saturating_sub(1));
    let mut min_x = frame.width();
    let mut max_x = 0;
    let mut found = false;
    for y in support_band_start..=support_band_end {
        for x in 0..frame.width() {
            if frame.get_pixel(x, y)[3] == 0 {
                continue;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            found = true;
        }
    }

    found.then_some(((min_x + max_x) / 2, baseline))
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

/// Idle is a deliberate static hold, so it must reuse the canonical base
/// instead of asking a provider to redraw eight approximate copies.
pub fn build_static_sprite_row(base: &RgbaImage) -> RgbaImage {
    let frame = DynamicImage::ImageRgba8(base.clone())
        .resize_exact(FRAME_W, FRAME_H, FilterType::Lanczos3)
        .to_rgba8();
    let mut row = RgbaImage::new(FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H);

    for frame_index in 0..DEFAULT_FRAME_COUNT {
        image::imageops::replace(&mut row, &frame, i64::from(frame_index * FRAME_W), 0);
    }

    row
}

/// Converts a transparent normalized frame back to an opaque chroma canvas
/// before sending it to an image-edit provider. Providers otherwise composite
/// transparent pixels against black or an arbitrary colour, which causes the
/// background and scene lighting to jump between sequential edits.
pub fn flatten_on_chroma_background(image: &RgbaImage, key: &ChromaKey) -> RgbaImage {
    let mut flattened = RgbaImage::from_pixel(
        image.width(),
        image.height(),
        Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
    );

    for (x, y, pixel) in image.enumerate_pixels() {
        let alpha = u32::from(pixel[3]);
        if alpha == 0 {
            continue;
        }
        let inverse_alpha = 255 - alpha;
        let blend = |foreground: u8, background: u8| {
            ((u32::from(foreground) * alpha
                + u32::from(background) * inverse_alpha
                + 127)
                / 255) as u8
        };
        flattened.put_pixel(
            x,
            y,
            Rgba([
                blend(pixel[0], key.rgb[0]),
                blend(pixel[1], key.rgb[1]),
                blend(pixel[2], key.rgb[2]),
                255,
            ]),
        );
    }

    flattened
}

/// Places independently generated 256×256 animation moments into one stable
/// horizontal 8-frame row. Every frame uses the same scale and the same
/// content box; only the generated pose inside that box is allowed to vary.
/// Keeps the previous animation frame everywhere except the explicitly
/// allowed motion region. Image-edit providers may still redraw pixels outside
/// their bbox, so their full output must never become the next reference frame.
pub fn preserve_motion_region_from_generated(
    previous: &RgbaImage,
    generated: &RgbaImage,
    motion_bbox: [u32; 4],
) -> Result<RgbaImage, String> {
    if previous.dimensions() != generated.dimensions() {
        return Err(format!(
            "animation frame dimensions do not match for motion-region compositing: previous {}x{}, generated {}x{}",
            previous.width(),
            previous.height(),
            generated.width(),
            generated.height(),
        ));
    }

    let (width, height) = previous.dimensions();
    let [min_x, min_y, max_x, max_y] = motion_bbox;
    if min_x >= max_x || min_y >= max_y || min_x >= width || min_y >= height {
        return Err(format!(
            "invalid animation motion region [{min_x},{min_y},{max_x},{max_y}] for {width}x{height} frame"
        ));
    }
    let max_x = max_x.min(width);
    let max_y = max_y.min(height);
    let mut composited = previous.clone();
    for y in min_y..max_y {
        for x in min_x..max_x {
            composited.put_pixel(x, y, *generated.get_pixel(x, y));
        }
    }
    Ok(composited)
}

pub fn assemble_animation_frames(frames: &[RgbaImage]) -> Result<RgbaImage, String> {
    assemble_animation_frames_with_count(frames, DEFAULT_FRAME_COUNT)
}

pub fn assemble_animation_frames_with_count(
    frames: &[RgbaImage],
    frame_count: u32,
) -> Result<RgbaImage, String> {
    if frames.len() != frame_count as usize {
        return Err(format!(
            "animation sequence must contain exactly {frame_count} frames (received {})",
            frames.len()
        ));
    }

    // Keep the provider's full 256x256 coordinate system intact. Cropping to
    // each frame's visible bounds makes shared furniture and props recenter
    // whenever a pose changes its silhouette.
    let row_width = FRAME_W
        .checked_mul(frame_count)
        .ok_or_else(|| "animation sequence width overflow".to_string())?;
    let mut row = RgbaImage::new(row_width, FRAME_H);

    for (frame_index, frame) in frames.iter().enumerate() {
        if frame.dimensions() != (API_FRAME_W, API_FRAME_H) {
            return Err(format!(
                "animation sequence frame {frame_index} has invalid dimensions: expected {}x{}, got {}x{}",
                API_FRAME_W,
                API_FRAME_H,
                frame.width(),
                frame.height()
            ));
        }
        if find_visible_bounds(frame).is_none() {
            return Err(format!(
                "animation sequence frame {frame_index} is empty after background removal"
            ));
        }

        let scaled = DynamicImage::ImageRgba8(frame.clone())
            .resize_exact(FRAME_W, FRAME_H, FilterType::Nearest)
            .to_rgba8();
        let frame_x = frame_index as u32 * FRAME_W;
        image::imageops::replace(&mut row, &scaled, i64::from(frame_x), 0);
    }

    align_frames_to_shared_anchor(&mut row, frame_count);
    Ok(row)
}

/// Validates a short, raw API-resolution chain before it is aligned or
/// promoted to a complete sprite row. This is intentionally conservative:
/// it catches whole-scene translation and repeated frames while allowing the
/// local hand, head, or shoulder motion requested by the prompt.
pub fn validate_animated_frame_sequence(
    frames: &[RgbaImage],
    expected_frame_count: u32,
) -> Result<AnimationProbeValidation, String> {
    validate_animated_frame_sequence_with_motion_region(frames, expected_frame_count, None)
}

/// Validates an animation chain and, when the provider supports precise local
/// editing, rejects any meaningful redraw outside the allowed motion box. This
/// prevents a prop such as the working laptop display from appearing halfway
/// through a sequence after the first frame has established the scene.
pub fn validate_animated_frame_sequence_with_motion_region(
    frames: &[RgbaImage],
    expected_frame_count: u32,
    motion_bbox: Option<[u32; 4]>,
) -> Result<AnimationProbeValidation, String> {
    if frames.len() != expected_frame_count as usize {
        return Err(format!(
            "animation probe must contain exactly {expected_frame_count} frames (received {})",
            frames.len()
        ));
    }
    if frames.is_empty() {
        return Err("animation probe must contain at least one frame".to_string());
    }

    for (frame_index, frame) in frames.iter().enumerate() {
        if frame.dimensions() != (API_FRAME_W, API_FRAME_H) {
            return Err(format!(
                "animation probe frame {frame_index} has invalid dimensions: expected {}x{}, got {}x{}",
                API_FRAME_W,
                API_FRAME_H,
                frame.width(),
                frame.height()
            ));
        }
        if find_visible_bounds(frame).is_none() {
            return Err(format!(
                "animation probe frame {frame_index} is empty after background removal"
            ));
        }
    }

    const MAX_ANCHOR_DRIFT: u32 = 8;
    const CHANNEL_DELTA: u8 = 12;
    const MIN_CHANGED_PIXELS: usize = 8;
    const MAX_NON_MOTION_CHANGED_PIXELS: usize = 768;

    let anchors = frames
        .iter()
        .enumerate()
        .map(|(frame_index, frame)| {
            frame_ground_anchor(frame).ok_or_else(|| {
                format!("animation probe frame {frame_index} has no stable ground anchor")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let first_center = anchors[0].0;
    let first_baseline = anchors[0].1;
    let max_center_drift = anchors
        .iter()
        .map(|(center, _)| center.abs_diff(first_center))
        .max()
        .unwrap_or(0);
    let max_baseline_drift = anchors
        .iter()
        .map(|(_, baseline)| baseline.abs_diff(first_baseline))
        .max()
        .unwrap_or(0);
    if max_center_drift > MAX_ANCHOR_DRIFT || max_baseline_drift > MAX_ANCHOR_DRIFT {
        return Err(format!(
            "animation probe detected whole-scene drift: center {max_center_drift}px, baseline {max_baseline_drift}px"
        ));
    }

    let mut min_changed_pixels = usize::MAX;
    for frame_index in 0..frames.len().saturating_sub(1) {
        let current = &frames[frame_index];
        let next = &frames[frame_index + 1];
        if let Some([min_x, min_y, max_x, max_y]) = motion_bbox {
            let mut changed_outside_motion = 0usize;
            for y in 0..API_FRAME_H {
                for x in 0..API_FRAME_W {
                    if x >= min_x && x < max_x && y >= min_y && y < max_y {
                        continue;
                    }
                    let current_pixel = current.get_pixel(x, y);
                    let next_pixel = next.get_pixel(x, y);
                    let current_visible = current_pixel[3] > CHANNEL_DELTA;
                    let next_visible = next_pixel[3] > CHANNEL_DELTA;
                    if !current_visible && !next_visible {
                        continue;
                    }
                    if current_pixel
                        .0
                        .iter()
                        .zip(next_pixel.0.iter())
                        .any(|(left, right)| (*left).abs_diff(*right) > CHANNEL_DELTA)
                    {
                        changed_outside_motion += 1;
                    }
                }
            }
            if changed_outside_motion > MAX_NON_MOTION_CHANGED_PIXELS {
                return Err(format!(
                    "animation frames {} and {} changed {changed_outside_motion} pixels outside the motion region; a new or removed object may have appeared",
                    frame_index + 1,
                    frame_index + 2,
                ));
            }
        }
        let mut changed_pixels = 0usize;
        for (current_pixel, next_pixel) in current.pixels().zip(next.pixels()) {
            let current_visible = current_pixel[3] > CHANNEL_DELTA;
            let next_visible = next_pixel[3] > CHANNEL_DELTA;
            if !current_visible && !next_visible {
                continue;
            }
            if current_pixel
                .0
                .iter()
                .zip(next_pixel.0.iter())
                .any(|(left, right)| (*left).abs_diff(*right) > CHANNEL_DELTA)
            {
                changed_pixels += 1;
            }
        }
        min_changed_pixels = min_changed_pixels.min(changed_pixels);
        if changed_pixels < MIN_CHANGED_PIXELS {
            return Err(format!(
                "animation probe frame {} and frame {} have no visible motion",
                frame_index + 1,
                frame_index + 2
            ));
        }
    }

    Ok(AnimationProbeValidation {
        passed: true,
        max_center_drift,
        max_baseline_drift,
        min_changed_pixels: min_changed_pixels.min(u32::MAX as usize) as u32,
    })
}

/// Rejects a sequence that looks like repeated stills after normalization.
/// Independent model calls can still ignore a phase instruction and return a
/// duplicate pose; that must never be persisted as an animated state.
pub fn validate_animated_sprite_row(row: &RgbaImage) -> Result<(), String> {
    validate_sprite_row(row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT)?;

    const CHANNEL_DELTA: u8 = 12;
    const MIN_CHANGED_PIXELS: usize = 4;
    // The first and last frames may intentionally share the same pose to make
    // the loop seamless. Validate the seven internal transitions; a static
    // row still fails every one of them.
    for frame_index in 0..(DEFAULT_FRAME_COUNT - 1) {
        let next_frame_index = frame_index + 1;
        let start_x = frame_index * FRAME_W;
        let next_start_x = next_frame_index * FRAME_W;
        let mut changed_pixels = 0usize;
        for y in 0..FRAME_H {
            for x in 0..FRAME_W {
                let current = row.get_pixel(start_x + x, y);
                let next = row.get_pixel(next_start_x + x, y);
                let current_visible = current[3] > CHANNEL_DELTA;
                let next_visible = next[3] > CHANNEL_DELTA;
                if !current_visible && !next_visible {
                    continue;
                }
                if current
                    .0
                    .iter()
                    .zip(next.0.iter())
                    .any(|(left, right)| (*left).abs_diff(*right) > CHANNEL_DELTA)
                {
                    changed_pixels += 1;
                }
            }
        }
        if changed_pixels < MIN_CHANGED_PIXELS {
            return Err(format!(
                "animated sequence frame {} and frame {} have no visible motion",
                frame_index + 1,
                next_frame_index + 1
            ));
        }
    }

    Ok(())
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
        apply_chroma_key, assemble_animation_frames, assemble_animation_frames_with_count,
        assemble_rows, build_row_reference,
        build_static_sprite_row, choose_chroma_key, flatten_on_chroma_background,
        chroma_key_from_hex, image_to_data_url, normalize_base_image, normalize_horizontal_row,
        preserve_motion_region_from_generated,
        remove_chroma_background, validate_animated_frame_sequence,
        validate_animated_frame_sequence_with_motion_region, validate_animated_sprite_row,
        validate_sprite_row, ChromaKey,
        CHROMA_KEY_CANDIDATES,
    };
    use crate::commands::generation::types::{
        API_FRAME_H, API_FRAME_W, DEFAULT_FRAME_COUNT, FRAME_H, FRAME_W,
        ANIMATION_PROBE_FRAME_COUNT,
    };
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
    fn builds_idle_row_from_eight_identical_canonical_base_frames() {
        let mut base = RgbaImage::new(API_FRAME_W, API_FRAME_H);
        for x in 64..192 {
            for y in 24..240 {
                base.put_pixel(x, y, Rgba([20, 30, 40, 255]));
            }
        }

        let row = build_static_sprite_row(&base);

        assert_eq!(row.dimensions(), (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H));
        validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT).unwrap();
        for frame_index in 1..DEFAULT_FRAME_COUNT {
            for x in 0..FRAME_W {
                for y in 0..FRAME_H {
                    assert_eq!(
                        row.get_pixel(x, y),
                        row.get_pixel(frame_index * FRAME_W + x, y),
                        "idle frame {frame_index} differs at ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn flattens_transparency_onto_the_selected_opaque_chroma_background() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut frame = RgbaImage::new(3, 1);
        frame.put_pixel(1, 0, Rgba([20, 40, 60, 255]));
        frame.put_pixel(2, 0, Rgba([0, 0, 0, 128]));

        let flattened = flatten_on_chroma_background(&frame, &key);

        assert_eq!(flattened.get_pixel(0, 0).0, [255, 0, 255, 255]);
        assert_eq!(flattened.get_pixel(1, 0).0, [20, 40, 60, 255]);
        assert_eq!(flattened.get_pixel(2, 0)[3], 255);
        assert!(flattened.get_pixel(2, 0)[0] > 120);
        assert!(flattened.pixels().all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn assembles_eight_independent_frames_into_a_continuous_horizontal_sequence() {
        let mut frames = Vec::new();
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 72..184 {
                for y in 32..224 {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            let hand_x = 88 + frame_index * 8;
            for x in hand_x..hand_x + 20 {
                for y in 112..144 {
                    frame.put_pixel(x, y, Rgba([220, 80, 80, 255]));
                }
            }
            frames.push(frame);
        }

        let row = assemble_animation_frames(&frames).unwrap();

        assert_eq!(row.dimensions(), (FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H));
        validate_sprite_row(&row, FRAME_W, FRAME_H, DEFAULT_FRAME_COUNT).unwrap();
        let frame0 = image::imageops::crop_imm(&row, 0, 0, FRAME_W, FRAME_H).to_image();
        let frame1 = image::imageops::crop_imm(&row, FRAME_W, 0, FRAME_W, FRAME_H).to_image();
        assert_ne!(frame0, frame1, "distinct generated poses must remain distinct");
    }

    #[test]
    fn assembles_a_four_frame_probe_without_promoting_it_to_a_full_row() {
        let frames = (0..ANIMATION_PROBE_FRAME_COUNT)
            .map(|frame_index| {
                let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
                for x in 72..184 {
                    for y in 32..224 {
                        frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                    }
                }
                frame.put_pixel(80 + frame_index, 120, Rgba([220, 80, 80, 255]));
                frame
            })
            .collect::<Vec<_>>();

        let row = assemble_animation_frames_with_count(&frames, ANIMATION_PROBE_FRAME_COUNT)
            .unwrap();

        assert_eq!(
            row.dimensions(),
            (FRAME_W * ANIMATION_PROBE_FRAME_COUNT, FRAME_H)
        );
        validate_sprite_row(&row, FRAME_W, FRAME_H, ANIMATION_PROBE_FRAME_COUNT).unwrap();
    }

    #[test]
    fn accepts_local_motion_when_the_shared_ground_anchor_stays_fixed() {
        let mut frames = Vec::new();
        for frame_index in 0..ANIMATION_PROBE_FRAME_COUNT {
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 72..184 {
                for y in 32..224 {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            for x in 96 + frame_index * 8..112 + frame_index * 8 {
                for y in 112..128 {
                    frame.put_pixel(x, y, Rgba([220, 80, 80, 255]));
                }
            }
            frames.push(frame);
        }

        let validation = validate_animated_frame_sequence(&frames, ANIMATION_PROBE_FRAME_COUNT)
            .unwrap();

        assert!(validation.passed);
        assert_eq!(validation.max_center_drift, 0);
        assert_eq!(validation.max_baseline_drift, 0);
        assert!(validation.min_changed_pixels >= 8);
    }

    #[test]
    fn rejects_a_new_object_outside_the_motion_region_after_the_first_frame() {
        let mut frames = Vec::new();
        for frame_index in 0..ANIMATION_PROBE_FRAME_COUNT {
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 48..208 {
                for y in 40..240 {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            for x in 100 + frame_index * 4..116 + frame_index * 4 {
                for y in 160..180 {
                    frame.put_pixel(x, y, Rgba([220, 80, 80, 255]));
                }
            }
            if frame_index >= 2 {
                // The laptop display appears only after the sequence has
                // already started. It is outside the allowed hand region and
                // must be rejected as a sudden new object.
                for x in 88..168 {
                    for y in 72..132 {
                        frame.put_pixel(x, y, Rgba([80, 180, 220, 255]));
                    }
                }
            }
            frames.push(frame);
        }

        let error = validate_animated_frame_sequence_with_motion_region(
            &frames,
            ANIMATION_PROBE_FRAME_COUNT,
            Some([48, 176, 136, 224]),
        )
        .unwrap_err();

        assert!(error.contains("outside the motion region"));
    }

    #[test]
    fn preserves_previous_frame_pixels_outside_the_motion_region() {
        let previous = RgbaImage::from_pixel(API_FRAME_W, API_FRAME_H, Rgba([20, 30, 40, 255]));
        let mut generated = RgbaImage::from_pixel(
            API_FRAME_W,
            API_FRAME_H,
            Rgba([180, 190, 200, 255]),
        );
        for x in 104..136 {
            for y in 168..188 {
                generated.put_pixel(x, y, Rgba([220, 80, 80, 255]));
            }
        }

        let composited = preserve_motion_region_from_generated(
            &previous,
            &generated,
            [48, 176, 136, 224],
        )
        .unwrap();

        assert_eq!(composited.get_pixel(24, 24), previous.get_pixel(24, 24));
        assert_eq!(composited.get_pixel(120, 100), previous.get_pixel(120, 100));
        assert_eq!(composited.get_pixel(120, 176), generated.get_pixel(120, 176));
    }

    #[test]
    fn ignores_low_alpha_fringe_when_finding_ground_anchor() {
        let mut frames = Vec::new();
        for frame_index in 0..ANIMATION_PROBE_FRAME_COUNT {
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 72..184 {
                for y in 32..224 {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            for x in 96 + frame_index * 8..112 + frame_index * 8 {
                for y in 112..128 {
                    frame.put_pixel(x, y, Rgba([220, 80, 80, 255]));
                }
            }
            if frame_index == 0 {
                // A chroma-removal fringe can leave a barely visible tail
                // below the real support. It must not become the baseline.
                frame.put_pixel(128, 236, Rgba([20, 30, 40, 8]));
            }
            frames.push(frame);
        }

        let validation = validate_animated_frame_sequence(&frames, ANIMATION_PROBE_FRAME_COUNT)
            .expect("low-alpha fringe should not count as whole-scene drift");

        assert_eq!(validation.max_center_drift, 0);
        assert_eq!(validation.max_baseline_drift, 0);
    }

    #[test]
    fn ignores_an_isolated_opaque_pixel_below_the_support_surface() {
        let mut frames = Vec::new();
        for frame_index in 0..ANIMATION_PROBE_FRAME_COUNT {
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 72..184 {
                for y in 32..224 {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            for x in 96 + frame_index * 8..112 + frame_index * 8 {
                for y in 112..128 {
                    frame.put_pixel(x, y, Rgba([220, 80, 80, 255]));
                }
            }
            if frame_index == 0 {
                frame.put_pixel(128, 236, Rgba([20, 30, 40, 255]));
            }
            frames.push(frame);
        }

        let validation = validate_animated_frame_sequence(&frames, ANIMATION_PROBE_FRAME_COUNT)
            .expect("isolated pixel should not count as whole-scene drift");

        assert_eq!(validation.max_center_drift, 0);
        assert_eq!(validation.max_baseline_drift, 0);
    }

    #[test]
    fn rejects_a_probe_when_the_whole_scene_drifts_vertically() {
        let mut frames = Vec::new();
        for frame_index in 0..ANIMATION_PROBE_FRAME_COUNT {
            let offset = frame_index * 4;
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 72..184 {
                for y in 32 + offset..224 + offset {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            frames.push(frame);
        }

        let error = validate_animated_frame_sequence(&frames, ANIMATION_PROBE_FRAME_COUNT)
            .unwrap_err();

        assert!(error.contains("whole-scene drift"));
        assert!(error.contains("baseline 12px"));
    }

    #[test]
    fn rejects_a_probe_when_the_whole_scene_drifts_between_frames() {
        let mut frames = Vec::new();
        for frame_index in 0..ANIMATION_PROBE_FRAME_COUNT {
            let offset = frame_index * 12;
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 72 + offset..184 + offset {
                for y in 32..224 {
                    frame.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
            frames.push(frame);
        }

        let error = validate_animated_frame_sequence(&frames, ANIMATION_PROBE_FRAME_COUNT)
            .unwrap_err();

        assert!(error.contains("whole-scene drift"));
    }

    #[test]
    fn assembly_preserves_shared_scene_coordinates_when_character_bounds_change() {
        let mut frames = Vec::new();
        let desk = Rgba([20, 220, 40, 255]);
        let anchor = Rgba([80, 80, 220, 255]);

        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let mut frame = RgbaImage::new(API_FRAME_W, API_FRAME_H);
            for x in 100..156 {
                for y in 160..170 {
                    frame.put_pixel(x, y, desk);
                }
            }
            let character_start = if frame_index % 2 == 0 { 20 } else { 196 };
            for x in character_start..character_start + 40 {
                for y in 40..150 {
                    frame.put_pixel(x, y, Rgba([220, 80, 80, 255]));
                }
            }
            // Keep the ground anchor fixed so any output difference comes from
            // per-frame recentering, not from a legitimate whole-frame shift.
            for x in 124..132 {
                for y in 248..252 {
                    frame.put_pixel(x, y, anchor);
                }
            }
            frames.push(frame);
        }

        let row = assemble_animation_frames(&frames).unwrap();
        let desk_lefts = (0..DEFAULT_FRAME_COUNT)
            .map(|frame_index| {
                let start_x = frame_index * FRAME_W;
                (start_x..start_x + FRAME_W)
                    .find(|x| {
                        (0..FRAME_H).any(|y| {
                            let pixel = row.get_pixel(*x, y);
                            pixel[0] == desk[0] && pixel[1] == desk[1] && pixel[2] == desk[2]
                        })
                    })
                    .map(|x| x - start_x)
                    .expect("shared desk marker should remain visible")
            })
            .collect::<Vec<_>>();

        assert!(
            desk_lefts.windows(2).all(|pair| pair[0] == pair[1]),
            "shared desk marker drifted after frame assembly: {desk_lefts:?}"
        );
    }

    #[test]
    fn rejects_an_animation_sequence_with_the_wrong_number_of_frames() {
        let frames = vec![RgbaImage::from_pixel(
            API_FRAME_W,
            API_FRAME_H,
            Rgba([20, 30, 40, 255]),
        ); DEFAULT_FRAME_COUNT as usize - 1];

        let error = assemble_animation_frames(&frames).unwrap_err();

        assert!(error.contains("exactly 8 frames"));
    }

    #[test]
    fn rejects_eight_repeated_frames_as_a_static_animation() {
        let row = RgbaImage::from_pixel(
            FRAME_W * DEFAULT_FRAME_COUNT,
            FRAME_H,
            Rgba([20, 30, 40, 255]),
        );

        let error = validate_animated_sprite_row(&row).unwrap_err();

        assert!(error.contains("no visible motion"));
    }

    #[test]
    fn allows_large_pose_changes_when_every_frame_still_has_visible_motion() {
        let mut row = RgbaImage::new(FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H);
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let frame_start = frame_index * FRAME_W;
            let shape_start = if frame_index % 2 == 0 {
                frame_start
            } else {
                frame_start + FRAME_W * 2 / 3
            };
            let shape_end = shape_start + FRAME_W / 3;
            for x in shape_start..shape_end {
                for y in 0..FRAME_H {
                    row.put_pixel(x, y, Rgba([20, 40, 60, 255]));
                }
            }
        }

        validate_animated_sprite_row(&row).unwrap();
    }

    #[test]
    fn allows_texture_redraw_when_the_silhouette_stays_stable() {
        let mut row = RgbaImage::new(FRAME_W * DEFAULT_FRAME_COUNT, FRAME_H);
        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let color = Rgba([20 + frame_index as u8 * 20, 40, 60, 255]);
            let frame_start = frame_index * FRAME_W;
            for x in frame_start + FRAME_W / 4..frame_start + FRAME_W * 3 / 4 {
                for y in FRAME_H / 4..FRAME_H {
                    row.put_pixel(x, y, color);
                }
            }
        }

        validate_animated_sprite_row(&row).unwrap();
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
    fn row_normalization_keeps_shared_desk_x_coordinates_when_character_width_changes() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let width = API_FRAME_W * DEFAULT_FRAME_COUNT;
        let mut source = RgbaImage::from_pixel(
            width,
            API_FRAME_H,
            Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
        );
        let desk = Rgba([20, 220, 40, 255]);
        let character = Rgba([220, 80, 80, 255]);
        let anchor = Rgba([80, 80, 220, 255]);

        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let source_x = frame_index * API_FRAME_W;
            for x in (source_x + 80)..(source_x + 160) {
                for y in 190..203 {
                    source.put_pixel(x, y, desk);
                }
            }
            let character_start = match frame_index {
                0 => source_x + 20,
                1 => source_x + 180,
                _ => source_x + 105,
            };
            for x in character_start..(character_start + 30) {
                for y in 40..180 {
                    source.put_pixel(x, y, character);
                }
            }
        }
        source.put_pixel(0, 0, anchor);
        source.put_pixel(width - 1, 0, anchor);

        let normalized = normalize_horizontal_row(&png_bytes(&source), &key).unwrap();
        let offsets = (0..DEFAULT_FRAME_COUNT)
            .map(|frame_index| {
                let start_x = frame_index * FRAME_W;
                (start_x..start_x + FRAME_W)
                    .filter(|x| {
                        (0..FRAME_H).any(|y| {
                            let pixel = normalized.get_pixel(*x, y);
                            pixel[1] > 150 && pixel[0] < 100 && pixel[2] < 100
                        })
                    })
                    .map(|x| x - start_x)
                    .next()
                    .expect("desk marker should be present in every frame")
            })
            .collect::<Vec<_>>();

        assert!(
            offsets.windows(2).all(|pair| pair[0] == pair[1]),
            "shared desk marker drifted between frames: {offsets:?}"
        );
    }

    #[test]
    fn row_normalization_removes_whole_frame_horizontal_drift() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let mut source = RgbaImage::from_pixel(
            API_FRAME_W * DEFAULT_FRAME_COUNT,
            API_FRAME_H,
            Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]),
        );

        for frame_index in 0..DEFAULT_FRAME_COUNT {
            let start_x = frame_index * API_FRAME_W + 28 + frame_index * 14;
            for x in start_x..start_x + 96 {
                for y in 40..240 {
                    source.put_pixel(x, y, Rgba([20, 30, 40, 255]));
                }
            }
        }

        let normalized = normalize_horizontal_row(&png_bytes(&source), &key).unwrap();
        let centers = (0..DEFAULT_FRAME_COUNT)
            .map(|frame_index| {
                let start_x = frame_index * FRAME_W;
                let min_x = (start_x..start_x + FRAME_W)
                    .find(|x| (0..FRAME_H).any(|y| normalized.get_pixel(*x, y)[3] > 0))
                    .unwrap()
                    - start_x;
                let max_x = (start_x..start_x + FRAME_W)
                    .rev()
                    .find(|x| (0..FRAME_H).any(|y| normalized.get_pixel(*x, y)[3] > 0))
                    .unwrap()
                    - start_x;
                (min_x + max_x) / 2
            })
            .collect::<Vec<_>>();

        assert!(
            centers.windows(2).all(|pair| pair[0] == pair[1]),
            "whole-frame horizontal drift remained after normalization: {centers:?}"
        );
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
        // Off-key background with a small light character intruding into the
        // top-left corner.
        // Corner-mean would pull the sampled bg toward the blob; the median
        // survives because <50% of border pixels are the intruder.
        let key = ChromaKey {
            name: "green",
            hex: "#00FF00",
            rgb: [0, 255, 0],
        };
        let background = Rgba([12, 12, 12, 255]);
        let character = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(64, 64, background);
        for y in 0..3 {
            for x in 0..3 {
                image.put_pixel(x, y, character);
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
        assert_eq!(image.get_pixel(1, 1).0, character.0);
    }

    #[test]
    fn removes_a_diagonally_connected_sampled_background_pixel() {
        let sampled_background = Rgba([12, 12, 12, 255]);
        let diagonal_connector = Rgba([30, 30, 30, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(16, 16, sampled_background);

        let diagonal_chain = [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)];
        for &(x, y) in &diagonal_chain {
            image.put_pixel(x, y, diagonal_connector);
            if x > 0 {
                image.put_pixel(x - 1, y, foreground);
            }
            if x + 1 < image.width() {
                image.put_pixel(x + 1, y, foreground);
            }
            if y > 0 {
                image.put_pixel(x, y - 1, foreground);
            }
            if y + 1 < image.height() {
                image.put_pixel(x, y + 1, foreground);
            }
        }

        remove_chroma_background(&mut image, &CHROMA_KEY_CANDIDATES[0]);

        assert_eq!(image.get_pixel(5, 5).0, [0, 0, 0, 0]);
    }

    #[test]
    fn removes_a_small_enclosed_background_hole_like_a_hair_gap() {
        let background = Rgba([12, 12, 12, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(32, 32, background);

        for y in 8..24 {
            for x in 8..24 {
                image.put_pixel(x, y, foreground);
            }
        }
        for y in 12..20 {
            for x in 12..20 {
                image.put_pixel(x, y, background);
            }
        }

        remove_chroma_background(&mut image, &CHROMA_KEY_CANDIDATES[0]);

        assert_eq!(image.get_pixel(15, 15).0, [0, 0, 0, 0]);
    }

    #[test]
    fn preserves_an_oversized_enclosed_background_like_region() {
        let background = Rgba([12, 12, 12, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(120, 120, background);

        for y in 10..110 {
            for x in 10..110 {
                image.put_pixel(x, y, foreground);
            }
        }
        for y in 20..100 {
            for x in 20..100 {
                image.put_pixel(x, y, background);
            }
        }

        remove_chroma_background(&mut image, &CHROMA_KEY_CANDIDATES[0]);

        assert_eq!(image.get_pixel(60, 60).0, background.0);
    }

    #[test]
    fn removes_an_enclosed_configured_key_colored_hair_gap() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let background = Rgba([12, 12, 12, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(32, 32, background);

        for y in 8..24 {
            for x in 8..24 {
                image.put_pixel(x, y, foreground);
            }
        }
        image.put_pixel(15, 15, Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]));

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(15, 15).0, [0, 0, 0, 0]);
    }

    #[test]
    fn normalizes_rgb_for_every_fully_transparent_cleanup_pixel() {
        let background = Rgba([12, 12, 12, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(32, 32, background);

        for y in 8..24 {
            for x in 8..24 {
                image.put_pixel(x, y, foreground);
            }
        }
        image.put_pixel(0, 0, Rgba([7, 8, 9, 0]));
        image.put_pixel(15, 15, Rgba([40, 50, 60, 0]));

        remove_chroma_background(&mut image, &CHROMA_KEY_CANDIDATES[0]);

        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 0]);
        assert_eq!(image.get_pixel(15, 15).0, [0, 0, 0, 0]);
    }

    #[test]
    fn preserves_enclosed_key_colored_character_detail_but_removes_key_hair_gap() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let background = Rgba([12, 12, 12, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let key_pixel = Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]);
        let mut image = RgbaImage::from_pixel(40, 40, background);

        for y in 8..32 {
            for x in 8..32 {
                image.put_pixel(x, y, foreground);
            }
        }
        for y in 10..13 {
            for x in 10..15 {
                image.put_pixel(x, y, key_pixel);
            }
        }
        image.put_pixel(12, 11, Rgba([key.rgb[0], key.rgb[1], 250, 255]));
        image.put_pixel(24, 24, key_pixel);

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(12, 11).0, [key.rgb[0], key.rgb[1], 250, 255]);
        assert_eq!(image.get_pixel(10, 10).0, key_pixel.0);
        assert_eq!(image.get_pixel(24, 24).0, [0, 0, 0, 0]);
    }

    #[test]
    fn preserves_an_enclosed_near_background_highlight_but_removes_a_sampled_hair_gap() {
        let background = Rgba([200, 200, 200, 255]);
        let foreground = Rgba([20, 30, 40, 255]);
        let highlight = Rgba([205, 205, 205, 255]);
        let mut image = RgbaImage::from_pixel(40, 40, background);

        for y in 8..32 {
            for x in 8..32 {
                image.put_pixel(x, y, foreground);
            }
        }
        for y in 10..12 {
            for x in 10..12 {
                image.put_pixel(x, y, highlight);
            }
        }
        image.put_pixel(24, 24, background);

        remove_chroma_background(&mut image, &CHROMA_KEY_CANDIDATES[0]);

        assert_eq!(image.get_pixel(10, 10).0, highlight.0);
        assert_eq!(image.get_pixel(24, 24).0, [0, 0, 0, 0]);
    }

    #[test]
    fn preserves_an_oversized_hole_and_still_cleans_a_later_small_hair_gap() {
        let background = Rgba([12, 12, 12, 255]);
        let foreground = Rgba([220, 220, 220, 255]);
        let mut image = RgbaImage::from_pixel(180, 180, background);

        for y in 8..172 {
            for x in 8..172 {
                image.put_pixel(x, y, foreground);
            }
        }
        for y in 16..144 {
            for x in 16..144 {
                image.put_pixel(x, y, background);
            }
        }
        image.put_pixel(152, 152, background);

        remove_chroma_background(&mut image, &CHROMA_KEY_CANDIDATES[0]);

        assert_eq!(image.get_pixel(80, 80).0, background.0);
        assert_eq!(image.get_pixel(152, 152).0, [0, 0, 0, 0]);
    }

    #[test]
    fn removes_an_off_color_snowflake_decoration_from_a_blue_background() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let background = Rgba([24, 96, 220, 255]);
        let character = Rgba([220, 80, 80, 255]);
        let snowflake = Rgba([240, 248, 255, 255]);
        let mut image = RgbaImage::from_pixel(64, 64, background);

        for y in 16..52 {
            for x in 24..40 {
                image.put_pixel(x, y, character);
            }
        }
        for &(x, y) in &[
            (8, 8),
            (9, 8),
            (10, 8),
            (8, 9),
            (9, 9),
            (10, 9),
            (8, 10),
            (9, 10),
            (10, 10),
            (7, 9),
            (11, 9),
            (9, 7),
            (9, 11),
        ] {
            image.put_pixel(x, y, snowflake);
        }

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(32, 32).0, character.0);
        for &(x, y) in &[
            (8, 8),
            (9, 8),
            (10, 8),
            (8, 9),
            (9, 9),
            (10, 9),
            (8, 10),
            (9, 10),
            (10, 10),
            (7, 9),
            (11, 9),
            (9, 7),
            (9, 11),
        ] {
            assert_eq!(
                image.get_pixel(x, y).0,
                [0, 0, 0, 0],
                "background decoration pixel at ({x},{y}) should be removed"
            );
        }
    }

    #[test]
    fn removes_noisy_key_colored_background_beyond_the_rgb_threshold() {
        let key = CHROMA_KEY_CANDIDATES[0];
        let noisy_background = Rgba([140, 20, 135, 255]);
        let character = Rgba([40, 40, 40, 255]);
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([key.rgb[0], key.rgb[1], key.rgb[2], 255]));

        for y in 4..60 {
            for x in 4..60 {
                image.put_pixel(x, y, noisy_background);
            }
        }
        for y in 16..52 {
            for x in 24..40 {
                image.put_pixel(x, y, character);
            }
        }

        remove_chroma_background(&mut image, &key);

        assert_eq!(image.get_pixel(10, 10).0, [0, 0, 0, 0]);
        assert_eq!(image.get_pixel(32, 32).0, character.0);
    }

}
