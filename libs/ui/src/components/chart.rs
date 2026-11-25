use freya::prelude::*;
use crate::theme::use_app_theme;

/// A mini bar chart matching .mini-chart in ui_demo.html
pub fn mini_bar_chart(
    values_pct: [f32; 7],
    labels: [&'static str; 7],
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(64.))
                .horizontal()
                .cross_align(Alignment::End)
                .spacing(8.)
                .margin((8., 0., 0., 0.))
                .content(Content::Flex)
                .children(values_pct.into_iter().map(move |pct| {
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::percent(pct))
                        .corner_radius((4., 4., 2., 2.))
                        .background(t.accent)
                })),
        )
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(8.)
                .margin((6., 0., 0., 0.))
                .content(Content::Flex)
                .children(labels.into_iter().map(move |lbl| {
                    rect()
                        .width(Size::flex(1.))
                        .center()
                        .child(
                            label()
                                .font_size(10.5)
                                .color(t.text_dim)
                                .text(lbl),
                        )
                })),
        )
}
