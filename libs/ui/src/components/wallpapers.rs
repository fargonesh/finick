use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_SM};

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


