use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_PILL};

/// A custom switch toggle matching .switch in ui_demo.html
pub fn pill_switch(checked: bool, mut on_toggle: impl FnMut(bool) + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let track_bg = if checked { t.accent } else { t.track };
    let thumb_left = if checked { 17.5 } else { 2.5 };

    rect()
        .width(Size::px(36.))
        .height(Size::px(21.))
        .corner_radius(RADIUS_PILL)
        .background(track_bg)
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_toggle(!checked))
        .child(
            rect()
                .width(Size::px(16.))
                .height(Size::px(16.))
                .corner_radius(RADIUS_PILL)
                .background(Color::from_rgb(255, 255, 255))
                .margin((2.5, 0., 0., thumb_left)),
        )
}
