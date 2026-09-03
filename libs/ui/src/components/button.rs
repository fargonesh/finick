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
        .on_pointer_enter(|_| {
            Cursor::set(CursorIcon::Pointer);
        })
        .on_pointer_leave(|_| {
            Cursor::set(CursorIcon::default());
        })
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
        .on_pointer_enter(|_| {
            Cursor::set(CursorIcon::Pointer);
        })
        .on_pointer_leave(|_| {
            Cursor::set(CursorIcon::default());
        })
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
        .on_pointer_enter(|_| {
            Cursor::set(CursorIcon::Pointer);
        })
        .on_pointer_leave(|_| {
            Cursor::set(CursorIcon::default());
        })
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
        .on_pointer_enter(|_| {
            Cursor::set(CursorIcon::Pointer);
        })
        .on_pointer_leave(|_| {
            Cursor::set(CursorIcon::default());
        })
        .on_press(move |_| on_press())
        .child(label().font_size(16.).color(t.text_primary).text(icon_str))
}
