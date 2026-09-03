use freya::{elements::{extensions::{AccessibilityExt, ChildrenExt, ContainerExt, ContainerWithContentExt, EventHandlersExt, StyleExt, TextStyleExt}, label::label, rect::rect}, prelude::{AccessibilityRole, Alignment, Color, EventHandler, FontWeight, IntoElement}, winit::window::CursorIcon};

use crate::{RADIUS_PILL, use_app_theme};

/// A segmented control matching .segmented in ui_demo.html
pub fn segmented_control<T: Clone + PartialEq + 'static>(
    items: Vec<(&'static str, T)>,
    selected: T,
    on_select: impl Into<EventHandler<T>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let on_select: EventHandler<T> = on_select.into();

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(3.)
        .corner_radius(RADIUS_PILL)
        .background(t.track)
        .children(items.into_iter().map(move |(label_text, val)| {
            let is_active = val == selected;
            let bg = if is_active {
                t.panel_raised
            } else {
                Color::TRANSPARENT
            };
            let text_color = if is_active {
                t.text
            } else {
                t.text_dim
            };
            let on_select = on_select.clone();
            let val_clone = val.clone();

            rect()
                .padding((6., 13.))
                .corner_radius(RADIUS_PILL)
                .background(bg)
                .cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::RadioButton)
                .a11y_focusable(true)
                .on_press(move |_| on_select.call(val_clone.clone()))
                .child(
                    label()
                        .font_size(12.5)
                        .font_weight(FontWeight::MEDIUM)
                        .color(text_color)
                        .text(label_text),
                )
        }))
}


