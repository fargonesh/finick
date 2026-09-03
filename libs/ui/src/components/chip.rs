use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_PILL};

/// A status chip matching .chip in ui_demo.html
pub fn status_chip(
    text: impl Into<String>,
    is_on: bool,
    on_press: Option<EventHandler<()>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();

    let bg = if is_on {
        t.bg_active
    } else {
        t.track
    };

    let text_color = if is_on {
        t.accent
    } else {
        t.text_dim
    };

    rect()
        .padding((3., 9.))
        .corner_radius(RADIUS_PILL)
        .background(bg)
        .map(on_press, |el, cb| {
            el.cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::Button)
                .a11y_focusable(true)
                .on_press(move |_| cb.call(()))
        })
        .child(
            label()
                .font_size(11.)
                .font_weight(FontWeight::MEDIUM)
                .color(text_color)
                .text(text_str),
        )
}

