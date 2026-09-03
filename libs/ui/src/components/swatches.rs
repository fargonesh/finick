use freya::prelude::*;
use crate::theme::{use_app_theme, Accent, ACCENTS, RADIUS_PILL};

/// Swatch picker matching .swatches in ui_demo.html
pub fn swatch_picker(
    selected_accent_name: &'static str,
    is_big: bool,
    on_select: impl Into<EventHandler<Accent>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let size = if is_big { 28. } else { 22. };
    let on_select: EventHandler<Accent> = on_select.into();

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .margin((2., 0., 0., 0.))
        .children(ACCENTS.into_iter().map(move |acc| {
            let is_selected = acc.name == selected_accent_name;
            let on_select = on_select.clone();
            let border = if is_selected {
                Border::new().width(2.).fill(t.text)
            } else {
                Border::new().width(1.).fill(t.border)
            };

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
}


