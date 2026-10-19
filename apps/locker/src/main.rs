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

#[derive(Clone, PartialEq)]
struct LockMedia {
    available: bool,
    title: String,
    artist: String,
    status: String,
}

impl LockMedia {
    fn unavailable() -> Self {
        Self { available: false, title: String::new(), artist: String::new(), status: String::new() }
    }

    fn headline(&self) -> String {
        let t = self.title.trim();
        let a = self.artist.trim();
        if t.is_empty() && a.is_empty() {
            return "No track info".to_string();
        }
        if a.is_empty() {
            return t.to_string();
        }
        if t.is_empty() {
            return a.to_string();
        }
        format!("{t} — {a}")
    }

    fn playing(&self) -> bool { self.status.eq_ignore_ascii_case("playing") }
}

#[derive(Clone, PartialEq)]
struct LockNotif {
    app: String,
    summary: String,
}

impl LockNotif {
    fn row_text(&self) -> String {
        let a = self.app.trim();
        let s = self.summary.trim().replace('\n', " ");
        if a.is_empty() { s } else { format!("{a}: {s}") }
    }
}

fn playerctl_output(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("playerctl").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn query_lock_media() -> LockMedia {
    if playerctl_output(&["--version"]).is_none() {
        return LockMedia::unavailable();
    }
    let title = playerctl_output(&["metadata", "--format", "{{title}}"]).unwrap_or_default();
    let artist = playerctl_output(&["metadata", "--format", "{{artist}}"]).unwrap_or_default();
    let status = playerctl_output(&["status"]).unwrap_or_default();
    if title.is_empty() && artist.is_empty() && status.is_empty() {
        return LockMedia::unavailable();
    }
    LockMedia { available: true, title, artist, status }
}

fn query_lock_notifs() -> Vec<LockNotif> {
    ipsea::notifications::get_active_notifications(ipsea::notifications::NOTIFICATIONS_SOCKET_NAME)
        .unwrap_or_default()
        .into_iter()
        .take(3)
        .map(|n| LockNotif { app: n.app_name, summary: n.summary })
        .filter(|n| !n.row_text().trim().is_empty())
        .collect()
}

fn locker_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let _t = use_app_theme();
    let media = use_state(LockMedia::unavailable);
    let notifs = use_state(Vec::<LockNotif>::new);
    let mut password = use_state(String::new);
    let mut error = use_state(|| Option::<String>::None);
    let authenticating = use_state(|| false);
    let attempts = use_state(|| 0u32);
    let time_str = use_state(|| chrono::Local::now().format("%H:%M").to_string());
    let date_str = use_state(|| chrono::Local::now().format("%A, %B %-d").to_string());
    let battery = use_state(|| HyprlandBackend.get_power_info());
    let wallpaper = use_hook(parse_wallpaper);
    let username = system::current_username();
    let hostname = use_hook(|| HyprlandBackend.get_host_info().hostname);
    let initial = username.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string());

    use_hook(move || {
        let mut ts = time_str;
        let mut ds = date_str;
        let mut bat = battery;
        let mut med = media;
        let mut nts = notifs;
        spawn(async move {
            // First snapshot up front so the cards never show stale data.
            let m0 = tokio::task::spawn_blocking(query_lock_media).await.unwrap_or(LockMedia::unavailable());
            med.set_if_modified(m0);
            let n0 = tokio::task::spawn_blocking(query_lock_notifs).await.unwrap_or_default();
            nts.set_if_modified(n0);
            let mut tick = 0u32;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                ts.set(chrono::Local::now().format("%H:%M").to_string());
                ds.set(chrono::Local::now().format("%A, %B %-d").to_string());
                tick += 1;
                if tick % 5 == 0 {
                    let m = tokio::task::spawn_blocking(query_lock_media).await.unwrap_or(LockMedia::unavailable());
                    med.set_if_modified(m);
                    let n = tokio::task::spawn_blocking(query_lock_notifs).await.unwrap_or_default();
                    nts.set_if_modified(n);
                }
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

fn media_press(action: &'static str, media: State<LockMedia>) -> impl FnMut() + 'static {
    move || {
        spawn(async move {
            let mut med = media;
            // playerctl is the only transport backend here; when it is
            // missing the media card stays hidden and this never fires.
            let ran = tokio::task::spawn_blocking(move || {
                std::process::Command::new("playerctl")
                    .arg(action)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            })
            .await
            .unwrap_or(false);
            if ran {
                let m = tokio::task::spawn_blocking(query_lock_media).await.unwrap_or(LockMedia::unavailable());
                med.set_if_modified(m);
            }
        });
    }
}

    let do_unlock = {
        let pwd = password;
        let mut err = error;
        let att = attempts;
        let user = username.clone();
        let mut busy = authenticating;
        move |_| {
            if *busy.read() {
                return;
            }
            let p = pwd.read().clone();
            if p.is_empty() {
                err.set(Some("Enter your password".to_string()));
                return;
            }
            err.set(None);
            busy.set(true);
            let user_c = user.clone();
            let mut err_c = err;
            let mut att_c = att;
            let mut pwd_c = pwd;
            let mut busy_c = busy;
            spawn(async move {
                let p2 = p.clone();
                let user2 = user_c.clone();
                let ok = tokio::task::spawn_blocking(move || system::authenticate_user(&user2, &p2)).await.unwrap_or(false);
                if ok {
                    release_single_instance();
                    start_shell_overlay();
                    std::process::exit(0);
                } else {
                    busy_c.set(false);
                    err_c.set(Some("Incorrect password - try again".to_string()));
                    let next = *att_c.read() + 1;
                    att_c.set(next);
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
    let is_authenticating = *authenticating.read();
    let lock_media = media.read().clone();
    let lock_notifs = notifs.read().clone();
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
                    .child(
                        ImageViewer::new(PathBuf::from(path))
                            .width(Size::fill())
                            .height(Size::fill())
                            .aspect_ratio(AspectRatio::Max)
                            .image_cover(ImageCover::Center),
                    )
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
                    FadeSlideIn::new().child(
                        rect()
                            .width(Size::px(380.))
                            .padding(24.)
                            .corner_radius(24.)
                            .background(Color::from_argb(205, 26, 27, 33))
                            .border(Border::new().width(1.).fill(Color::from_argb(55, 255, 255, 255)))
                            .shadow(Shadow::new().blur(40.).color(Color::from_argb(120, 0, 0, 0)))
                            .spacing(14.)
                            .content(Content::Flex)
                            .child(hero_identity(initial.clone(), username.clone(), hostname.clone()))
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
                                    .child(
                                        Input::new(pwd_writable)
                                            .width(Size::fill())
                                            .placeholder("Password")
                                            .mode(InputMode::new_password())
                                            .auto_focus(true)
                                            .theme_colors(InputColorsThemePartial {
                                                background: Some(Color::from_argb(255, 14, 15, 19).into()),
                                                focus_background: Some(Color::from_argb(255, 14, 15, 19).into()),
                                                border_fill: Some(Color::TRANSPARENT.into()),
                                                focus_border_fill: Some(Color::TRANSPARENT.into()),
                                                color: Some(Color::WHITE.into()),
                                                placeholder_color: Some(Color::from_rgb(150, 151, 158).into()),
                                            })
                                            .on_submit({
                                                let mut h = do_unlock.clone();
                                                move |_| h(())
                                            }),
                                    ),
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
                            .child(primary_button_full(if is_authenticating { "Checking..." } else { "Unlock" }, {
                                let mut h = do_unlock.clone();
                                move || h(())
                            }))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .cross_align(Alignment::Center)
                                    .spacing(4.)
                                    .child(
                                        label()
                                            .font_size(11.)
                                            .color(Color::from_rgb(165, 166, 174))
                                            .text("Having trouble? This bypasses password checking."),
                                    )
                                    .child(secondary_button_full("Emergency exit", emergency_exit)),
                            )
                            .into_element(),
                    )
                    .maybe(lock_media.available, |el| {
                        let headline = lock_media.headline();
                        let playing = lock_media.playing();
                        let status_label = if lock_media.status.trim().is_empty() {
                            "Playing".to_string()
                        } else {
                            lock_media.status.clone()
                        };
                        el.child(
                            rect()
                                .width(Size::px(380.))
                                .padding(18.)
                                .corner_radius(24.)
                                .background(Color::from_argb(205, 26, 27, 33))
                                .border(Border::new().width(1.).fill(Color::from_argb(55, 255, 255, 255)))
                                .shadow(Shadow::new().blur(40.).color(Color::from_argb(120, 0, 0, 0)))
                                .spacing(10.)
                                .content(Content::Flex)
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .content(Content::Flex)
                                        .spacing(8.)
                                        .child(icon(MUSIC, 15., Color::from_rgb(200, 200, 210)))
                                        .child(
                                            label()
                                                .font_size(13.)
                                                .font_weight(FontWeight::SEMI_BOLD)
                                                .color(Color::WHITE)
                                                .text(headline),
                                        ),
                                )
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .content(Content::Flex)
                                        .spacing(8.)
                                        .child(status_chip(status_label, playing, None))
                                        .child(
                                            label()
                                                .font_size(11.)
                                                .color(Color::from_rgb(165, 166, 174))
                                                .text("Media keys still work while locked."),
                                        ),
                                )
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .horizontal()
                                        .main_align(Alignment::Center)
                                        .content(Content::Flex)
                                        .spacing(8.)
                                        .child(ghost_button("Prev", media_press("previous", media)))
                                        .child(ghost_button("Play/Pause", media_press("play-pause", media)))
                                        .child(ghost_button("Next", media_press("next", media))),
                                ),
                        )
                    })
                    .maybe(!lock_notifs.is_empty(), |el| {
                        let count = lock_notifs.len();
                        let mut card = rect()
                            .width(Size::px(380.))
                            .padding(18.)
                            .corner_radius(24.)
                            .background(Color::from_argb(205, 26, 27, 33))
                            .border(Border::new().width(1.).fill(Color::from_argb(55, 255, 255, 255)))
                            .shadow(Shadow::new().blur(40.).color(Color::from_argb(120, 0, 0, 0)))
                            .spacing(8.)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .content(Content::Flex)
                                    .spacing(8.)
                                    .child(icon(NOTIFICATIONS, 14., Color::from_rgb(200, 200, 210)))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(Color::WHITE)
                                            .text(format!("Notifications ({count})")),
                                    ),
                            );
                        for n in lock_notifs.iter().take(3) {
                            let text = n.row_text();
                            card = card.child(
                                label().font_size(12.).color(Color::from_rgb(210, 210, 215)).text(text),
                            );
                        }
                        el.child(card.child(
                            label()
                                .font_size(11.)
                                .color(Color::from_rgb(165, 166, 174))
                                .text("Bodies hidden while locked."),
                        ))
                    }),
                ),
        )
        .into_element()
}

// P3 status: this Freya window-layer locker is bypassable by design (any
// shell access defeats it) and stays only as the session-lock
// `finished`-event fallback. Super+L and all trigger paths resolve to
// `finick-lock` first (see system::locker_binary); both `finick-lock`
// entry points exec this binary when the session client exits 42
// (denied/superseded/manager missing). The per-window audit loop below is
// retained on purpose: HM ships no locker window rules, so the fallback
// self-pins while shown. Emergency exit stays as a safety valve so a
// broken auth stack can never brick the session.
#[derive(Clone)]
struct LockerMonitor {
    name: String,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

fn locker_monitors() -> Vec<LockerMonitor> {
    let mut mons: Vec<LockerMonitor> = HyprlandBackend
        .get_displays()
        .into_iter()
        .map(|d| {
            let (raw_w, raw_h) = system::parse_display_dimensions(&d.resolution);
            let (w, h) = if d.transform == 1 || d.transform == 3 { (raw_h, raw_w) } else { (raw_w, raw_h) };
            let scale: f64 = d.scale.parse().unwrap_or(1.0);
            let scale = if scale <= 0.0 { 1.0 } else { scale };
            LockerMonitor {
                name: d.name,
                x: d.x as i64,
                y: d.y as i64,
                w: (w as f64 / scale).round() as i64,
                h: (h as f64 / scale).round() as i64,
            }
        })
        .collect();
    if mons.is_empty() {
        mons.push(LockerMonitor { name: "default".to_string(), x: 0, y: 0, w: 1920, h: 1080 });
    }
    mons
}

fn claim_single_instance() -> bool {
    if system::locker_process_alive() {
        return false;
    }
    let _ = std::fs::remove_file(system::locker_lockfile());
    let _ = std::fs::write(system::locker_lockfile(), std::process::id().to_string());
    true
}

fn release_single_instance() { let _ = std::fs::remove_file(system::locker_lockfile()); }

fn stop_shell_overlay() {
    let _ = std::process::Command::new("systemctl").args(["--user", "stop", "finick-overlay"]).output();
}

fn start_shell_overlay() {
    let _ = std::process::Command::new("systemctl").args(["--user", "start", "finick-overlay"]).output();
}

fn emergency_exit() {
    release_single_instance();
    start_shell_overlay();
    std::process::exit(0);
}

fn locker_audit_lua(mons: &[LockerMonitor]) -> String {
    let mut chunks: Vec<String> = mons
        .iter()
        .map(|m| {
            format!(
                r#"
            for _, w in ipairs(hl.get_windows()) do
                if w.title == 'locker-{name}' then
                    local addr = 'address:' .. tostring(w.address)
                    if not w.floating or not w.pinned or math.floor(w.at.x) ~= {x} or math.floor(w.at.y) ~= {y} or math.floor(w.size.x) ~= {w} or math.floor(w.size.y) ~= {h} then
                        hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                        hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                        hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {w}, y = {h}, relative = false }}))
                        hl.dispatch(hl.dsp.window.move({{ window = addr, x = {x}, y = {y} }}))
                    end
                    hl.dispatch(hl.dsp.window.bring_to_top({{ window = addr }}))
                end
            end"#,
                name = m.name,
                w = m.w,
                h = m.h,
                x = m.x,
                y = m.y
            )
        })
        .collect();
    if let Some(first) = mons.first() {
        chunks.push(format!(
            r#"
            local aw = hl.get_active_window()
            if aw == nil or aw.class ~= 'locker' then
                for _, w in ipairs(hl.get_windows()) do
                    if w.title == 'locker-{name}' then
                        hl.dispatch(hl.dsp.focus({{ window = 'address:' .. tostring(w.address) }}))
                    end
                end
            end"#,
            name = first.name
        ));
    }
    chunks.join("\n")
}

fn place_locker_windows(mons: Vec<LockerMonitor>) {
    std::thread::spawn(move || {
        let lua = locker_audit_lua(&mons);
        for wait in [800u64, 1500, 2500] {
            std::thread::sleep(std::time::Duration::from_millis(wait));
            let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
        }
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !system::locker_locked() {
                break;
            }
            let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
        }
    });
}

/// Locker has no sub-pages; this lists the (empty) page set for `--help`.
pub const LOCKER_PAGE_SLUGS: &[&str] = &[];

fn main() {
    let args = config::LaunchArgs::parse();
    if args.wants_help() {
        args.print_help("locker", LOCKER_PAGE_SLUGS);
        return;
    }
    if args.page.is_some() {
        eprintln!("[locker] locker has no pages; ignoring page argument");
    }
    args.maybe_arm_screenshot("locker");
    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());

    if !claim_single_instance() {
        return;
    }
    stop_shell_overlay();

    let mons = locker_monitors();
    place_locker_windows(mons.clone());
    let _ = std::thread::spawn(watch_lid_and_lock);
    let mut launch_config = LaunchConfig::new().with_exit_on_close(false);
    for m in &mons {
        let title: &'static str = Box::leak(format!("locker-{}", m.name).into_boxed_str());
        launch_config = launch_config.with_window(
            WindowConfig::new(locker_app)
                .with_title(title)
                .with_app_id("locker")
                .with_size(m.w as f64, m.h as f64)
                .with_decorations(false)
                .with_transparency(false)
                .with_background(Color::from_rgb(12, 12, 16))
                .with_on_close(|_ctx, _wid| CloseDecision::KeepOpen),
        );
    }
    launch(launch_config);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn media(title: &str, artist: &str, status: &str) -> LockMedia {
        LockMedia { available: true, title: title.to_string(), artist: artist.to_string(), status: status.to_string() }
    }

    #[test]
    fn headline_joins_title_and_artist() {
        assert_eq!(media("Song", "Band", "Playing").headline(), "Song — Band");
        assert_eq!(media("Song", "", "Paused").headline(), "Song");
        assert_eq!(media("", "Band", "Playing").headline(), "Band");
        assert_eq!(media("", "", "Stopped").headline(), "No track info");
    }

    #[test]
    fn playing_matches_case_insensitively() {
        assert!(media("t", "a", "Playing").playing());
        assert!(media("t", "a", "playing").playing());
        assert!(!media("t", "a", "Paused").playing());
    }

    #[test]
    fn notif_row_withholds_body() {
        let n = LockNotif { app: "Chat".to_string(), summary: "hi there".to_string() };
        assert_eq!(n.row_text(), "Chat: hi there");
        let n = LockNotif { app: "".to_string(), summary: "update done".to_string() };
        assert_eq!(n.row_text(), "update done");
    }
}
