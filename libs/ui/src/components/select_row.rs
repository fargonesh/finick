use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_SM};
use crate::icons::{icon, CHEVRON_DOWN};

/// A select row matching .select-row in ui_demo.html
pub fn select_row(
    label_text: impl Into<String>,
    on_press: Option<EventHandler<()>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let label_str = label_text.into();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .padding((9., 11.))
        .background(t.panel_raised)
        .border(Border::new().width(1.).fill(t.border))
        .corner_radius(RADIUS_SM)
        .margin((10., 0., 0., 0.))
        .map(on_press, |el, cb| {
            el.cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::ComboBox)
                .a11y_focusable(true)
                .on_press(move |_| cb.call(()))
        })
        .child(
            label()
                .font_size(12.5)
                .color(t.text)
                .text(label_str),
        )
        .child(icon(CHEVRON_DOWN, 13., t.text_dim))
}

