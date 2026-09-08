use {
    freya::prelude::*,
    ipsea::settings::{
        SETTINGS_SOCKET_NAME, SettingEntry, SettingKey, SettingValue, SettingsEvent, SubscriptionFilter,
        get_all_settings, set_and_apply,
    },
    serde::{Deserialize, Serialize},
    system::{HyprlandBackend, SystemBackend},
};

pub fn socket() -> String {
    SETTINGS_SOCKET_NAME.to_string()
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub level: NotificationLevel,
    pub timestamp: u64,
    pub app_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NotifRequest {
    Push { title: String, body: String, level: NotificationLevel, app_name: Option<String> },
    List,
    Dismiss { id: String },
    Clear,
    Subscribe,
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

pub fn subscribe_live(
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
    connected: State<bool>,
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
                    SettingKey::PowerProfile => {}
                    _ => {}
                },
                SettingsEvent::LockChanged { .. } => {}
                SettingsEvent::Reloaded => {
                    load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected);
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

pub fn clock_now() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

pub fn power_action(action: &'static str) {
    std::thread::spawn(move || {
        let ok = match action {
            "logout" => HyprlandBackend.log_out(),
            "reboot" => HyprlandBackend.reboot(),
            _ => HyprlandBackend.power_off(),
        };
        if !ok {
            eprintln!("topbar power action {action} failed");
        }
    });
}

pub fn fetch_notifications_blocking() -> Vec<Notification> {
    let shared = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Notification>::new()));
    let shared2 = shared.clone();
    let _ = ipsea::send_command(
        "notifications".to_string(),
        &NotifRequest::List,
        Some(move |n: Notification| {
            if let Ok(mut g) = shared2.lock() {
                g.push(n);
            }
        }),
    );
    shared.lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn clear_notifications() {
    std::thread::spawn(|| {
        let _: std::io::Result<()> = ipsea::send_command::<NotifRequest, Notification, fn(Notification)>(
            "notifications".to_string(),
            &NotifRequest::Clear,
            None,
        );
    });
}
