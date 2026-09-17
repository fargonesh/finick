use freya::prelude::*;
use crate::state::power_action;
use ui::*;

#[derive(PartialEq)]
pub struct SessionMenu;

impl Component for SessionMenu {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let close = move || Platform::get().close_window(Platform::window_id());
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(140, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(420.))
                    .padding(20.)
                    .corner_radius(20.)
                    .background(t.panel)
                    .border(Border::new().width(1.).fill(t.border))
                    .vertical()
                    .spacing(14.)
                    .content(Content::Flex)
                    .child(label().font_size(18.).font_weight(FontWeight::BOLD).color(t.text).text("Session"))
                    .child(label().font_size(12.).color(t.text_dim).text("Choose a power action"))
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .spacing(10.)
                            .content(Content::Flex)
                            .child(session_btn("Logout", ARROW_RIGHT, t.panel_raised, t.text, move || { power_action("logout"); close(); }))
                            .child(session_btn("Reboot", REFRESH_CW, t.panel_raised, t.text, move || { power_action("reboot"); close(); })),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .spacing(10.)
                            .content(Content::Flex)
                            .child(session_btn("Sleep", FOCUS, t.panel_raised, t.text, move || { power_action("sleep"); close(); }))
                            .child(session_btn("Shutdown", LOCK, t.accent_red, Color::WHITE, move || { power_action("shutdown"); close(); })),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .margin((10., 0., 0., 0.))
                            .child(ghost_button("Cancel", move || close())),
                    ),
            )
            .into_element()
    }
}

fn session_btn(label_text: &'static str, svg: &'static str, bg: Color, fg: Color, mut act: impl FnMut() + 'static) -> impl IntoElement {
    rect()
        .width(Size::flex(1.))
        .height(Size::px(72.))
        .corner_radius(12.)
        .background(bg)
        .border(Border::new().width(1.).fill(if bg == Color::WHITE { bg } else { Color::TRANSPARENT }))
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| act())
        .child(rect().vertical().center().spacing(6.).child(icon(svg, 18., fg)).child(label().font_size(12.).font_weight(FontWeight::SEMI_BOLD).color(fg).text(label_text.to_string())))
}

pub fn session_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    SessionMenu.into_element()
}

pub fn session_window_config() -> WindowConfig {
    WindowConfig::new(session_app)
        .with_title("session")
        .with_app_id("overlay-session")
        .with_size(480., 320.)
        .with_decorations(false)
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
}

pub fn open_session_menu() {
    let cfg = session_window_config();
    spawn(async move {
        Platform::get().launch_window(cfg).await;
    });
}
