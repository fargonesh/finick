use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_PILL};

/// A battery ring indicator matching .ring in ui_demo.html
pub fn battery_ring(percentage: u8, is_big: bool) -> impl IntoElement {
    let t = use_app_theme();
    let outer_size = if is_big { 84. } else { 56. };
    let inner_size = if is_big { 66. } else { 44. };
    let font_size = if is_big { 16. } else { 12. };

    rect()
        .width(Size::px(outer_size))
        .height(Size::px(outer_size))
        .corner_radius(RADIUS_PILL)
        .background(t.track)
        .border(Border::new().width(4.).fill(t.accent))
        .center()
        .child(
            rect()
                .width(Size::px(inner_size))
                .height(Size::px(inner_size))
                .corner_radius(RADIUS_PILL)
                .background(t.ring_cut)
                .center()
                .child(
                    label()
                        .font_size(font_size)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(t.text)
                        .text(format!("{percentage}%")),
                ),
        )
}

/// Battery mode selector buttons matching .battery-modes in ui_demo.html
pub fn battery_modes<T: Clone + PartialEq + 'static>(
    modes: Vec<(&'static str, T)>,
    selected: T,
    is_row: bool,
    on_select: impl Into<EventHandler<T>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let on_select: EventHandler<T> = on_select.into();

    rect()
        .maybe(is_row, |el| el.horizontal().spacing(10.))
        .maybe(!is_row, |el| el.vertical().spacing(5.))
        .children(modes.into_iter().map(move |(name, val)| {
            let is_active = val == selected;
            let color = if is_active { t.text } else { t.text_dim };
            let font_weight = if is_active {
                FontWeight::SEMI_BOLD
            } else {
                FontWeight::NORMAL
            };
            let on_select = on_select.clone();
            let val_clone = val.clone();

            rect()
                .cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::Button)
                .a11y_focusable(true)
                .padding((2., 0.))
                .on_press(move |_| on_select.call(val_clone.clone()))
                .child(
                    label()
                        .font_size(11.5)
                        .font_weight(font_weight)
                        .color(color)
                        .text(name),
                )
        }))
}


