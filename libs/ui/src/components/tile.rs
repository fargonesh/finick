use freya::prelude::*;
use crate::theme;

/// A toggleable quick action button (e.g. Wi-Fi, Bluetooth, DND).
pub fn quick_action_btn(
    title: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let bg = if active {
        theme::ACCENT_BLUE
    } else {
        theme::BG_ACTIVE
    };

    rect()
        .width(Size::px(100.))
        .height(Size::px(80.))
        .corner_radius(12.)
        .background(bg)
        .center()
        .on_press(move |_| on_press())
        .child(
            label()
                .font_weight(FontWeight::BOLD)
                .color(theme::TEXT_PRIMARY)
                .text(title.into()),
        )
}

/// A quick action tile with an icon, title, and active state.
pub fn quick_action_tile(
    icon: impl Into<String>,
    title: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let bg = if active {
        theme::ACCENT_BLUE
    } else {
        theme::BG_CARD
    };
    let border_color = if active {
        theme::ACCENT_BLUE
    } else {
        theme::BORDER_CARD
    };

    rect()
        .width(Size::px(120.))
        .height(Size::px(90.))
        .corner_radius(12.)
        .background(bg)
        .border(Border::new().width(1.).fill(border_color))
        .center()
        .on_press(move |_| on_press())
        .child(
            rect()
                .vertical()
                .center()
                .child(
                    label()
                        .font_size(24.)
                        .margin((0., 0., 6., 0.))
                        .text(icon.into()),
                )
                .child(
                    label()
                        .font_size(13.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(theme::TEXT_PRIMARY)
                        .text(title.into()),
                ),
        )
}
