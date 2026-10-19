use freya::prelude::*;
use crate::theme::use_app_theme;

/// A standard application top bar / navigation bar.
pub fn top_bar(
    leading: impl IntoElement,
    title_or_path: impl Into<String>,
) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .height(Size::px(60.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(12.)
        .background(t.panel)
        .border(Border::new().width(1.).fill(t.border))
        .child(leading)
        .child(
            label()
                .margin((0., 0., 0., 16.))
                .font_size(15.)
                .color(t.text_dim)
                .text(title_or_path.into()),
        )
}

/// A top bar with leading and trailing custom element slots.
pub fn top_bar_custom(
    leading: impl IntoElement,
    content: impl IntoElement,
    trailing: Option<impl IntoElement>,
) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .height(Size::px(60.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(12.)
        .background(t.panel)
        .border(Border::new().width(1.).fill(t.border))
        .child(leading)
        .child(
            rect()
                .expanded()
                .margin((0., 0., 0., 16.))
                .child(content),
        )
        .maybe_child(trailing)
}


