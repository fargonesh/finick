use freya::prelude::*;
pub use freya::prelude::{ContextMenu, ContextMenuViewer, Menu, MenuButton};

pub fn ctx_divider() -> impl IntoElement {
    let t = crate::theme::use_app_theme();
    rect().width(Size::fill()).height(Size::px(1.)).background(t.border).margin((6., 0.))
}

pub fn ctx_button(label: impl Into<String>, mut on_press: impl FnMut() + 'static) -> MenuButton {
    let txt = label.into();
    MenuButton::new().on_press(move |_| on_press()).child(txt)
}

pub fn open_ctx(menu: Menu) {
    ContextMenu::open_from_down(menu);
}
