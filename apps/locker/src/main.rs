#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
use {
    freya::prelude::*,
    std::path::PathBuf,
    system::{HyprlandBackend, SystemBackend},
    ui::*,
};

fn parse_wallpaper() -> Option<WallpaperBg> {
    let raw = HyprlandBackend.get_current_wallpaper()?;
    let s = raw.trim().to_string();
    if s.is_empty() {
        return None;
    }
    let clean = s.strip_prefix("file://").unwrap_or(&s);
    if std::path::Path::new(clean).is_file() {
        return Some(WallpaperBg::Image(clean.to_string()));
    }
    if let Some(hex) = s
        .strip_prefix("preset:")
        .and_then(|p| p.parse::<usize>().ok())
        .and_then(|i| system::WALLPAPER_COLOR_PRESETS.get(i).map(|(_, c)| *c))
    {
        return Some(WallpaperBg::Color(hex.to_string()));
    }
    if s.starts_with('#') && (s.len() == 7 || s.len() == 4) {
        return Some(WallpaperBg::Color(s));
    }
    None
}

#[derive(Clone, PartialEq)]
enum WallpaperBg {
    Image(String),
    Color(String),
}

fn hex_to_color(hex: &str) -> Option<Color> {
    let h = hex.trim_start_matches('#');
    let (r, g, b) = match h.len() {
        6 => (
            u8::from_str_radix(&h[0..2], 16).ok()?,
            u8::from_str_radix(&h[2..4], 16).ok()?,
            u8::from_str_radix(&h[4..6], 16).ok()?,
        ),
        3 => {
            let r = u8::from_str_radix(&h[0..1], 16).ok()?;
            let g = u8::from_str_radix(&h[1..2], 16).ok()?;
            let b = u8::from_str_radix(&h[2..3], 16).ok()?;
            (r * 17, g * 17, b * 17)
        }
        _ => return None,
    };
    Some(Color::from_rgb(r, g, b))
}

fn locker_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let mut password = use_state(String::new);
    let mut error = use_state(|| Option::<String>::None);
    let attempts = use_state(|| 0u32);
    let time_str = use_state(|| chrono::Local::now().format("%H:%M").to_string());
    let date_str = use_state(|| chrono::Local::now().format("%A, %B %-d").to_string());
    let battery = use_state(|| HyprlandBackend.get_power_info());
    let wallpaper = use_hook(|| parse_wallpaper());
    let username = system::current_username();
    let hostname = use_hook(|| HyprlandBackend.get_host_info().hostname);
    let initial = username.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string());

    use_hook(move || {
        let mut ts = time_str;
        let mut ds = date_str;
        let mut bat = battery;
        spawn(async move {
            let mut tick = 0u32;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                ts.set(chrono::Local::now().format("%H:%M").to_string());
                ds.set(chrono::Local::now().format("%A, %B %-d").to_string());
                tick += 1;
                if tick % 30 == 0 {
                    let info = tokio::task::spawn_blocking(|| HyprlandBackend.get_power_info()).await.unwrap_or(
                        system::PowerInfo {
                            capacity: "Unknown".to_string(),
                            status: "Unknown".to_string(),
                            health_percent: None,
                            cycle_count: None,
                        },
                    );
                    bat.set(info);
                }
            }
        });
    });

    let do_unlock = {
        let pwd = password;
        let mut err = error;
        let att = attempts;
        let user = username.clone();
        move |_| {
            let p = pwd.read().clone();
            if p.is_empty() {
                err.set(Some("Enter your password".to_string()));
                return;
            }
            err.set(None);
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
                    err_c.set(Some("Incorrect password — try again".to_string()));
                    att_c.set(*att_c.read() + 1);
                    pwd_c.set(String::new());
                }
            });
        }
    };

    let bg_fallback = wallpaper
        .as_ref()
        .and_then(|w| match w {
            WallpaperBg::Color(hex) => hex_to_color(hex),
            WallpaperBg::Image(_) => None,
        })
        .unwrap_or(Color::from_rgb(12, 12, 16));
    let pwd_writable = password.into_writable();
    let has_error = error.read().is_some();
    let err_msg = error.read().clone().unwrap_or_default();
    let fail_count = *attempts.read();
    let bat_info = battery.read().clone();
    let bat_txt = if bat_info.capacity == "Unknown" || bat_info.capacity.is_empty() {
        String::new()
    } else if bat_info.status.is_empty() || bat_info.status == "Unknown" {
        bat_info.capacity.clone()
    } else {
        format!("{} · {}", bat_info.capacity, bat_info.status)
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(bg_fallback)
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
        .child({
            let wallpaper = wallpaper.clone();
            match wallpaper {
                Some(WallpaperBg::Image(path)) => rect()
                    .position(Position::new_global().top(0.).left(0.))
                    .width(Size::fill())
                    .height(Size::fill())
                    .overflow(Overflow::Clip)
                    .child(ImageViewer::new(PathBuf::from(path)).width(Size::fill()).height(Size::fill()))
                    .into_element(),
                _ => rect().into_element(),
            }
        })
        .child(
            rect()
                .position(Position::new_global().top(0.).left(0.))
                .width(Size::fill())
                .height(Size::fill())
                .background(Color::from_argb(150, 0, 0, 0)),
        )
        .child(
            rect()
                .position(Position::new_global().top(18.).right(18.))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .padding((8., 14.))
                .corner_radius(999.)
                .background(Color::from_argb(140, 20, 21, 26))
                .border(Border::new().width(1.).fill(Color::from_argb(60, 255, 255, 255)))
                .child(icon(BATTERY, 14., Color::from_rgb(220, 220, 225)))
                .child(
                    label()
                        .font_size(12.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(Color::from_rgb(235, 235, 240))
                        .text(if bat_txt.is_empty() { hostname.clone() } else { format!("{bat_txt}  ·  {hostname}") }),
                ),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .vertical()
                .main_align(Alignment::Center)
                .cross_align(Alignment::Center)
                .spacing(18.)
                .content(Content::Flex)
                .child(
                    rect()
                        .vertical()
                        .cross_align(Alignment::Center)
                        .spacing(4.)
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(8.)
                                .padding((6., 14.))
                                .corner_radius(999.)
                                .background(Color::from_argb(130, 20, 21, 26))
                                .border(Border::new().width(1.).fill(Color::from_argb(60, 255, 255, 255)))
                                .child(icon(LOCK, 12., Color::from_rgb(220, 220, 225)))
                                .child(
                                    label()
                                        .font_size(11.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(Color::from_rgb(220, 220, 225))
                                        .text("LOCKED"),
                                ),
                        )
                        .child(
                            label()
                                .font_size(76.)
                                .font_weight(FontWeight::BOLD)
                                .color(Color::WHITE)
                                .text(time_str.read().clone()),
                        )
                        .child(
                            label()
                                .font_size(15.)
                                .font_weight(FontWeight::MEDIUM)
                                .color(Color::from_rgb(210, 210, 215))
                                .text(date_str.read().clone()),
                        ),
                )
                .child(
                    rect()
                        .width(Size::px(380.))
                        .padding(24.)
                        .corner_radius(24.)
                        .background(Color::from_argb(205, 26, 27, 33))
                        .border(Border::new().width(1.).fill(Color::from_argb(55, 255, 255, 255)))
                        .shadow(Shadow::new().blur(40.).color(Color::from_argb(120, 0, 0, 0)))
                        .spacing(14.)
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(12.)
                                .content(Content::Flex)
                                .child(
                                    rect()
                                        .width(Size::px(52.))
                                        .height(Size::px(52.))
                                        .corner_radius(999.)
                                        .background(t.accent)
                                        .center()
                                        .child(
                                            label()
                                                .font_size(20.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(Color::WHITE)
                                                .text(initial),
                                        ),
                                )
                                .child(
                                    rect()
                                        .vertical()
                                        .spacing(2.)
                                        .child(
                                            label()
                                                .font_size(15.)
                                                .font_weight(FontWeight::SEMI_BOLD)
                                                .color(Color::WHITE)
                                                .text(username.clone()),
                                        )
                                        .child(
                                            label()
                                                .font_size(12.)
                                                .color(Color::from_rgb(170, 171, 178))
                                                .text(hostname.clone()),
                                        ),
                                ),
                        )
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(46.))
                                .corner_radius(14.)
                                .background(Color::from_argb(255, 14, 15, 19))
                                .border(Border::new().width(1.).fill(if has_error {
                                    Color::from_rgb(180, 70, 70)
                                } else {
                                    Color::from_argb(50, 255, 255, 255)
                                }))
                                .padding((0., 14.))
                                .cross_align(Alignment::Center)
                                .content(Content::Flex)
                                .horizontal()
                                .spacing(10.)
                                .child(icon(LOCK, 15., Color::from_rgb(150, 151, 158)))
                                .child(Input::new(pwd_writable).width(Size::fill()).placeholder("Password").on_submit({
                                    let mut h = do_unlock.clone();
                                    move |_| h(())
                                })),
                        )
                        .maybe(has_error, |el| {
                            el.child(
                                rect()
                                    .width(Size::fill())
                                    .padding((8., 12.))
                                    .corner_radius(12.)
                                    .background(Color::from_argb(255, 52, 22, 22))
                                    .border(Border::new().width(1.).fill(Color::from_rgb(130, 55, 55)))
                                    .child(
                                        label()
                                            .font_size(12.)
                                            .font_weight(FontWeight::MEDIUM)
                                            .color(Color::from_rgb(255, 140, 140))
                                            .text(err_msg.clone()),
                                    ),
                            )
                        })
                        .maybe(fail_count > 0 && !has_error, |el| {
                            el.child(rect().width(Size::fill()).center().child(
                                label().font_size(11.).color(Color::from_rgb(150, 151, 158)).text(format!(
                                    "{fail_count} failed {} — Caps Lock?",
                                    if fail_count == 1 { "attempt" } else { "attempts" }
                                )),
                            ))
                        })
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(46.))
                                .corner_radius(14.)
                                .background(t.accent)
                                .center()
                                .cursor(CursorIcon::Pointer)
                                .on_press({
                                    let mut h = do_unlock.clone();
                                    move |_: Event<PressEventData>| h(())
                                })
                                .child(
                                    rect()
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .spacing(8.)
                                        .content(Content::Flex)
                                        .child(
                                            label()
                                                .font_size(14.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(Color::WHITE)
                                                .text("Unlock"),
                                        )
                                        .child(icon(ARROW_RIGHT, 15., Color::WHITE)),
                                ),
                        ),
                )
                .child(
                    label()
                        .font_size(11.)
                        .color(Color::from_argb(160, 235, 235, 240))
                        .text("Enter unlocks  •  Esc clears  •  Lid close locks"),
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
