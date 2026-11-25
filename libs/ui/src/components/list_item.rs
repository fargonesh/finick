use freya::prelude::*;
use crate::theme::use_app_theme;

/// A row item with an icon and label, commonly used in lists and file browsers.
pub fn list_item(
    icon: impl Into<String>,
    title: impl Into<String>,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let icon_str = icon.into();
    let title_str = title.into();

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(8.)
        .corner_radius(8.)
        .cross_align(Alignment::Center)
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(22.)
                .margin((0., 14., 0., 0.))
                .text(icon_str),
        )
        .child(
            label()
                .font_size(14.)
                .color(t.text_primary)
                .text(title_str),
        )
}

/// A row item with an icon, title, and secondary subtitle/metadata.
pub fn list_item_with_subtitle(
    icon: impl Into<String>,
    title: impl Into<String>,
    subtitle: impl Into<String>,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let icon_str = icon.into();
    let title_str = title.into();
    let subtitle_str = subtitle.into();

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(8.)
        .corner_radius(8.)
        .cross_align(Alignment::Center)
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(24.)
                .margin((0., 14., 0., 0.))
                .text(icon_str),
        )
        .child(
            rect()
                .vertical()
                .child(
                    label()
                        .font_size(14.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(t.text_primary)
                        .text(title_str),
                )
                .child(
                    label()
                        .font_size(12.)
                        .color(t.text_secondary)
                        .text(subtitle_str),
                ),
        )
}

