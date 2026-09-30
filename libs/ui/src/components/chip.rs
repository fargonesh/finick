use {
    crate::theme::{RADIUS_PILL, use_app_theme},
    freya::prelude::*,
};

/// A status chip matching .chip in ui_demo.html
pub fn status_chip(text: impl Into<String>, is_on: bool, on_press: Option<EventHandler<()>>) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();

    let bg = if is_on { t.bg_active } else { t.track };

    let text_color = if is_on { t.accent } else { t.text_dim };

    rect()
        .padding((3., 9.))
        .corner_radius(RADIUS_PILL)
        .background(bg)
        .map(on_press, |el, cb| {
            el.cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::Button)
                .a11y_focusable(true)
                .on_press(move |_| cb.call(()))
        })
        .child(label().font_size(11.).font_weight(FontWeight::MEDIUM).color(text_color).text(text_str))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoticeKind {
    Info,
    Success,
    Warn,
    Error,
}

pub fn notice_pill(text: impl Into<String>, kind: NoticeKind) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    let kind_color = match kind {
        NoticeKind::Info => t.accent,
        NoticeKind::Success => t.accent_green,
        NoticeKind::Warn => t.accent_orange,
        NoticeKind::Error => t.accent_red,
    };

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(6.)
        .padding((5., 10.))
        .corner_radius(RADIUS_PILL)
        .background(t.track)
        .child(rect().width(Size::px(6.)).height(Size::px(6.)).corner_radius(RADIUS_PILL).background(kind_color))
        .child(label().font_size(12.).font_weight(FontWeight::MEDIUM).color(kind_color).text(text_str))
}
