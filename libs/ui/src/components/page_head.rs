use freya::prelude::*;
use crate::theme::use_app_theme;
use crate::icons::icon;

/// A page header matching .page-head in ui_demo.html
pub fn page_head(
    icon_svg: &'static str,
    title: impl Into<String>,
    description: impl Into<String>,
) -> impl IntoElement {
    let t = use_app_theme();
    let title_str = title.into();
    let desc_str = description.into();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Start)
        .spacing(13.)
        .margin((0., 0., 22., 0.))
        .child(
            rect()
                .margin((2., 0., 0., 0.))
                .child(icon(icon_svg, 24., t.accent)),
        )
        .child(
            rect()
                .vertical()
                .child(
                    label()
                        .font_size(21.)
                        .font_weight(FontWeight::BOLD)
                        .color(t.text)
                        .margin((0., 0., 3., 0.))
                        .text(title_str),
                )
                .child(
                    label()
                        .font_size(13.)
                        .color(t.text_dim)
                        .text(desc_str),
                ),
        )
}
