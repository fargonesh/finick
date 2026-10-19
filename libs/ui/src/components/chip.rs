use {
    crate::theme::{RADIUS_PILL, use_app_theme},
    freya::prelude::*,
};

#[derive(Clone, PartialEq)]
struct StatusChipInner {
    text: String,
    is_on: bool,
    on_press: Option<EventHandler<()>>,
}

impl Component for StatusChipInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let is_hovered = *hovered.read();
        let mut h1 = hovered;
        let mut h2 = hovered;

        let bg = if self.is_on {
            if self.on_press.is_some() && is_hovered {
                t.bg_hover
            } else {
                t.bg_active
            }
        } else if self.on_press.is_some() && is_hovered {
            t.panel_raised
        } else {
            t.track
        };
        let text_color = if self.is_on { t.accent } else { t.text_dim };

        rect()
            .padding((3., 9.))
            .corner_radius(RADIUS_PILL)
            .background(bg)
            .maybe(self.on_press.is_some(), |el| {
                let cb = self.on_press.clone().unwrap();
                el.cursor(CursorIcon::Pointer)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_focusable(true)
                    .on_pointer_enter(move |_| h1.set(true))
                    .on_pointer_leave(move |_| h2.set(false))
                    .on_press(move |_| cb.call(()))
            })
            .child(
                label()
                    .font_size(11.)
                    .font_weight(FontWeight::MEDIUM)
                    .color(text_color)
                    .max_lines(Some(1))
                    .text(self.text.clone()),
            )
    }
}

/// A status chip matching .chip in ui_demo.html
pub fn status_chip(text: impl Into<String>, is_on: bool, on_press: Option<EventHandler<()>>) -> impl IntoElement {
    StatusChipInner {
        text: text.into(),
        is_on,
        on_press,
    }
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
