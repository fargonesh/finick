use freya::prelude::*;
use crate::theme::{use_app_theme, Accent, ACCENTS, RADIUS_PILL};

pub fn hex_to_color(hex: &str) -> Option<Color> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() < 6 { return None; }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Color::from_rgb(r, g, b))
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let d = max - min;
    let h = if d == 0.0 { 0.0 } else if max == rf { 60.0 * ((gf - bf)/d % 6.0) } else if max == gf { 60.0 * ((bf - rf)/d + 2.0) } else { 60.0 * ((rf - gf)/d + 4.0) };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (rf, gf, bf) = match (h as i32 / 60) % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgb(((rf+m)*255.0) as u8, ((gf+m)*255.0) as u8, ((bf+m)*255.0) as u8)
}

pub fn material_colors_from_hex(hex: &str) -> Vec<(String, Color)> {
    let base = match hex_to_color(hex) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let (h, s, v) = rgb_to_hsv(base.r(), base.g(), base.b());
    let s = s.max(0.45).min(0.85);
    let v = v.max(0.75).min(0.95);
    let shifts = [30.0, -30.0, 60.0, 180.0, -60.0];
    shifts.into_iter().enumerate().map(|(i, sh)| {
        let nh = (h + sh + 360.0) % 360.0;
        let ns = (s * 0.9).clamp(0.4, 0.9);
        let nv = v;
        (format!("Material {}", i+1), hsv_to_rgb(nh, ns, nv))
    }).collect()
}

pub fn extract_wallpaper_colors(wp_path: &str) -> Vec<Accent> {
    let clean = wp_path.strip_prefix("file://").unwrap_or(wp_path);
    if clean.is_empty() || clean.starts_with('#') {
        return Vec::new();
    }
    // Attempt pywal if installed
    let _ = std::process::Command::new("wal")
        .args(["-i", clean, "-n", "-s", "-t", "-q"])
        .output();
    if let Ok(home) = std::env::var("HOME") {
        let wal_colors = format!("{home}/.cache/wal/colors");
        if let Ok(content) = std::fs::read_to_string(wal_colors) {
            let mut list = Vec::new();
            for (i, line) in content.lines().enumerate().take(4) {
                let hex = line.trim();
                if let Some(col) = hex_to_color(hex) {
                    list.push(Accent {
                        name: Box::leak(format!("Wallpaper {i}").into_boxed_str()),
                        hex: Box::leak(hex.to_string().into_boxed_str()),
                        color: col,
                    });
                }
            }
            if !list.is_empty() {
                return list;
            }
        }
    }
    // Fallback: extract distinct tones from image bytes
    if let Ok(bytes) = std::fs::read(clean) {
        if bytes.len() > 1024 {
            let mut list = Vec::new();
            let step = bytes.len() / 5;
            for i in 1..4 {
                let pos = i * step;
                let r = bytes[pos].clamp(70, 210);
                let g = bytes[pos + 1].clamp(70, 210);
                let b = bytes[pos + 2].clamp(70, 210);
                let hex = format!("#{r:02X}{g:02X}{b:02X}");
                list.push(Accent {
                    name: Box::leak(format!("Wallpaper {i}").into_boxed_str()),
                    hex: Box::leak(hex.into_boxed_str()),
                    color: Color::from_rgb(r, g, b),
                });
            }
            return list;
        }
    }
    Vec::new()
}

pub fn swatch_picker(
    selected_accent_name: &'static str,
    is_big: bool,
    on_select: impl Into<EventHandler<Accent>>,
) -> impl IntoElement {
    swatch_picker_merged(selected_accent_name, Vec::new(), is_big, on_select)
}

pub fn swatch_picker_merged(
    selected_accent_name: &'static str,
    extra_accents: Vec<Accent>,
    is_big: bool,
    on_select: impl Into<EventHandler<Accent>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let size = if is_big { 28. } else { 22. };
    let on_select: EventHandler<Accent> = on_select.into();

    let mut all_accents: Vec<Accent> = ACCENTS.into_iter().collect();
    for ex in extra_accents {
        if !all_accents.iter().any(|a| a.hex.eq_ignore_ascii_case(ex.hex)) {
            all_accents.push(ex);
        }
    }

    let on_select_plus = on_select.clone();

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .margin((2., 0., 0., 0.))
        .children(all_accents.into_iter().map(move |acc| {
            let is_selected = acc.name == selected_accent_name || acc.hex.eq_ignore_ascii_case(selected_accent_name);
            let on_select = on_select.clone();
            let border = if is_selected { Border::new().width(2.).fill(t.text) } else { Border::new().width(1.).fill(t.border) };
            rect()
                .width(Size::px(size))
                .height(Size::px(size))
                .corner_radius(RADIUS_PILL)
                .background(acc.color)
                .border(border)
                .margin((0., 9., 0., 0.))
                .cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::RadioButton)
                .a11y_focusable(true)
                .on_press(move |_| on_select.call(acc))
        }))
        .child({
            // '+' button at the end to initiate custom colour picker
            rect()
                .width(Size::px(size))
                .height(Size::px(size))
                .corner_radius(RADIUS_PILL)
                .background(t.panel_raised)
                .border(Border::new().width(1.).fill(t.border))
                .margin((0., 9., 0., 0.))
                .cursor(CursorIcon::Pointer)
                .center()
                .on_press(move |_| {
                    let custom_palette = ["#FF6B6B", "#4ECDC4", "#FFE66D", "#A8E6CF", "#DED2F9", "#FF8B94", "#70A1FF"];
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
                    let hex = custom_palette[(now as usize) % custom_palette.len()];
                    if let Some(c) = hex_to_color(hex) {
                        let acc = Accent {
                            name: Box::leak(format!("Custom {hex}").into_boxed_str()),
                            hex: Box::leak(hex.to_string().into_boxed_str()),
                            color: c,
                        };
                        on_select_plus.call(acc);
                    }
                })
                .child(label().font_size(13.).font_weight(FontWeight::BOLD).color(t.text).text("+"))
        })
}

pub fn accent_picker_with_material(
    selected_accent_name: &'static str,
    wallpaper_hex: String,
    is_big: bool,
    on_select: impl Into<EventHandler<Accent>>,
) -> impl IntoElement {
    let wallpaper_colors = extract_wallpaper_colors(&wallpaper_hex);
    swatch_picker_merged(selected_accent_name, wallpaper_colors, is_big, on_select)
}
