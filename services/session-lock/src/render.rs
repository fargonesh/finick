//! Lock-screen rendering: wallpaper/cover background, clock + date text,
//! fingerprint ring, password dots. Text needs a font (discovered at runtime
//! from system font dirs); without one the frame degrades to shapes only.

use {
    crate::FingerStatus,
    ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont, point},
    std::path::{Path, PathBuf},
};

/// Opaque fallback background (matches finick dark surface).
pub const LOCK_COLOR: u32 = 0xFF_23_23_2E;
/// Dots cap on narrow outputs; the buffer itself is uncapped.
const MAX_DOTS: usize = 16;
/// Wallpaper dim so clock/dots stay readable (×158/255).
const DIM_NUM: u32 = 158;
/// Skip absurdly large images instead of OOMing the locker.
const MAX_WP_PIXELS: u64 = 7680 * 4320;

pub enum Background {
    Solid(u32),
    Image(image::RgbaImage),
}

pub struct Theme {
    pub bg: Background,
    pub font: Option<FontArc>,
    pub use_24h: bool,
}

pub struct ClockText {
    pub time: String,
    pub date: String,
}

pub struct Overlays<'a> {
    pub dots: usize,
    pub dots_fail: bool,
    pub finger: FingerStatus,
    pub clock: ClockText,
    pub hint: Option<&'a str>,
    pub font: Option<FontArc>,
}

/// One-shot duration until the next minute boundary (for the clock timer).
pub fn seconds_to_next_minute() -> std::time::Duration {
    let s = chrono::Local::now().format("%S").to_string().parse::<u64>().unwrap_or(0);
    std::time::Duration::from_secs(60 - s.min(59))
}

pub fn clock_text(use_24h: bool) -> ClockText {
    let now = chrono::Local::now();
    let time = if use_24h {
        now.format("%H:%M").to_string()
    } else {
        now.format("%I:%M %p").to_string().trim_start_matches('0').to_string()
    };
    ClockText { time, date: now.format("%a %b %e").to_string() }
}

pub fn load_theme() -> Theme {
    let (wallpaper, use_24h) = match config::load_settings() {
        Ok(p) => ((*p.personalization.appearance.wallpaper).clone(), *p.system.date_time.time_format_24h),
        Err(e) => {
            eprintln!("finick-lock: settings unreadable ({e}), defaults");
            (String::new(), true)
        }
    };
    let wallpaper = std::env::var("FINICK_LOCK_WALLPAPER").unwrap_or(wallpaper);
    Theme { bg: resolve_wallpaper(wallpaper.trim()), font: discover_font(), use_24h }
}

fn resolve_wallpaper(value: &str) -> Background {
    if value.is_empty() {
        return Background::Solid(LOCK_COLOR);
    }
    if let Some(idx) = value.strip_prefix("preset:").and_then(|s| s.parse::<usize>().ok()) {
        let hex = system::WALLPAPER_COLOR_PRESETS.get(idx).map(|(_, c)| *c).unwrap_or("#1e1e2e");
        return Background::Solid(parse_hex_color(hex).unwrap_or(LOCK_COLOR));
    }
    if value.starts_with('#') {
        return Background::Solid(parse_hex_color(value).unwrap_or(LOCK_COLOR));
    }
    let path = Path::new(value);
    if !path.is_file() {
        return Background::Solid(LOCK_COLOR);
    }
    match image::open(path) {
        Ok(img) => {
            let img = img.to_rgba8();
            if img.width() as u64 * img.height() as u64 > MAX_WP_PIXELS {
                eprintln!("finick-lock: wallpaper too large, solid fallback");
                Background::Solid(LOCK_COLOR)
            } else {
                Background::Image(img)
            }
        }
        Err(e) => {
            eprintln!("finick-lock: wallpaper undecodable ({e}), solid fallback");
            Background::Solid(LOCK_COLOR)
        }
    }
}

pub fn parse_hex_color(s: &str) -> Option<u32> {
    fn byte(h: &str, i: usize, len: usize) -> Option<u8> {
        let part = if len == 1 { h[i..i + 1].repeat(2) } else { h[i..i + 2].to_string() };
        u8::from_str_radix(&part, 16).ok()
    }
    let h = s.trim().strip_prefix('#')?;
    let (r, g, b) = match h.len() {
        3 => (byte(h, 0, 1)?, byte(h, 1, 1)?, byte(h, 2, 1)?),
        6 => (byte(h, 0, 2)?, byte(h, 2, 2)?, byte(h, 4, 2)?),
        _ => return None,
    };
    Some(0xFF_00_00_00 | (r as u32) << 16 | (g as u32) << 8 | b as u32)
}

/// Cover-fit background at exact pixel size, dimmed for readability.
pub fn background_pixels(bg: &Background, w: i32, h: i32) -> Vec<u32> {
    match bg {
        Background::Solid(c) => vec![*c; (w * h) as usize],
        Background::Image(img) => {
            let fit = image::DynamicImage::ImageRgba8(img.clone())
                .resize_to_fill(w as u32, h as u32, image::imageops::FilterType::Triangle)
                .to_rgba8();
            fit.pixels()
                .map(|p| {
                    let dim = |c: u8| (c as u32 * DIM_NUM / 255) as u32;
                    0xFF_00_00_00 | dim(p[0]) << 16 | dim(p[1]) << 8 | dim(p[2])
                })
                .collect()
        }
    }
}

pub fn draw_overlays(px: &mut [u32], w: i32, h: i32, ov: &Overlays) {
    let cy = h / 2;
    if let Some(font) = ov.font.clone() {
        draw_centered(px, w, h, &font, 76.0, &ov.clock.time, cy - 170, 0xFF_FF_FF_FF);
        draw_centered(px, w, h, &font, 24.0, &ov.clock.date, cy - 105, 0xFF_B0_B0_B8);
    }
    let (rcx, rcy, rr) = (w / 2, cy - 10, 28);
    match ov.finger {
        FingerStatus::Unavailable => ring(px, w, h, rcx, rcy, rr, 4, 0xFF_55_55_5F),
        FingerStatus::Waiting => ring(px, w, h, rcx, rcy, rr, 4, 0xFF_89_B4_FA),
        FingerStatus::NoMatch => ring(px, w, h, rcx, rcy, rr, 4, 0xFF_F3_8B_A8),
        FingerStatus::Match => fill_circle(px, w, h, rcx, rcy, rr - 8, 0xFF_A6_E3_A1),
    }
    let n = ov.dots.min(MAX_DOTS);
    if n > 0 {
        let color = if ov.dots_fail { 0xFF_F3_8B_A8 } else { 0xFF_FF_FF_FF };
        let spacing = 26;
        let start_x = w / 2 - ((n as i32 - 1) * spacing) / 2;
        for i in 0..n as i32 {
            fill_circle(px, w, h, start_x + i * spacing, cy + 72, 8, color);
        }
    }
    if let (Some(font), Some(hint)) = (ov.font.clone(), ov.hint) {
        if !hint.is_empty() {
            draw_centered(px, w, h, &font, 20.0, hint, cy + 122, 0xFF_88_88_90);
        }
    }
}

fn draw_centered(px: &mut [u32], w: i32, h: i32, font: &FontArc, size: f32, text: &str, center_y: i32, color: u32) {
    if text.is_empty() {
        return;
    }
    let scaled = font.as_scaled(PxScale::from(size));
    let total: f32 = text.chars().map(|c| scaled.h_advance(scaled.glyph_id(c))).sum();
    let (r, g, b) = (((color >> 16) & 255) as u8, ((color >> 8) & 255) as u8, (color & 255) as u8);
    let baseline = center_y as f32 + size * 0.35;
    let mut x = w as f32 / 2.0 - total / 2.0;
    for c in text.chars() {
        let id = scaled.glyph_id(c);
        let glyph = Glyph { id, scale: PxScale::from(size), position: point(x, baseline) };
        if let Some(outlined) = font.outline_glyph(glyph) {
            outlined.draw(|gx, gy, cov| {
                let (px_, py) = (gx as i32, gy as i32);
                if px_ >= 0 && px_ < w && py >= 0 && py < h && cov > 0.0 {
                    let i = (py * w + px_) as usize;
                    px[i] = blend(px[i], r, g, b, cov.min(1.0));
                }
            });
        }
        x += scaled.h_advance(id);
    }
}

fn blend(dst: u32, r: u8, g: u8, b: u8, a: f32) -> u32 {
    let mix = |d: u32, s: u8| (d as f32 * (1.0 - a) + s as f32 * a) as u32;
    0xFF_00_00_00 | mix((dst >> 16) & 255, r) << 16 | mix((dst >> 8) & 255, g) << 8 | mix(dst & 255, b)
}

fn fill_circle(px: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
    for dy in -r..=r {
        let dx = ((r * r - dy * dy) as f64).sqrt() as i32;
        let y = cy + dy;
        if y < 0 || y >= h {
            continue;
        }
        for x in (cx - dx)..=(cx + dx) {
            if x >= 0 && x < w {
                px[(y * w + x) as usize] = color;
            }
        }
    }
}

fn ring(px: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, thickness: i32, color: u32) {
    let (outer, inner) = (r * r, (r - thickness) * (r - thickness));
    for dy in -r..=r {
        let y = cy + dy;
        if y < 0 || y >= h {
            continue;
        }
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 <= outer && d2 > inner {
                let x = cx + dx;
                if x >= 0 && x < w {
                    px[(y * w + x) as usize] = color;
                }
            }
        }
    }
}

/// First parseable regular-ish sans font found under the system font dirs.
pub fn discover_font() -> Option<FontArc> {
    let mut dirs = vec![
        PathBuf::from("/run/current-system/sw/share/fonts"),
        PathBuf::from("/usr/share/fonts"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
        dirs.push(PathBuf::from(&home).join(".fonts"));
    }
    let mut cands: Vec<(i32, PathBuf)> = Vec::new();
    fn walk(dir: &Path, depth: u8, out: &mut Vec<(i32, PathBuf)>) {
        if depth == 0 || out.len() >= 500 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth - 1, out);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf") {
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() < 30_000_000 {
                            out.push((score_font(&path), path));
                        }
                    }
                }
            }
            if out.len() >= 500 {
                return;
            }
        }
    }
    for dir in &dirs {
        walk(dir, 4, &mut cands);
    }
    cands.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in cands {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                eprintln!("finick-lock: font {}", path.display());
                return Some(font);
            }
        }
    }
    eprintln!("finick-lock: no font found, shapes only");
    None
}

fn score_font(path: &Path) -> i32 {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    let mut score = 0;
    if name.contains("sans") {
        score += 3;
    }
    if name.contains("noto") {
        score += 2;
    }
    if name.contains("regular") || name.contains("medium") || name.contains("book") {
        score += 2;
    }
    if name.contains("italic") || name.contains("oblique") {
        score -= 2;
    }
    if name.contains("bold") || name.contains("light") || name.contains("thin") {
        score -= 1;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlays(dots: usize, fail: bool, finger: FingerStatus) -> Overlays<'static> {
        Overlays { dots, dots_fail: fail, finger, clock: ClockText { time: "12:00".into(), date: "Tue".into() }, hint: None, font: None }
    }

    fn non_bg(px: &[u32]) -> usize {
        px.iter().filter(|p| **p != LOCK_COLOR).count()
    }

    #[test]
    fn test_frame_shapes() {
        let w = 400;
        let h = 300;
        let mut base = background_pixels(&Background::Solid(LOCK_COLOR), w, h);
        draw_overlays(&mut base, w, h, &overlays(0, false, FingerStatus::Unavailable));
        let ring_only = non_bg(&base);
        assert!(ring_only > 0);
        let mut dots = background_pixels(&Background::Solid(LOCK_COLOR), w, h);
        draw_overlays(&mut dots, w, h, &overlays(3, false, FingerStatus::Unavailable));
        assert!(non_bg(&dots) > ring_only);
        let mut fail = background_pixels(&Background::Solid(LOCK_COLOR), w, h);
        draw_overlays(&mut fail, w, h, &overlays(3, true, FingerStatus::Unavailable));
        assert_eq!(non_bg(&fail), non_bg(&dots));
        assert_ne!(fail, dots);
    }

    #[test]
    fn test_hex_colors() {
        assert_eq!(parse_hex_color("#1e1e2e"), Some(0xFF_1E_1E_2E));
        assert_eq!(parse_hex_color("#fff"), Some(0xFF_FF_FF_FF));
        assert_eq!(parse_hex_color("nope"), None);
        assert_eq!(parse_hex_color("#12345"), None);
    }

    #[test]
    fn test_discover_font_runs() {
        // Must not panic; finds a font wherever system fonts exist.
        let _ = discover_font();
    }

    #[test]
    fn test_cover_dim_size() {
        let img = image::RgbaImage::from_pixel(8, 4, image::Rgba([255, 0, 0, 255]));
        let px = background_pixels(&Background::Image(img), 16, 16);
        assert_eq!(px.len(), 256);
        // dimmed red, still opaque
        assert!(px.iter().all(|p| p & 0xFF_00_00_00 == 0xFF_00_00_00));
        assert!(px.iter().all(|p| (p & 0x00_FF_FF_FF) != 0));
    }
}
