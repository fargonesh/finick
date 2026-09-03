use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_PILL};

#[derive(Clone, Debug)]
pub struct StorageLegendItem {
    pub label: &'static str,
    pub value: String,
    pub color: Color,
}

/// A multi-segment storage/usage bar matching .bar in ui_demo.html
pub fn multi_segment_bar(
    segments: Vec<(f32, Color)>,
    is_big: bool,
) -> impl IntoElement {
    let t = use_app_theme();
    let height = if is_big { 14. } else { 10. };
    let radius = if is_big { 7. } else { 5. };

    rect()
        .width(Size::fill())
        .height(Size::px(height))
        .corner_radius(radius)
        .background(t.track)
        .horizontal()
        .children(segments.into_iter().map(|(pct, color)| {
            rect()
                .width(Size::percent(pct))
                .height(Size::fill())
                .background(color)
        }))
}

/// A storage legend matching .legend in ui_demo.html
pub fn storage_legend(items: Vec<StorageLegendItem>) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(18.)
        .margin((14., 0., 0., 0.))
            .content(Content::Flex)
        .children(items.into_iter().map(move |item| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(7.)
                .child(
                    rect()
                        .width(Size::px(7.))
                        .height(Size::px(7.))
                        .corner_radius(RADIUS_PILL)
                        .background(item.color),
                )
                .child(
                    label()
                        .font_size(12.)
                        .color(t.text_dim)
                        .text(item.label),
                )
                .child(
                    label()
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(t.text)
                        .text(item.value),
                )
        }))
}
