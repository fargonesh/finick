use freya::prelude::*;
use crate::{icons::{icon, LOCK}, theme::{use_app_theme, RADIUS_PILL}};

/// A subtle lock badge indicating a setting is restricted (e.g. by NixOS or policy).
pub fn lock_badge(text: impl Into<String>) -> impl IntoElement {
    let _t = use_app_theme();
    let label_text = text.into();
    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(4.)
        .padding((2., 7.))
        .corner_radius(RADIUS_PILL)
        .background(Color::from_af32rgb(0.15, 245, 158, 11))
        .border(Border::new().width(1.).fill(Color::from_af32rgb(0.35, 245, 158, 11)))
        .child(icon(LOCK, 10., Color::from_rgb(245, 158, 11)))
        .child(
            label()
                .font_size(10.5)
                .font_weight(FontWeight::SEMI_BOLD)
                .color(Color::from_rgb(245, 158, 11))
                .text(label_text),
        )
}

/// A settings row that supports displaying a lock badge and disabling/dimming its control.
pub fn setting_row_locked(
    name: impl Into<String>,
    meta: Option<impl Into<String>>,
    has_top_border: bool,
    lock_label: Option<impl Into<String>>,
    control: impl IntoElement,
) -> impl IntoElement {
    let t = use_app_theme();
    let name_str = name.into();
    let meta_str = meta.map(|m| m.into());
    let lock_str = lock_label.map(|l| l.into());
    let is_locked = lock_str.is_some();

    let title_row = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(8.)
        .child(
            label()
                .font_size(13.)
                .font_weight(FontWeight::MEDIUM)
                .color(if is_locked { t.text_dim } else { t.text })
                .text(name_str),
        )
        .maybe_child(lock_str.map(|s| lock_badge(s)));

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .maybe(has_top_border, |el| el.padding((9., 0., 9., 0.)))
        .maybe(!has_top_border, |el| el.padding((0., 0., 9., 0.)))
        .child(
            rect()
                .vertical()
                .child(title_row)
                .maybe_child(meta_str.map(|m| {
                    rect()
                        .margin((1., 0., 0., 0.))
                        .child(
                            label()
                                .font_size(12.)
                                .color(t.text_dim)
                                .text(m),
                        )
                })),
        )
        .child(
            rect()
                .opacity(if is_locked { 0.45 } else { 1.0 })
                .child(control),
        )
        .content(Content::Flex)
}

/// A settings row matching .row in ui_demo.html
pub fn setting_row(
    name: impl Into<String>,
    meta: Option<impl Into<String>>,
    has_top_border: bool,
    control: impl IntoElement,
) -> impl IntoElement {
    let t = use_app_theme();
    let name_str = name.into();
    let meta_str = meta.map(|m| m.into());

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        // .maybe(has_top_border, |el| el.border(Border::new().width(1.).fill(t.border)).padding((9., 0., 9., 0.)))
        .maybe(has_top_border, |el| el.padding((9., 0., 9., 0.)))
        .maybe(!has_top_border, |el| el.padding((0., 0., 9., 0.)))
        .child(
            rect()
                .vertical()
                .child(
                    label()
                        .font_size(13.)
                        .font_weight(FontWeight::MEDIUM)
                        .color(t.text)
                        .text(name_str),
                )
                .maybe_child(meta_str.map(|m| {
                    rect()
                        .margin((1., 0., 0., 0.))
                        .child(
                            label()
                                .font_size(12.)
                                .color(t.text_dim)
                                .text(m),
                        )
                })),
        )
        .child(control)
            .content(Content::Flex)
}

