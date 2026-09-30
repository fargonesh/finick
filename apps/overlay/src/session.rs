use {crate::state::power_action, freya::prelude::*, ui::*};

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
                FadeSlideIn::new().child(
                    dialog_card(420.)
                        .spacing(14.)
                        .child(label().font_size(18.).font_weight(FontWeight::BOLD).color(t.text).text("Session"))
                        .child(label().font_size(12.).color(t.text_dim).text("Choose a power action"))
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .spacing(10.)
                                .content(Content::Flex)
                                .child(action_tile_flex("⎋", "Logout", false, move || {
                                    power_action("logout");
                                    close();
                                }))
                                .child(action_tile_flex("↻", "Reboot", false, move || {
                                    power_action("reboot");
                                    close();
                                })),
                        )
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .spacing(10.)
                                .content(Content::Flex)
                                .child(action_tile_flex("☾", "Sleep", false, move || {
                                    power_action("sleep");
                                    close();
                                }))
                                .child(action_tile_flex("⏻", "Shutdown", true, move || {
                                    power_action("shutdown");
                                    close();
                                })),
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
                ),
            )
            .into_element()
    }
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
