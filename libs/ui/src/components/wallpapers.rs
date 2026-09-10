use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_SM};

pub const WALLPAPER_COLOR_DEFAULTS: &[(&str, &str, Color)] = &[
    ("Indigo Haze", "#5B5FE9", Color::from_rgb(0x5B, 0x5F, 0xE9)),
    ("Coral Glow", "#FF7A66", Color::from_rgb(0xFF, 0x7A, 0x66)),
    ("Amber Light", "#E8B86A", Color::from_rgb(0xE8, 0xB8, 0x6A)),
    ("Teal Mist", "#4ECDC4", Color::from_rgb(0x4E, 0xCD, 0xC4)),
    ("Rose Bloom", "#F07AAE", Color::from_rgb(0xF0, 0x7A, 0xAE)),
    ("Sky Blue", "#7C8CF0", Color::from_rgb(0x7C, 0x8C, 0xF0)),
    ("Sage Green", "#8ED0A8", Color::from_rgb(0x8E, 0xD0, 0xA8)),
    ("Sand Warm", "#E6C79A", Color::from_rgb(0xE6, 0xC7, 0x9A)),
];

/// Wallpaper color swatch picker displaying curated default solid colors
pub fn wallpaper_color_picker(
    selected_val: &str,
    on_select: impl Into<EventHandler<String>>,
) -> Rect {
    let t = use_app_theme();
    let on_select: EventHandler<String> = on_select.into();

    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(8.)
        .content(Content::Flex)
        .children(WALLPAPER_COLOR_DEFAULTS.iter().enumerate().map(move |(idx, (_name, hex, color))| {
            let is_selected = selected_val == *hex || selected_val == format!("preset:{idx}");
            let border_color = if is_selected { t.accent } else { t.border };
            let border_width = if is_selected { 2. } else { 1. };
            let hex_val = hex.to_string();
            let on_select = on_select.clone();

            rect()
                .width(Size::flex(1.))
                .height(Size::px(44.))
                .corner_radius(RADIUS_SM)
                .background(*color)
                .border(Border::new().width(border_width).fill(border_color))
                .cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::RadioButton)
                .a11y_focusable(true)
                .on_press(move |_| on_select.call(hex_val.clone()))
        }))
}

/// Wallpaper thumbnail picker matching .wallpapers in ui_demo.html
pub fn wallpaper_picker(
    count: usize,
    selected_idx: usize,
    is_big: bool,
    on_select: impl Into<EventHandler<usize>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let height = if is_big { 56. } else { 34. };
    let on_select: EventHandler<usize> = on_select.into();

    // Gradients for wallpaper previews
    let gradients = [
        (Color::from_rgb(0x2B, 0x2C, 0x31), Color::from_rgb(0x3A, 0x3C, 0x48)),
        (Color::from_rgb(0x1F, 0x2A, 0x44), Color::from_rgb(0x3B, 0x4A, 0x6B)),
        (Color::from_rgb(0x3A, 0x2E, 0x47), Color::from_rgb(0x5E, 0x43, 0x6E)),
        (Color::from_rgb(0x26, 0x38, 0x36), Color::from_rgb(0x38, 0x58, 0x54)),
        (Color::from_rgb(0x40, 0x2E, 0x2E), Color::from_rgb(0x66, 0x43, 0x43)),
        (Color::from_rgb(0x3D, 0x38, 0x2C), Color::from_rgb(0x5C, 0x53, 0x3B)),
        (Color::from_rgb(0x28, 0x2B, 0x35), Color::from_rgb(0x42, 0x48, 0x5A)),
        (Color::from_rgb(0x33, 0x28, 0x38), Color::from_rgb(0x57, 0x3D, 0x61)),
    ];

    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(8.)
        .content(Content::Flex)
        .children((0..count).map(move |i| {
            let is_selected = i == selected_idx;
            let border_color = if is_selected { t.accent } else { t.border };
            let border_width = if is_selected { 2. } else { 1. };
            let (bg_color, _) = gradients[i % gradients.len()];
            let on_select = on_select.clone();

            rect()
                .width(Size::flex(1.))
                .height(Size::px(height))
                .corner_radius(RADIUS_SM)
                .background(bg_color)
                .border(Border::new().width(border_width).fill(border_color))
                .cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::RadioButton)
                .a11y_focusable(true)
                .on_press(move |_| on_select.call(i))
        }))
}


