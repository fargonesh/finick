use {
    crate::theme::{RADIUS_PILL, use_app_theme},
    freya::prelude::*,
};

#[derive(Clone, PartialEq)]
struct SegmentItem<T: Clone + PartialEq + 'static> {
    label_text: String,
    val: T,
    is_active: bool,
    flex: bool,
    on_select: EventHandler<T>,
}

impl<T: Clone + PartialEq + 'static> Component for SegmentItem<T> {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let is_hovered = *hovered.read();
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_select = self.on_select.clone();
        let val_clone = self.val.clone();

        let bg = if self.is_active {
            t.panel_raised
        } else if is_hovered {
            t.bg_hover
        } else {
            Color::TRANSPARENT
        };
        let text_color = if self.is_active { t.text } else { t.text_dim };

        rect()
            .maybe(self.flex, |el| el.width(Size::flex(1.)).content(Content::Flex))
            .padding((6., 13.))
            .corner_radius(RADIUS_PILL)
            .background(bg)
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_role(AccessibilityRole::RadioButton)
            .a11y_focusable(true)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_select.call(val_clone.clone()))
            .child(
                label()
                    .font_size(12.5)
                    .font_weight(FontWeight::MEDIUM)
                    .color(text_color)
                    .text(self.label_text.clone()),
            )
    }
}

fn segmented_inner<T: Clone + PartialEq + 'static>(
    items: Vec<(String, T)>,
    selected: T,
    on_select: EventHandler<T>,
    full: bool,
) -> Element {
    let t = use_app_theme();

    let mut outer = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(3.)
        .corner_radius(RADIUS_PILL)
        .background(t.track);
    if full {
        outer = outer.width(Size::fill()).content(Content::Flex);
    }
    outer
        .children(items.into_iter().map(move |(label_text, val)| {
            let is_active = val == selected;
            SegmentItem {
                label_text,
                val: val.clone(),
                is_active,
                flex: full,
                on_select: on_select.clone(),
            }
            .into_element()
        }))
        .into_element()
}

/// A segmented control matching .segmented in ui_demo.html.
/// Hug-content. For full-width rows use `segmented_control_full`.
pub fn segmented_control<T: Clone + PartialEq + 'static>(
    items: Vec<(&'static str, T)>,
    selected: T,
    on_select: impl Into<EventHandler<T>>,
) -> impl IntoElement {
    let on_select: EventHandler<T> = on_select.into();
    segmented_inner(
        items.into_iter().map(|(l, v)| (l.to_string(), v)).collect(),
        selected,
        on_select,
        false,
    )
}

/// Full-width segmented control with equal flex segments. The control itself fills, do not wrap it.
pub fn segmented_control_full<T: Clone + PartialEq + 'static>(
    items: Vec<(&'static str, T)>,
    selected: T,
    on_select: impl Into<EventHandler<T>>,
) -> impl IntoElement {
    let on_select: EventHandler<T> = on_select.into();
    segmented_inner(
        items.into_iter().map(|(l, v)| (l.to_string(), v)).collect(),
        selected,
        on_select,
        true,
    )
}

/// Segmented control variant accepting dynamic String labels. Hug-content.
pub fn segmented_control_dynamic<T: Clone + PartialEq + 'static>(
    items: Vec<(String, T)>,
    selected: T,
    on_select: impl Into<EventHandler<T>>,
) -> impl IntoElement {
    let on_select: EventHandler<T> = on_select.into();
    segmented_inner(items, selected, on_select, false)
}

/// Full-width dynamic segmented control with equal flex segments.
pub fn segmented_control_dynamic_full<T: Clone + PartialEq + 'static>(
    items: Vec<(String, T)>,
    selected: T,
    on_select: impl Into<EventHandler<T>>,
) -> impl IntoElement {
    let on_select: EventHandler<T> = on_select.into();
    segmented_inner(items, selected, on_select, true)
}
