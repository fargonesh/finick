use freya::prelude::*;
use crate::theme::use_app_theme;
use crate::icons::icon;

/// A slider row matching .slider-row in ui_demo.html
pub fn slider_row(
    icon_svg: Option<&'static str>,
    value: f64,
    on_change: impl FnMut(f64) + 'static,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .maybe_child(icon_svg.map(|svg| icon(svg, 15., t.text_dim)))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(
                    Slider::new(on_change).value(value),
                ),
        )
            .content(Content::Flex)

}
