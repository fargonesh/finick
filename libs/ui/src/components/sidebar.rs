use freya::prelude::*;
use crate::theme;

/// A navigation item for sidebars.
pub fn sidebar_item(
    name: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let bg = if active {
        theme::BG_SELECTED
    } else {
        Color::TRANSPARENT
    };
    let text_color = if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };

    rect()
        .width(Size::fill())
        .padding(8.)
        .corner_radius(6.)
        .background(bg)
        .margin((0., 0., 4., 0.))
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(14.)
                .color(text_color)
                .text(name.into()),
        )
}

/// A navigation item for sidebars with an icon or emoji prefix.
pub fn sidebar_icon_item(
    icon: impl Into<String>,
    name: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let bg = if active {
        theme::BG_SELECTED
    } else {
        Color::TRANSPARENT
    };
    let text_color = if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .width(Size::fill())
        .padding(8.)
        .corner_radius(6.)
        .background(bg)
        .margin((0., 0., 4., 0.))
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(16.)
                .margin((0., 8., 0., 0.))
                .text(icon.into()),
        )
        .child(
            label()
                .font_size(14.)
                .color(text_color)
                .text(name.into()),
        )
}

/// A standard sidebar header title.
pub fn sidebar_header(title: impl Into<String>) -> impl IntoElement {
    rect()
        .margin((0., 0., 16., 0.))
        .child(
            label()
                .font_size(22.)
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT_PRIMARY)
                .text(title.into()),
        )
}

/// A styled sidebar container.
pub fn sidebar_container(
    width_px: f32,
    children: impl IntoIterator<Item = impl IntoElement>,
) -> impl IntoElement {
    rect()
        .width(Size::px(width_px))
        .height(Size::fill())
        .background(theme::BG_SIDEBAR)
        .padding(16.)
        .children(children)
}
