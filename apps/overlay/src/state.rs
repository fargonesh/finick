use {
    freya::prelude::*,
    ipsea::settings::{
        SETTINGS_SOCKET_NAME, SettingEntry, SettingKey, SettingValue, SettingsEvent, SubscriptionFilter,
        get_all_settings, set_and_apply,
    },
    std::{
        sync::{
            Mutex,
            atomic::{AtomicBool, AtomicU64, Ordering},
        },
        time::{Duration, Instant},
    },
    system::{HyprlandBackend, SystemBackend},
};

pub use ipsea::notifications::{
    close_notification, subscribe_channel, Notification, NotificationEvent, NOTIFICATIONS_SOCKET_NAME,
};

pub fn socket() -> String {
    SETTINGS_SOCKET_NAME.to_string()
}

#[allow(dead_code)]
pub fn apply_entry(
    entry: &SettingEntry,
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
) {
    match entry.key {
        SettingKey::WifiEnabled => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = wifi;
                s.set_if_modified(v);
            }
        }
        SettingKey::BluetoothEnabled => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = bt;
                s.set_if_modified(v);
            }
        }
        SettingKey::AudioVolume => {
            if let Some(v) = entry.value.as_f64() {
                let mut s = volume;
                s.set_if_modified(v);
            }
        }
        SettingKey::AudioMuted => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = muted;
                s.set_if_modified(v);
            }
        }
        SettingKey::DisplayBrightness => {
            if let Some(v) = entry.value.as_f64() {
                let mut s = brightness;
                s.set_if_modified(v);
            }
        }
        SettingKey::DoNotDisturb => {
            let on = entry.value.as_str().map(|s| s != "off").unwrap_or(false)
                || entry.value.as_bool().unwrap_or(false);
            let mut s = dnd;
            s.set_if_modified(on);
        }
        SettingKey::PowerProfile => {}
        SettingKey::AirplaneMode => {}
        SettingKey::DisplayNightShift => {}
        _ => {}
    }
}

pub fn apply_extended_entry(
    entry: &SettingEntry,
    airplane: State<bool>,
    night: State<bool>,
    power_profile: State<String>,
) {
    match entry.key {
        SettingKey::AirplaneMode => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = airplane;
                s.set_if_modified(v);
            }
        }
        SettingKey::DisplayNightShift => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = night;
                s.set_if_modified(v);
            }
        }
        SettingKey::PowerProfile => {
            if let Some(s) = entry.value.as_str() {
                let mut p = power_profile;
                p.set_if_modified(s.to_string());
            }
        }
        _ => {}
    }
}

pub fn load_extended_batch(
    airplane: State<bool>,
    night: State<bool>,
    power_profile: State<String>,
) {
    spawn(async move {
        loop {
            let res = tokio::task::spawn_blocking(|| get_all_settings(socket())).await;
            match res {
                Ok(Ok(entries)) => {
                    for entry in &entries {
                        apply_extended_entry(entry, airplane, night, power_profile);
                    }
                    break;
                }
                Ok(Err(e)) => {
                    eprintln!("panel extended get_all_settings offline: {e}; retrying");
                }
                Err(e) => {
                    eprintln!("panel extended get_all_settings join failed: {e}; retrying");
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

pub fn subscribe_extended_live(
    airplane: State<bool>,
    night: State<bool>,
    power_profile: State<String>,
) {
    spawn(async move {
        loop {
            let rx = match ipsea::settings::subscribe_channel(socket(), SubscriptionFilter::all()) {
                Ok(rx) => rx,
                Err(e) => {
                    eprintln!("panel extended subscribe failed: {e}; retrying");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            let mut rx = rx;
            while let Some(evt) = rx.recv().await {
                match evt {
                    SettingsEvent::Changed { key, value, .. } => match key {
                        SettingKey::AirplaneMode => {
                            if let Some(v) = value.as_bool() {
                                let mut s = airplane;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::DisplayNightShift => {
                            if let Some(v) = value.as_bool() {
                                let mut s = night;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::PowerProfile => {
                            if let Some(s) = value.as_str() {
                                let mut p = power_profile;
                                p.set_if_modified(s.to_string());
                            }
                        }
                        _ => {}
                    },
                    SettingsEvent::Reloaded => {
                        load_extended_batch(airplane, night, power_profile);
                    }
                    _ => {}
                }
            }
            eprintln!("panel extended subscribe disconnected; retrying");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

pub fn open_in_settings() {
    std::thread::spawn(|| {
        if std::process::Command::new("finick-settings").spawn().is_err() {
            system::hypr_exec("finick-settings");
        }
    });
}

pub fn parse_capacity_pct(capacity: &str) -> u8 {
    capacity.trim().trim_end_matches('%').parse::<u8>().unwrap_or(0).min(100)
}

pub fn load_initial_batch(
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
    connected: State<bool>,
    mut topbar_settings: State<TopbarSettings>,
    mut theme_state: State<ui::AppTheme>,
) {
    spawn(async move {
        loop {
            let res = tokio::task::spawn_blocking(|| get_all_settings(socket())).await;
            let entries = match res {
                Ok(Ok(entries)) => entries,
                Ok(Err(e)) => {
                    eprintln!("topbar get_all_settings offline: {e}; retrying");
                    let mut c = connected;
                    c.set_if_modified(false);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
                Err(e) => {
                    eprintln!("topbar get_all_settings join failed: {e}; retrying");
                    let mut c = connected;
                    c.set_if_modified(false);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            for entry in &entries {
                apply_entry(entry, wifi, bt, volume, muted, brightness, dnd);
                if let SettingKey::Custom(ref k) = entry.key {
                    let mut ts = topbar_settings.read().clone();
                    if apply_topbar_custom(&mut ts, k.as_str(), &entry.value) {
                        topbar_settings.set(ts);
                    }
                }
                if entry.key == SettingKey::AccentColor {
                    if let Some(s) = entry.value.as_str() {
                        if let Some(acc) = ui::ACCENTS.iter().find(|a| a.name.eq_ignore_ascii_case(s) || a.hex.eq_ignore_ascii_case(s)) {
                            let new_t = theme_state.read().with_accent(*acc);
                            theme_state.set(new_t);
                            ui::set_theme(&new_t);
                        }
                    }
                }
                if entry.key == SettingKey::ThemeMode {
                    if let Some(s) = entry.value.as_str() {
                        let mode = match s { "light" => ui::ThemeMode::Light, "auto" => ui::ThemeMode::Auto, _ => ui::ThemeMode::Dark };
                        let new_t = theme_state.read().with_mode(mode);
                        theme_state.set(new_t);
                        ui::set_theme(&new_t);
                    }
                }
            }
            let mut c = connected;
            c.set_if_modified(true);
            break;
        }
    });
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopbarSettings {
    pub show_wifi: bool,
    pub show_bluetooth: bool,
    pub show_sound: bool,
    pub show_battery: bool,
    pub show_wired: bool,
    pub show_notifications: bool,
    pub text_size: f64,
    pub text_color: String,
    pub text_color_light: String,
    pub text_color_dark: String,
    pub theme: String,
    pub icon_size: f64,
    pub icon_stroke: f64,
}

impl Default for TopbarSettings {
    fn default() -> Self {
        Self {
            show_wifi: true,
            show_bluetooth: true,
            show_sound: true,
            show_battery: true,
            show_wired: true,
            show_notifications: true,
            text_size: 13.0,
            text_color: "Default".to_string(),
            text_color_light: String::new(),
            text_color_dark: String::new(),
            theme: "system".to_string(),
            icon_size: 16.0,
            icon_stroke: 1.6,
        }
    }
}

impl TopbarSettings {
    pub fn effective_text_color(&self, is_light: bool) -> &str {
        let override_c = if is_light { &self.text_color_light } else { &self.text_color_dark };
        if !override_c.is_empty() && *override_c != "Inherit" {
            override_c.as_str()
        } else {
            self.text_color.as_str()
        }
    }
}

fn apply_topbar_custom(ts: &mut TopbarSettings, k: &str, value: &SettingValue) -> bool {
    match k {
        "topbar.show_wifi" => if let Some(b) = value.as_bool() { ts.show_wifi = b; return true; },
        "topbar.show_bluetooth" => if let Some(b) = value.as_bool() { ts.show_bluetooth = b; return true; },
        "topbar.show_sound" => if let Some(b) = value.as_bool() { ts.show_sound = b; return true; },
        "topbar.show_battery" => if let Some(b) = value.as_bool() { ts.show_battery = b; return true; },
        "topbar.show_wired" => if let Some(b) = value.as_bool() { ts.show_wired = b; return true; },
        "topbar.show_brightness" => return true,
        "topbar.show_notifications" => if let Some(b) = value.as_bool() { ts.show_notifications = b; return true; },
        "topbar.show_focus" => return true,
        "topbar.text_size" => if let Some(f) = value.as_f64() { ts.text_size = f.clamp(10.0, 18.0); return true; },
        "topbar.text_color" => if let Some(s) = value.as_str() { ts.text_color = s.to_string(); return true; },
        "topbar.text_color_light" => if let Some(s) = value.as_str() { ts.text_color_light = s.to_string(); return true; },
        "topbar.text_color_dark" => if let Some(s) = value.as_str() { ts.text_color_dark = s.to_string(); return true; },
        "topbar.theme" => if let Some(s) = value.as_str() { ts.theme = s.to_string(); return true; },
        "topbar.icon_size" => {
            if let Some(f) = value.as_f64() { ts.icon_size = f.clamp(12.0, 20.0); return true; }
            else if let Some(i) = value.as_i64() { ts.icon_size = (i as f64).clamp(12.0, 20.0); return true; }
        }
        "topbar.icon_stroke" => {
            if let Some(f) = value.as_f64() { ts.icon_stroke = f.clamp(1.0, 3.0); return true; }
            else if let Some(i) = value.as_i64() { ts.icon_stroke = (i as f64).clamp(1.0, 3.0); return true; }
        }
        _ => {}
    }
    false
}

pub fn subscribe_live(
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
    connected: State<bool>,
    mut topbar_settings: State<TopbarSettings>,
    theme_state: State<ui::AppTheme>,
) {
    spawn(async move {
        loop {
            let rx = match ipsea::settings::subscribe_channel(socket(), SubscriptionFilter::all()) {
                Ok(rx) => rx,
                Err(e) => {
                    eprintln!("topbar subscribe failed: {e}; retrying");
                    let mut c = connected;
                    c.set_if_modified(false);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            let mut rx = rx;
            {
                let mut c = connected;
                c.set_if_modified(true);
            }
            // Re-pull full state on every (re)connect so changes made while
            // offline (or before finickd was up) are applied, including
            // AccentColor / ThemeMode.
            load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
            while let Some(evt) = rx.recv().await {
                match evt {
                    SettingsEvent::Changed { key, value, source: _ } => match key {
                        SettingKey::WifiEnabled => {
                            if let Some(v) = value.as_bool() {
                                let mut s = wifi;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::BluetoothEnabled => {
                            if let Some(v) = value.as_bool() {
                                let mut s = bt;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::AudioVolume => {
                            if let Some(v) = value.as_f64() {
                                let mut s = volume;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::AudioMuted => {
                            if let Some(v) = value.as_bool() {
                                let mut s = muted;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::DisplayBrightness => {
                            if let Some(v) = value.as_f64() {
                                let mut s = brightness;
                                s.set_if_modified(v);
                            }
                        }
                        SettingKey::DoNotDisturb => {
                            let on = value.as_str().map(|s| s != "off").unwrap_or(false)
                                || value.as_bool().unwrap_or(false);
                            let mut s = dnd;
                            s.set_if_modified(on);
                        }
                        SettingKey::Custom(ref k) => {
                            let mut ts = topbar_settings.read().clone();
                            if apply_topbar_custom(&mut ts, k.as_str(), &value) {
                                topbar_settings.set(ts);
                            }
                        }
                        SettingKey::AccentColor => {
                            if let Some(s) = value.as_str() {
                                if let Some(acc) = ui::ACCENTS.iter().find(|a| a.name.eq_ignore_ascii_case(s) || a.hex.eq_ignore_ascii_case(s)) {
                                    let mut th = theme_state;
                                    let new_t = th.read().with_accent(*acc);
                                    th.set(new_t);
                                    ui::set_theme(&new_t);
                                }
                            }
                        }
                        SettingKey::ThemeMode => {
                            if let Some(s) = value.as_str() {
                                let mode = match s { "light" => ui::ThemeMode::Light, "auto" => ui::ThemeMode::Auto, _ => ui::ThemeMode::Dark };
                                let mut th = theme_state;
                                let new_t = th.read().with_mode(mode);
                                th.set(new_t);
                                ui::set_theme(&new_t);
                            }
                        }
                        SettingKey::PowerProfile => {}
                        _ => {}
                    },
                    SettingsEvent::LockChanged { .. } => {}
                    SettingsEvent::Reloaded => {
                        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
                    }
                    SettingsEvent::Alert(_) => {}
                }
            }
            eprintln!("topbar subscribe disconnected; retrying");
            {
                let mut c = connected;
                c.set_if_modified(false);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

pub fn apply(key: SettingKey, val: SettingValue) {
    std::thread::spawn(move || {
        let _ = set_and_apply(socket(), key, val);
    });
}

pub fn clock_now_with_format(use_24h: bool) -> String {
    if use_24h {
        chrono::Local::now().format("%H:%M").to_string()
    } else {
        chrono::Local::now().format("%I:%M %p").to_string()
    }
}

pub fn power_action(action: &'static str) {
    std::thread::spawn(move || {
        let ok = match action {
            "logout" => HyprlandBackend.log_out(),
            "reboot" => HyprlandBackend.reboot(),
            "sleep" | "suspend" => HyprlandBackend.sleep(),
            _ => HyprlandBackend.power_off(),
        };
        if !ok {
            eprintln!("topbar power action {action} failed");
        }
    });
}

pub fn subscribe_notifications_live(notifications: State<Vec<Notification>>) {
    subscribe_notifications_live_from(NOTIFICATIONS_SOCKET_NAME, notifications);
}

pub fn subscribe_notifications_live_from(
    socket_name: impl Into<std::path::PathBuf> + std::fmt::Display + Clone + Send + 'static,
    notifications: State<Vec<Notification>>,
) {
    spawn(async move {
        loop {
            let sock_path = socket_name.clone();
            match subscribe_channel(sock_path.clone()) {
                Ok(mut rx) => {
                    while let Some(evt) = rx.recv().await {
                        match evt {
                            NotificationEvent::Show(notif) => {
                                let mut notifs = notifications;
                                let id = notif.id;

                                let mut current = notifs.read().clone();
                                if let Some(idx) = current.iter().position(|n| n.id == id) {
                                    current[idx] = notif;
                                } else {
                                    current.push(notif);
                                }
                                notifs.set(current);
                            }
                            NotificationEvent::Close(id) => {
                                let mut notifs = notifications;
                                let mut current = notifs.read().clone();
                                if let Some(pos) = current.iter().position(|n| n.id == id) {
                                    current.remove(pos);
                                    notifs.set(current);
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    });
}

pub fn dismiss_notification(id: u32) {
    dismiss_notification_from(NOTIFICATIONS_SOCKET_NAME, id);
}

pub fn dismiss_notification_from(
    socket_name: impl Into<std::path::PathBuf> + std::fmt::Display,
    id: u32,
) {
    let _ = close_notification(socket_name, id);
}

pub fn fetch_notifications_blocking() -> Vec<Notification> {
    fetch_notifications_blocking_from(NOTIFICATIONS_SOCKET_NAME)
}

pub fn fetch_notifications_blocking_from(
    socket_name: impl Into<std::path::PathBuf> + std::fmt::Display,
) -> Vec<Notification> {
    ipsea::notifications::get_active_notifications(socket_name).unwrap_or_default()
}

pub fn clear_notifications() {
    let notifs = fetch_notifications_blocking();
    for n in notifs {
        dismiss_notification(n.id);
    }
}

#[derive(Clone, Copy)]
struct TravelGuard {
    panel_x: f64,
    panel_y: f64,
    panel_w: f64,
    panel_h: f64,
    anchor_x: f64,
    anchor_y: f64,
    deadline: Instant,
}

static TRAVEL_GUARD: Mutex<Option<TravelGuard>> = Mutex::new(None);

pub fn arm_travel_guard(panel_x: f64, panel_y: f64, panel_w: f64, panel_h: f64, anchor_x: f64) {
    if let Ok(mut g) = TRAVEL_GUARD.lock() {
        *g = Some(TravelGuard {
            panel_x,
            panel_y,
            panel_w,
            panel_h,
            anchor_x,
            anchor_y: 18.0,
            deadline: Instant::now() + Duration::from_millis(2500),
        });
    }
}

fn point_in_triangle(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> bool {
    let sign = |x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64| (x1 - x3) * (y2 - y3) - (x2 - x3) * (y1 - y3);
    let d1 = sign(px, py, ax, ay, bx, by);
    let d2 = sign(px, py, bx, by, cx, cy);
    let d3 = sign(px, py, cx, cy, ax, ay);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

pub fn travel_guard_holds(cursor_x: f64, cursor_y: f64) -> bool {
    let guard = TRAVEL_GUARD.lock().ok().and_then(|g| *g);
    let Some(g) = guard else { return false };
    if Instant::now() > g.deadline {
        if let Ok(mut s) = TRAVEL_GUARD.lock() {
            *s = None;
        }
        return false;
    }
    let pad = 10.0;
    if cursor_x >= g.panel_x - pad
        && cursor_x <= g.panel_x + g.panel_w + pad
        && cursor_y >= g.panel_y - pad
        && cursor_y <= g.panel_y + g.panel_h + pad
    {
        return true;
    }
    point_in_triangle(
        cursor_x,
        cursor_y,
        g.anchor_x,
        g.anchor_y,
        g.panel_x - pad,
        g.panel_y,
        g.panel_x + g.panel_w + pad,
        g.panel_y,
    )
}

pub fn spawn_panel_focus_watcher(
    keep_classes: &'static [&'static str],
    own_title: &'static str,
    dismiss: State<bool>,
) {
    spawn_panel_focus_watcher_guarded(keep_classes, own_title, dismiss, || true);
}

pub fn spawn_panel_focus_watcher_guarded(
    keep_classes: &'static [&'static str],
    own_title: &'static str,
    dismiss: State<bool>,
    is_current: impl Fn() -> bool + Send + 'static,
) {
    let mut dismiss = dismiss;
    spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if !is_current() {
                break;
            }
            let should_close = tokio::task::spawn_blocking(move || {
                let out = std::process::Command::new("hyprctl").args(["activewindow", "-j"]).output();
                if let Ok(o) = out {
                    let s = String::from_utf8_lossy(&o.stdout).to_string();
                    if s.trim().is_empty() || s.contains("\"class\": \"\"") {
                        return false;
                    }
                    for c in keep_classes {
                        if s.contains(&format!("\"class\": \"{c}\"")) {
                            return false;
                        }
                    }
                    if s.contains(&format!("\"title\": \"{own_title}\"")) {
                        return false;
                    }
                    return true;
                }
                false
            })
            .await
            .unwrap_or(false);
            if should_close {
                let travelling =
                    tokio::task::spawn_blocking(cursor_in_travel_zone).await.unwrap_or(false);
                if travelling {
                    continue;
                }
                dismiss.set(true);
                break;
            }
        }
    });
}

pub fn cursor_pos_blocking() -> Option<(f64, f64)> {
    let out = std::process::Command::new("hyprctl").args(["cursorpos"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let (x, y) = s.trim().split_once(',')?;
    Some((x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?))
}

pub fn cursor_in_travel_zone() -> bool {
    let Some((x, y)) = cursor_pos_blocking() else { return false };
    travel_guard_holds(x, y)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Returns true when enough time passed since the last accepted toggle.
/// First call per store always passes. Used to swallow double-clicks that
/// otherwise open a panel twice (leaking one window) or open-then-instantly-close it.
pub fn rate_limit_ok(last: &AtomicU64, window_ms: u64) -> bool {
    let now = now_ms();
    let prev = last.load(Ordering::SeqCst);
    if prev == 0 || now.saturating_sub(prev) >= window_ms {
        last.store(now, Ordering::SeqCst);
        return true;
    }
    false
}

pub static LAST_PANEL_TOGGLE_MS: AtomicU64 = AtomicU64::new(0);
pub static LAST_NOTIF_TOGGLE_MS: AtomicU64 = AtomicU64::new(0);
pub static LAST_CALENDAR_TOGGLE_MS: AtomicU64 = AtomicU64::new(0);
pub static LAST_CLIPBOARD_TOGGLE_MS: AtomicU64 = AtomicU64::new(0);
pub static LAST_SCREENSHOT_TOGGLE_MS: AtomicU64 = AtomicU64::new(0);
static LAST_CLIPBOARD_OPEN_MS: AtomicU64 = AtomicU64::new(0);
static LAST_SCREENSHOT_OPEN_MS: AtomicU64 = AtomicU64::new(0);

pub fn note_clipboard_opened() {
    LAST_CLIPBOARD_OPEN_MS.store(now_ms(), Ordering::SeqCst);
}

pub fn clipboard_opened_recently() -> bool {
    now_ms().saturating_sub(LAST_CLIPBOARD_OPEN_MS.load(Ordering::SeqCst)) < 600
}

pub fn note_screenshot_opened() {
    LAST_SCREENSHOT_OPEN_MS.store(now_ms(), Ordering::SeqCst);
}

pub fn screenshot_opened_recently() -> bool {
    now_ms().saturating_sub(LAST_SCREENSHOT_OPEN_MS.load(Ordering::SeqCst)) < 600
}

/// Animated-close requests. Toggle buttons live in the topbar window while
/// the panel's dismiss state lives in the panel window, so a toggle cannot
/// set dismiss directly. Instant close_window from another window pops.
/// Instead the toggle sets a flag and the panel's watcher maps it to its
/// local dismiss, preserving the PanelShell exit animation.
pub static CLOSE_PANEL: AtomicBool = AtomicBool::new(false);
pub static CLOSE_NOTIFICATIONS: AtomicBool = AtomicBool::new(false);
pub static CLOSE_CALENDAR: AtomicBool = AtomicBool::new(false);
pub static CLOSE_CLIPBOARD: AtomicBool = AtomicBool::new(false);
pub static CLOSE_SCREENSHOT: AtomicBool = AtomicBool::new(false);

pub fn clear_close_flag(flag: &'static AtomicBool) {
    flag.store(false, Ordering::SeqCst);
}

pub fn spawn_close_flag_watcher_guarded(
    flag: &'static AtomicBool,
    dismiss: State<bool>,
    is_current: impl Fn() -> bool + Send + 'static,
) {
    let mut dismiss = dismiss;
    spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(80)).await;
            if !is_current() {
                break;
            }
            if flag.swap(false, Ordering::SeqCst) {
                dismiss.set(true);
                break;
            }
        }
    });
}

pub fn is_panel_window_current(my_wid: winit::window::WindowId) -> bool {
    GlobalContexts::get()
        .try_get_context::<crate::PanelWindowId>()
        .and_then(|c| c.0.lock().ok().and_then(|g| *g))
        .is_some_and(|id| id == my_wid)
}

pub fn is_notifications_window_current(my_wid: winit::window::WindowId) -> bool {
    GlobalContexts::get()
        .try_get_context::<crate::NotificationPanelWindowId>()
        .and_then(|c| c.0.lock().ok().and_then(|g| *g))
        .is_some_and(|id| id == my_wid)
}

pub fn is_calendar_window_current(my_wid: winit::window::WindowId) -> bool {
    GlobalContexts::get()
        .try_get_context::<crate::CalendarPanelWindowId>()
        .and_then(|c| c.0.lock().ok().and_then(|g| *g))
        .is_some_and(|id| id == my_wid)
}

pub fn is_clipboard_window_current(my_wid: winit::window::WindowId) -> bool {
    GlobalContexts::get()
        .try_get_context::<crate::ClipboardWindowId>()
        .and_then(|c| c.0.lock().ok().and_then(|g| *g))
        .is_some_and(|id| id == my_wid)
}

pub fn is_screenshot_window_current(my_wid: winit::window::WindowId) -> bool {
    GlobalContexts::get()
        .try_get_context::<crate::ScreenshotWindowId>()
        .and_then(|c| c.0.lock().ok().and_then(|g| *g))
        .is_some_and(|id| id == my_wid)
}

pub fn hypr_has_any_class_blocking(classes: &[&str]) -> bool {
    let out = match std::process::Command::new("hyprctl").args(["clients", "-j"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return true,
    };
    let raw = String::from_utf8_lossy(&out.stdout);
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return true;
    };
    let Some(arr) = v.as_array() else {
        return true;
    };
    for c in arr {
        if let Some(cls) = c.get("class").and_then(|x| x.as_str()) {
            if classes.iter().any(|w| *w == cls) {
                return true;
            }
        }
    }
    false
}

fn spawn_stale_close_watchdog(
    flag: &'static AtomicBool,
    hypr_class: &'static str,
    take_stale: impl FnOnce() -> Option<winit::window::WindowId> + Send + 'static,
) {
    spawn(async move {
        tokio::time::sleep(Duration::from_millis(900)).await;
        if flag.swap(false, Ordering::SeqCst) {
            if let Some(id) = take_stale() {
                eprintln!("[overlay] close request unacked, clearing stale window");
                Platform::get().close_window(id);
            }
            let _ = std::process::Command::new("hyprctl")
                .args(["eval", &format!("hl.dispatch(hl.dsp.window.close({{ window = 'class:^({hypr_class})$' }}))")])
                .output();
        }
    });
}

pub fn request_panel_close() {
    CLOSE_PANEL.store(true, Ordering::SeqCst);
    spawn_stale_close_watchdog(&CLOSE_PANEL, "overlay-panel", || {
        GlobalContexts::get()
            .try_get_context::<crate::PanelWindowId>()
            .and_then(|c| c.0.lock().ok().and_then(|mut g| g.take()))
    });
}

pub fn request_notifications_close() {
    CLOSE_NOTIFICATIONS.store(true, Ordering::SeqCst);
    spawn_stale_close_watchdog(&CLOSE_NOTIFICATIONS, "overlay-notifications", || {
        GlobalContexts::get()
            .try_get_context::<crate::NotificationPanelWindowId>()
            .and_then(|c| c.0.lock().ok().and_then(|mut g| g.take()))
    });
}

pub fn request_calendar_close() {
    CLOSE_CALENDAR.store(true, Ordering::SeqCst);
    spawn_stale_close_watchdog(&CLOSE_CALENDAR, "overlay-calendar", || {
        GlobalContexts::get()
            .try_get_context::<crate::CalendarPanelWindowId>()
            .and_then(|c| c.0.lock().ok().and_then(|mut g| g.take()))
    });
}

pub fn request_clipboard_close() {
    CLOSE_CLIPBOARD.store(true, Ordering::SeqCst);
    spawn_stale_close_watchdog(&CLOSE_CLIPBOARD, "overlay-clipboard", || {
        GlobalContexts::get()
            .try_get_context::<crate::ClipboardWindowId>()
            .and_then(|c| c.0.lock().ok().and_then(|mut g| g.take()))
    });
}

pub fn request_screenshot_close() {
    CLOSE_SCREENSHOT.store(true, Ordering::SeqCst);
    spawn_stale_close_watchdog(&CLOSE_SCREENSHOT, "overlay-screenshot", || {
        GlobalContexts::get()
            .try_get_context::<crate::ScreenshotWindowId>()
            .and_then(|c| c.0.lock().ok().and_then(|mut g| g.take()))
    });
}

// --- Issue #20: notification center, DND schedule, OSD live, media ---

fn overlay_data_file(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|c| c.join("finick").join(name))
}

#[derive(Clone, Debug, PartialEq)]
pub struct DndSchedule {
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

impl Default for DndSchedule {
    fn default() -> Self {
        Self { enabled: false, start: "22:00".to_string(), end: "07:00".to_string() }
    }
}

pub fn parse_hm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().split_whitespace().next()?.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

pub fn dnd_schedule_active(sched: &DndSchedule, now_hm: &str) -> bool {
    if !sched.enabled {
        return false;
    }
    let (sh, sm) = match parse_hm(&sched.start) {
        Some(v) => v,
        None => return false,
    };
    let (eh, em) = match parse_hm(&sched.end) {
        Some(v) => v,
        None => return false,
    };
    let (nh, nm) = match parse_hm(now_hm) {
        Some(v) => v,
        None => return false,
    };
    let to_min = |h: u32, m: u32| h * 60 + m;
    let s = to_min(sh, sm);
    let e = to_min(eh, em);
    let n = to_min(nh, nm);
    if s == e {
        return true;
    }
    if s < e {
        n >= s && n < e
    } else {
        n >= s || n < e
    }
}

pub fn now_hm_local() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

pub fn load_dnd_schedule() -> DndSchedule {
    let mut s = DndSchedule::default();
    let Some(f) = overlay_data_file("dnd_schedule.json") else {
        return s;
    };
    let Ok(raw) = std::fs::read_to_string(f) else {
        return s;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return s;
    };
    if let Some(b) = v.get("enabled").and_then(|x| x.as_bool()) {
        s.enabled = b;
    }
    if let Some(x) = v.get("start").and_then(|x| x.as_str()) {
        if parse_hm(x).is_some() {
            s.start = x.to_string();
        }
    }
    if let Some(x) = v.get("end").and_then(|x| x.as_str()) {
        if parse_hm(x).is_some() {
            s.end = x.to_string();
        }
    }
    s
}

pub fn save_dnd_schedule(sched: &DndSchedule) {
    let Some(f) = overlay_data_file("dnd_schedule.json") else {
        return;
    };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let v = serde_json::json!({ "enabled": sched.enabled, "start": sched.start, "end": sched.end });
    let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default());
}

fn dnd_manual_on_blocking() -> bool {
    let entries = ipsea::settings::get_all_settings(socket()).unwrap_or_default();
    for e in &entries {
        if e.key == ipsea::settings::SettingKey::DoNotDisturb {
            return e.value.as_str().map(|s| s != "off").unwrap_or(false)
                || e.value.as_bool().unwrap_or(false);
        }
    }
    false
}

/// True when toasts must stay silent: manual DND is on or the persisted
/// DND schedule window currently covers local time.
pub fn dnd_silenced_blocking() -> bool {
    if dnd_manual_on_blocking() {
        return true;
    }
    let sched = load_dnd_schedule();
    dnd_schedule_active(&sched, &now_hm_local())
}

pub fn load_muted_apps() -> std::collections::HashSet<String> {
    let Some(f) = overlay_data_file("notif_muted_apps.json") else {
        return std::collections::HashSet::new();
    };
    let Ok(raw) = std::fs::read_to_string(f) else {
        return std::collections::HashSet::new();
    };
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default().into_iter().collect()
}

pub fn save_muted_apps(muted: &std::collections::HashSet<String>) {
    let Some(f) = overlay_data_file("notif_muted_apps.json") else {
        return;
    };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut v: Vec<String> = muted.iter().cloned().collect();
    v.sort();
    let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default());
}

pub fn is_app_muted(muted: &std::collections::HashSet<String>, app: &str) -> bool {
    muted.contains(app)
}

pub fn set_app_muted(muted: &mut std::collections::HashSet<String>, app: &str, mute: bool) {
    if mute {
        muted.insert(app.to_string());
    } else {
        muted.remove(app);
    }
    save_muted_apps(muted);
}

/// Groups notifications by app name for the center. Groups are sorted by
/// app name so the list is stable; items keep newest-first order.
pub fn group_notifications(notifs: &[Notification]) -> Vec<(String, Vec<Notification>)> {
    let mut map: std::collections::BTreeMap<String, Vec<Notification>> = std::collections::BTreeMap::new();
    for n in notifs.iter().rev() {
        let app = if n.app_name.trim().is_empty() {
            "Other".to_string()
        } else {
            n.app_name.clone()
        };
        map.entry(app).or_default().push(n.clone());
    }
    map.into_iter().collect()
}

/// Live sink volume in percent, or None when wpctl is unavailable.
pub fn query_sink_volume_pct() -> Option<f64> {
    let out = std::process::Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let v: f64 = s.split_whitespace().nth(1)?.parse().ok()?;
    Some((v * 100.0).clamp(0.0, 150.0))
}

pub fn query_sink_muted() -> Option<bool> {
    let out = std::process::Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).contains("[MUTED]"))
}

/// Live display brightness in percent, or None when no backlight exists.
pub fn query_brightness_pct() -> Option<f64> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/backlight") {
        let mut best: Option<f64> = None;
        for e in entries.flatten() {
            let base = e.path();
            let cur: Option<u64> =
                std::fs::read_to_string(base.join("brightness")).ok().and_then(|s| s.trim().parse().ok());
            let max: Option<u64> =
                std::fs::read_to_string(base.join("max_brightness")).ok().and_then(|s| s.trim().parse().ok());
            if let (Some(c), Some(m)) = (cur, max) {
                if m > 0 {
                    let pct = (c as f64 / m as f64 * 100.0).clamp(0.0, 100.0);
                    best = Some(best.map_or(pct, |b: f64| b.max(pct)));
                }
            }
        }
        if best.is_some() {
            return best;
        }
    }
    let out = std::process::Command::new("brightnessctl").arg("get").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let cur: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    let max_out = std::process::Command::new("brightnessctl").arg("max").output().ok()?;
    let max: f64 = String::from_utf8_lossy(&max_out.stdout).trim().parse().ok()?;
    if max > 0.0 {
        Some((cur / max * 100.0).clamp(0.0, 100.0))
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaStatus {
    pub available: bool,
    pub player: String,
    pub title: String,
    pub artist: String,
    pub status: String,
}

impl MediaStatus {
    pub fn unavailable() -> Self {
        Self { available: false, player: String::new(), title: String::new(), artist: String::new(), status: String::new() }
    }
}

pub fn playerctl_present() -> bool {
    std::process::Command::new("playerctl").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn playerctl_arg(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("playerctl").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn query_media_status() -> MediaStatus {
    if !playerctl_present() {
        return MediaStatus::unavailable();
    }
    let player = playerctl_arg(&["metadata", "--format", "{{playerName}}"]).unwrap_or_default();
    let title = playerctl_arg(&["metadata", "--format", "{{title}}"]).unwrap_or_default();
    let artist = playerctl_arg(&["metadata", "--format", "{{artist}}"]).unwrap_or_default();
    let status = playerctl_arg(&["status"]).unwrap_or_default();
    if title.is_empty() && artist.is_empty() && status.is_empty() {
        return MediaStatus { available: true, player, title: String::new(), artist: String::new(), status: "Stopped".to_string() };
    }
    MediaStatus { available: true, player, title, artist, status }
}

pub fn media_control(action: &str) -> bool {
    if !playerctl_present() {
        return false;
    }
    let arg = match action {
        "play-pause" => "play-pause",
        "next" => "next",
        "previous" => "previous",
        _ => return false,
    };
    std::process::Command::new("playerctl").arg(arg).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn load_reduced_motion_blocking() -> bool {
    let entries = ipsea::settings::get_all_settings(socket()).unwrap_or_default();
    for e in &entries {
        if e.key == ipsea::settings::SettingKey::AccessibilityReduceMotion {
            return e.value.as_bool().unwrap_or(false);
        }
    }
    false
}

pub fn subscribe_reduced_motion(reduced: State<bool>) {
    spawn(async move {
        loop {
            let rx = match ipsea::settings::subscribe_channel(socket(), SubscriptionFilter::all()) {
                Ok(rx) => rx,
                Err(e) => {
                    eprintln!("reduced-motion subscribe failed: {e}; retrying");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            let mut rx = rx;
            while let Some(evt) = rx.recv().await {
                if let ipsea::settings::SettingsEvent::Changed { key, value, .. } = evt {
                    if key == ipsea::settings::SettingKey::AccessibilityReduceMotion {
                        if let Some(b) = value.as_bool() {
                            let mut s = reduced;
                            s.set_if_modified(b);
                        }
                    }
                }
            }
            eprintln!("reduced-motion subscribe disconnected; retrying");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

#[cfg(test)]
mod issue20_tests {
    use super::*;

    #[test]
    fn test_parse_hm_valid_and_invalid() {
        assert_eq!(parse_hm("22:00"), Some((22, 0)));
        assert_eq!(parse_hm("07:30"), Some((7, 30)));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(parse_hm("22:60"), None);
        assert_eq!(parse_hm("nope"), None);
    }

    #[test]
    fn test_dnd_schedule_overnight_window() {
        let sched = DndSchedule { enabled: true, start: "22:00".to_string(), end: "07:00".to_string() };
        assert!(dnd_schedule_active(&sched, "22:00"));
        assert!(dnd_schedule_active(&sched, "23:59"));
        assert!(dnd_schedule_active(&sched, "03:00"));
        assert!(!dnd_schedule_active(&sched, "07:00"));
        assert!(!dnd_schedule_active(&sched, "12:00"));
        assert!(!dnd_schedule_active(&sched, "21:59"));
    }

    #[test]
    fn test_dnd_schedule_daytime_window_and_disabled() {
        let day = DndSchedule { enabled: true, start: "09:00".to_string(), end: "17:00".to_string() };
        assert!(dnd_schedule_active(&day, "12:00"));
        assert!(!dnd_schedule_active(&day, "08:59"));
        assert!(!dnd_schedule_active(&day, "17:00"));
        let off = DndSchedule { enabled: false, start: "22:00".to_string(), end: "07:00".to_string() };
        assert!(!dnd_schedule_active(&off, "23:00"));
    }

    #[test]
    fn test_group_notifications_by_app_newest_first() {
        let mk = |id: u32, app: &str| Notification {
            id,
            app_name: app.to_string(),
            summary: format!("s{id}"),
            body: String::new(),
            icon: String::new(),
            timeout: 5000,
        };
        let notifs = vec![mk(1, "Mail"), mk(2, "Chat"), mk(3, "Mail")];
        let groups = group_notifications(&notifs);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "Chat");
        assert_eq!(groups[1].0, "Mail");
        assert_eq!(groups[1].1.iter().map(|n| n.id).collect::<Vec<_>>(), vec![3, 1]);
    }

    #[test]
    fn test_group_notifications_empty_app_becomes_other() {
        let n = Notification {
            id: 1,
            app_name: String::new(),
            summary: "s".to_string(),
            body: String::new(),
            icon: String::new(),
            timeout: 5000,
        };
        let groups = group_notifications(&[n]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "Other");
    }
}
