#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use {freya::prelude::*, ui::*};

fn locker_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let mut password = use_state(String::new);
    let mut error = use_state(|| Option::<String>::None);
    let attempts = use_state(|| 0u32);
    let time_str = use_state(|| chrono::Local::now().format("%H:%M").to_string());
    let date_str = use_state(|| chrono::Local::now().format("%A, %B %d").to_string());
    let username = system::current_username();

    use_hook(move || {
        let mut ts = time_str;
        let mut ds = date_str;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                ts.set(chrono::Local::now().format("%H:%M").to_string());
                ds.set(chrono::Local::now().format("%A, %B %d").to_string());
            }
        });
    });

    let do_unlock = {
        let pwd = password;
        let mut err = error;
        let mut att = attempts;
        let user = username.clone();
        move |_| {
            let p = pwd.read().clone();
            if p.is_empty() {
                err.set(Some("Enter password".to_string()));
                return;
            }
            let user_c = user.clone();
            let mut err_c = err;
            let mut att_c = att;
            let mut pwd_c = pwd;
            spawn(async move {
                let p2 = p.clone();
                let user2 = user_c.clone();
                let ok = tokio::task::spawn_blocking(move || system::verify_password(&user2, &p2)).await.unwrap_or(false);
                if ok {
                    std::process::exit(0);
                } else {
                    err_c.set(Some("Incorrect password".to_string()));
                    att_c.set(*att_c.read() + 1);
                    pwd_c.set(String::new());
                }
            });
        }
    };

    let pwd_writable = password.into_writable();
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(Color::from_rgb(18, 18, 24))
        .center()
        .content(Content::Flex)
        .on_global_key_down({
            let mut handler = do_unlock.clone();
            move |e: Event<KeyboardEventData>| {
                if e.data().key == Key::Named(NamedKey::Enter) {
                    handler(());
                } else if e.data().key == Key::Named(NamedKey::Escape) {
                    password.set(String::new());
                    error.set(None);
                }
            }
        })
        .child(
            rect()
                .width(Size::px(420.))
                .padding(24.)
                .corner_radius(20.)
                .background(t.panel)
                .border(Border::new().width(1.).fill(t.border))
                .spacing(16.)
                .child(
                    rect()
                        .width(Size::fill())
                        .vertical()
                        .cross_align(Alignment::Center)
                        .spacing(6.)
                        .child(
                            label().font_size(48.).font_weight(FontWeight::BOLD).color(t.text).text(time_str.read().clone()),
                        )
                        .child(label().font_size(14.).color(t.text_dim).text(date_str.read().clone())),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .vertical()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(
                            rect()
                                .width(Size::px(72.))
                                .height(Size::px(72.))
                                .corner_radius(999.)
                                .background(t.panel_raised)
                                .border(Border::new().width(1.).fill(t.border))
                                .center()
                                .child(icon(GENERAL, 32., t.text_dim)),
                        )
                        .child(
                            label().font_size(16.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(username.clone()),
                        )
                        .child(label().font_size(12.).color(t.text_dim).text(format!("{} attempts", *attempts.read()))),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .vertical()
                        .spacing(10.)
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(44.))
                                .corner_radius(12.)
                                .background(t.bg)
                                .border(Border::new().width(1.).fill(t.border))
                                .padding((0., 12.))
                                .cross_align(Alignment::Center)
                                .content(Content::Flex)
                                .horizontal()
                                .spacing(10.)
                                .child(icon(LOCK, 16., t.text_dim))
                                .child(Input::new(pwd_writable).width(Size::fill()).placeholder("Password").on_submit({
                                    let mut h = do_unlock.clone();
                                    move |_| h(())
                                })),
                        )
                        .maybe(error.read().is_some(), |el| {
                            let msg = error.read().clone().unwrap_or_default();
                            el.child(
                                rect()
                                    .width(Size::fill())
                                    .padding((6., 10.))
                                    .corner_radius(10.)
                                    .background(Color::from_rgb(60, 20, 20))
                                    .border(Border::new().width(1.).fill(Color::from_rgb(120, 40, 40)))
                                    .child(label().font_size(12.).color(Color::from_rgb(255, 120, 120)).text(msg)),
                            )
                        })
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(44.))
                                .corner_radius(12.)
                                .background(t.accent)
                                .center()
                                .cursor(CursorIcon::Pointer)
                                .on_press({
                                    let mut h = do_unlock.clone();
                                    move |_: Event<PressEventData>| h(())
                                })
                                .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.panel).text("Unlock")),
                        ),
                )
                .child(
                    rect().width(Size::fill()).center().child(
                        label().font_size(11.).color(t.text_muted).text("Super+L  •  Lid close locks  •  Esc clears"),
                    ),
                ),
        )
        .into_element()
}

fn main() {
    let username = system::current_username();
    let _ = std::thread::spawn(|| watch_lid_and_lock());
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(locker_app)
                .with_title("locker")
                .with_app_id("locker")
                .with_size(1920., 1080.)
                .with_decorations(false)
                .with_transparency(false)
                .with_background(Color::from_rgb(12, 12, 16))
                .with_window_attributes(|a, _el| a.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))),
        ),
    );
    let _ = username;
}

fn watch_lid_and_lock() {
    let lid_path = "/proc/acpi/button/lid/LID0/state";
    let mut last_closed = false;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if let Ok(content) = std::fs::read_to_string(lid_path) {
            let closed = content.to_lowercase().contains("closed");
            if closed && !last_closed {
                let exe = std::env::current_exe()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "locker".to_string());
                let _ = std::process::Command::new(&exe).spawn();
            }
            last_closed = closed;
        }
        if std::env::var("FINICK_LOCKER_WATCH_LID").is_err() {
            break;
        }
    }
}
