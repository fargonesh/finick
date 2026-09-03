use freya::prelude::*;
use crate::theme::use_app_theme;

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

