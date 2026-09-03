use crate::theme::use_app_theme;
use freya::prelude::*;

/// A primary accent-styled action button.
pub fn primary_button(text: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    rect()
        .padding((10., 16.))
        .corner_radius(8.)
        .background(t.primary_accent)
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(text_str))
}

/// A secondary outlined action button with surface background.
pub fn secondary_button(text: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    rect()
        .padding((10., 16.))
        .corner_radius(8.)
        .background(t.bg_card)
        .border(Border::new().width(1.).fill(t.border_card))
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(label().font_size(14.).color(t.text_primary).text(text_str))
}

/// A destructive/danger action button with red background.
pub fn danger_button(text: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    rect()
        .padding((10., 16.))
        .corner_radius(8.)
        .background(t.accent_red)
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(text_str))
}

/// An icon button with subtle rounded background.
pub fn icon_button(icon: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let icon_str = icon.into();
    rect()
        .width(Size::px(36.))
        .height(Size::px(36.))
        .corner_radius(8.)
        .background(t.bg_card)
        .border(Border::new().width(1.).fill(t.border_card))
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(label().font_size(16.).color(t.text_primary).text(icon_str))
}

/// A ghost button matching .ghost-btn in ui_demo.html (pill shape, panel_raised, border)
pub fn ghost_button(text: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    rect()
        .padding((6., 12.))
        .corner_radius(999.)
        .background(t.panel_raised)
        .border(Border::new().width(1.).fill(t.border))
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(12.)
                .font_weight(FontWeight::MEDIUM)
                .color(t.text)
                .text(text_str),
        )
}

/// A focus pill button with indicator dot matching .focus-pill in ui_demo.html
pub fn focus_pill(
    label_text: impl Into<String>,
    is_active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let label_str = label_text.into();
    let border_color = if is_active { t.accent } else { t.border };
    let text_color = if is_active { t.accent } else { t.text_dim };
    let dot_color = if is_active { t.accent } else { t.text_dim };

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .padding((7., 12.))
        .corner_radius(999.)
        .background(t.panel_raised)
        .border(Border::new().width(1.).fill(border_color))
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(
            rect()
                .width(Size::px(6.))
                .height(Size::px(6.))
                .corner_radius(999.)
                .background(dot_color)
                .margin((0., 6., 0., 0.)),
        )
        .child(
            label()
                .font_size(12.5)
                .font_weight(FontWeight::MEDIUM)
                .color(text_color)
                .text(label_str),
        )
}

