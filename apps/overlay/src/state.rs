use {
    freya::prelude::*,
    ipsea::settings::{
        SETTINGS_SOCKET_NAME, SettingEntry, SettingKey, SettingValue, SettingsEvent, SubscriptionFilter,
        get_all_settings, set_and_apply,
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
        _ => {}
    }
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
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<Vec<SettingEntry>>>();
    std::thread::spawn(move || match get_all_settings(socket()) {
        Ok(entries) => {
            let _ = tx.send(Some(entries));
        }
        Err(e) => {
            eprintln!("topbar get_all_settings offline: {e}");
            let _ = tx.send(None);
        }
    });
    spawn(async move {
        if let Some(opt) = rx.recv().await {
            match opt {
                Some(entries) => {
                    for entry in &entries {
                        apply_entry(entry, wifi, bt, volume, muted, brightness, dnd);
                        if let SettingKey::Custom(ref k) = entry.key {
                            let mut needs = false;
                            let mut ts = topbar_settings.read().clone();
                            match k.as_str() {
                                "topbar.show_wifi" => if let Some(b) = entry.value.as_bool() { ts.show_wifi = b; needs = true; },
                                "topbar.show_bluetooth" => if let Some(b) = entry.value.as_bool() { ts.show_bluetooth = b; needs = true; },
                                "topbar.show_sound" => if let Some(b) = entry.value.as_bool() { ts.show_sound = b; needs = true; },
                                "topbar.show_battery" => if let Some(b) = entry.value.as_bool() { ts.show_battery = b; needs = true; },
                                "topbar.text_size" => if let Some(f) = entry.value.as_f64() { ts.text_size = f; needs = true; },
                                "topbar.text_color" => if let Some(s) = entry.value.as_str() { ts.text_color = s.to_string(); needs = true; },
                                "topbar.theme" => if let Some(s) = entry.value.as_str() { ts.theme = s.to_string(); needs = true; },
                                "topbar.icon_size" => if let Some(f) = entry.value.as_f64() { ts.icon_size = f; needs = true; } else if let Some(i) = entry.value.as_i64() { ts.icon_size = i as f64; needs = true; },
                                "topbar.icon_stroke" => if let Some(f) = entry.value.as_f64() { ts.icon_stroke = f; needs = true; } else if let Some(i) = entry.value.as_i64() { ts.icon_stroke = i as f64; needs = true; },
                                _ => {}
                            }
                            if needs { topbar_settings.set(ts); }
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
                }
                None => {
                    let mut c = connected;
                    c.set_if_modified(false);
                }
            }
        }
    });
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopbarSettings {
    pub show_wifi: bool,
    pub show_bluetooth: bool,
    pub show_sound: bool,
    pub show_battery: bool,
    pub text_size: f64,
    pub text_color: String,
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
            text_size: 13.0,
            text_color: "Default".to_string(),
            theme: "system".to_string(),
            icon_size: 14.0,
            icon_stroke: 1.6,
        }
    }
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
    let rx = match ipsea::settings::subscribe_channel(socket(), SubscriptionFilter::all()) {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("topbar subscribe failed: {e}");
            return;
        }
    };
    let mut rx = rx;
    spawn(async move {
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
                        let mut changed = false;
                        match k.as_str() {
                            "topbar.show_wifi" => if let Some(b) = value.as_bool() { ts.show_wifi = b; changed = true; },
                            "topbar.show_bluetooth" => if let Some(b) = value.as_bool() { ts.show_bluetooth = b; changed = true; },
                            "topbar.show_sound" => if let Some(b) = value.as_bool() { ts.show_sound = b; changed = true; },
                            "topbar.show_battery" => if let Some(b) = value.as_bool() { ts.show_battery = b; changed = true; },
                            "topbar.text_size" => if let Some(f) = value.as_f64() { ts.text_size = f; changed = true; },
                            "topbar.text_color" => if let Some(s) = value.as_str() { ts.text_color = s.to_string(); changed = true; },
                            "topbar.theme" => if let Some(s) = value.as_str() { ts.theme = s.to_string(); changed = true; },
                            "topbar.icon_size" => if let Some(f) = value.as_f64() { ts.icon_size = f; changed = true; } else if let Some(i) = value.as_i64() { ts.icon_size = i as f64; changed = true; },
                            "topbar.icon_stroke" => if let Some(f) = value.as_f64() { ts.icon_stroke = f; changed = true; } else if let Some(i) = value.as_i64() { ts.icon_stroke = i as f64; changed = true; },
                            _ => {}
                        }
                        if changed { topbar_settings.set(ts); }
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

pub fn clock_now() -> String {
    clock_now_with_format(true)
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
