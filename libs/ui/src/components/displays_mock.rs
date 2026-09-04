use freya::prelude::*;
use crate::icons::{CHEVRON_LEFT, CHEVRON_RIGHT, icon};
use crate::theme::{RADIUS_SM, use_app_theme};

#[derive(Clone, Debug)]
pub struct DisplayMockItem {
    pub name: String,
    pub height_px: f32,
}

/// Display monitor mock arrangement matching .displays-mock in ui_demo.html
pub fn displays_mock(displays: Vec<DisplayMockItem>) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::End)
        .spacing(14.)
        .margin((4., 0., 0., 0.))
        .content(Content::Flex)
        .children(displays.into_iter().map(move |disp| {
            rect()
                .width(Size::flex(1.))
                .vertical()
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(disp.height_px))
                        .corner_radius(RADIUS_SM)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .center()
                        .margin((6., 0., 0., 0.))
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_dim)
                                .text(disp.name),
                        ),
                )
        }))
}

fn arrow_button(
    svg: &'static str,
    enabled: bool,
    on_press: Option<EventHandler<()>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let icon_color = if enabled { t.text } else { t.text_disabled };
    let bg = t.panel_raised;
    let border_color = t.border;

    let base = rect()
        .width(Size::px(28.))
        .height(Size::px(22.))
        .corner_radius(6.)
        .background(bg)
        .border(Border::new().width(1.).fill(border_color))
        .center()
        .opacity(if enabled { 1.0 } else { 0.35 })
        .child(icon(svg, 12., icon_color));

    if let Some(cb) = on_press {
        base.cursor(CursorIcon::Pointer).on_press(move |_| cb.call(()))
    } else {
        base
    }
}

/// Reorderable variant with arrow controls per monitor.
/// `on_swap` is called with (from_idx, to_idx) to swap adjacent monitors.
/// Keeps original `displays_mock` API backward-compatible.
pub fn displays_mock_reorderable(
    displays: Vec<DisplayMockItem>,
    on_swap: impl Into<EventHandler<(usize, usize)>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let on_swap: EventHandler<(usize, usize)> = on_swap.into();

    if displays.is_empty() {
        return rect()
            .width(Size::fill())
            .center()
            .padding((18., 12.))
            .child(
                label()
                    .font_size(12.)
                    .color(t.text_dim)
                    .text("No displays detected"),
            )
            .into_element();
    }

    let total = displays.len();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::End)
        .spacing(14.)
        .margin((4., 0., 0., 0.))
        .content(Content::Flex)
        .children(displays.into_iter().enumerate().map(move |(idx, disp)| {
            let name_key = disp.name.clone();
            let name_label = disp.name.clone();
            let can_left = idx > 0;
            let can_right = idx + 1 < total;

            let left_handler: Option<EventHandler<()>> = if can_left {
                let cb = on_swap.clone();
                Some((move |_| cb.call((idx, idx - 1))).into())
            } else {
                None
            };
            let right_handler: Option<EventHandler<()>> = if can_right {
                let cb = on_swap.clone();
                Some((move |_| cb.call((idx, idx + 1))).into())
            } else {
                None
            };

            rect()
                .width(Size::flex(1.))
                .vertical()
                .spacing(6.)
                .key(name_key)
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .main_align(Alignment::Center)
                        .cross_align(Alignment::Center)
                        .spacing(6.)
                        .content(Content::Flex)
                        .child(arrow_button(CHEVRON_LEFT, can_left, left_handler))
                        .child(arrow_button(CHEVRON_RIGHT, can_right, right_handler)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(disp.height_px))
                        .corner_radius(RADIUS_SM)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .center()
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_dim)
                                .text(name_label),
                        ),
                )
        }))
        .into_element()
}
