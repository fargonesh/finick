use freya::prelude::*;
use crate::theme;

/// A standard application top bar / navigation bar.
pub fn top_bar(
    leading: impl IntoElement,
    title_or_path: impl Into<String>,
) -> impl IntoElement {
    rect()
        .height(Size::px(60.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(12.)
        .background(theme::BG_SURFACE)
        .border(Border::new().width(1.).fill(theme::BORDER_SUBTLE))
        .child(leading)
        .child(
            label()
                .margin((0., 0., 0., 16.))
                .font_size(15.)
                .color(theme::TEXT_MUTED)
                .text(title_or_path.into()),
        )
}

/// A top bar with leading and trailing custom element slots.
pub fn top_bar_custom(
    leading: impl IntoElement,
    content: impl IntoElement,
    trailing: Option<impl IntoElement>,
) -> impl IntoElement {
    let mut bar = rect()
        .height(Size::px(60.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(12.)
        .background(theme::BG_SURFACE)
        .border(Border::new().width(1.).fill(theme::BORDER_SUBTLE))
        .child(leading)
        .child(
            rect()
                .expanded()
                .margin((0., 0., 0., 16.))
                .child(content),
        );

    if let Some(tr) = trailing {
        bar = bar.child(tr);
    }

    bar
}
