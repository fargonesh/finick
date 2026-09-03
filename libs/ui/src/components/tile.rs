use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_LG, RADIUS_MD};
use crate::icons::icon;

/// A tile container matching .tile in ui_demo.html
pub fn tile() -> Rect {
    let t = use_app_theme();
    rect()
        .width(Size::fill())
        .background(t.panel)
        .border(Border::new().width(1.).fill(t.border))
        .corner_radius(RADIUS_LG)
        .padding(18.)
        .vertical()
}


/// A tile header matching .tile-head in ui_demo.html
pub fn tile_head(
    icon_svg: Option<&'static str>,
    title: impl Into<String>,
    right_action: Option<impl IntoElement>,
) -> impl IntoElement {
    let t = use_app_theme();
    let title_str = title.into();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .margin((0., 0., 14., 0.))
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .maybe_child(icon_svg.map(|svg| {
                    rect().margin((0., 9., 0., 0.)).child(icon(svg, 17., t.text_dim))
                }))
                .child(
                    label()
                        .font_size(14.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(t.text)
                        .text(title_str),
                ),
        )
        .maybe_child(right_action)
}


/// A field label matching .field-label in ui_demo.html
pub fn field_label(text: impl Into<String>) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .margin((0., 0., 8., 0.))
        .child(
            label()
                .font_size(12.)
                .color(t.text_dim)
                .text(text.into()),
        )
}

/// A tile subtitle matching .tile-sub in ui_demo.html
pub fn tile_sub(text: impl Into<String>) -> impl IntoElement {
    let t = use_app_theme();
    label()
        .font_size(12.)
        .color(t.text_dim)
        .text(text.into())
}

/// A toggleable quick action button (e.g. Wi-Fi, Bluetooth, DND).
pub fn quick_action_btn(
    title: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let bg = if active {
        t.accent
    } else {
        t.panel_raised
    };

    rect()
        .width(Size::px(100.))
        .height(Size::px(80.))
        .corner_radius(RADIUS_MD)
        .background(bg)
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(
            label()
                .font_weight(FontWeight::BOLD)
                .color(t.text)
                .text(title.into()),
        )
}

/// A quick action tile with an icon, title, and active state.
pub fn quick_action_tile(
    icon_str: impl Into<String>,
    title: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let bg = if active {
        t.accent
    } else {
        t.panel
    };
    let border_color = if active {
        t.accent
    } else {
        t.border
    };

    rect()
        .width(Size::px(120.))
        .height(Size::px(90.))
        .corner_radius(RADIUS_MD)
        .background(bg)
        .border(Border::new().width(1.).fill(border_color))
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(
            rect()
                .vertical()
                .center()
                .child(
                    label()
                        .font_size(24.)
                        .margin((0., 0., 6., 0.))
                        .text(icon_str.into()),
                )
                .child(
                    label()
                        .font_size(13.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(t.text)
                        .text(title.into()),
                ),
        )
}

