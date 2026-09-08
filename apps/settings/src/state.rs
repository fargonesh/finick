use std::collections::HashMap;
use freya::prelude::*;
use ipsea::settings::{
    get_all_settings, set_and_apply, subscribe_channel, LockState,
    SettingEntry, SettingKey, SettingValue, SubscriptionFilter, SETTINGS_SOCKET_NAME,
};
use system::{HyprlandBackend, SystemBackend};
use ui::ResolutionChoice;

#[derive(Clone, Copy, PartialEq)]
pub struct SettingsStore {
    // Connectivity
    pub wifi_power: State<bool>,
    pub ask_to_join: State<bool>,
    pub limit_tracking: State<bool>,
    pub bt_power: State<bool>,
    pub bt_discoverable: State<bool>,

    // Sound
    pub volume: State<f64>,
    pub is_mute: State<bool>,
    pub feedback_on_change: State<bool>,

    // Display
    pub brightness: State<f64>,
    pub auto_brightness: State<bool>,
    pub true_tone: State<bool>,
    pub res_choice: State<ResolutionChoice>,
    pub night_shift: State<bool>,
    pub night_shift_mode: State<usize>,
    pub color_temp: State<f64>,

    // Appearance & Personalization
    pub wallpaper: State<String>,
    pub wallpaper_idx: State<usize>,
    pub wallpaper_folder: State<String>,
    pub wallpaper_interval: State<usize>,
    pub scrollbar_pref: State<usize>,
    pub icon_size_pref: State<usize>,
    pub focus_mode: State<&'static str>,

    // Desktop & Dock
    pub dock_position: State<usize>,
    pub dock_size: State<f64>,
    pub dock_autohide: State<bool>,
    pub window_layout: State<usize>,
    pub workspace_gap: State<f64>,

    // System & General
    pub time_24h: State<bool>,
    pub auto_updates: State<bool>,
    pub ntp_sync: State<bool>,
    pub battery_pct: State<u8>,
    pub battery_mode: State<&'static str>,
    pub empty_trash_auto: State<bool>,
    pub save_cloud: State<bool>,
    pub opt_charging: State<bool>,

    // Notifications
    pub notif_messages: State<bool>,
    pub notif_calendar: State<bool>,
    pub notif_mail: State<bool>,
    pub notif_photos: State<bool>,
    pub notif_weather: State<bool>,
    pub allow_notif: State<bool>,
    pub notif_style: State<usize>,
    pub silence_sleep: State<bool>,

    // Screen Time
    pub screen_time_enabled: State<bool>,
    pub downtime_enabled: State<bool>,
    pub app_limits_enabled: State<bool>,
    pub downtime_from: State<String>,
    pub downtime_to: State<String>,

    // Accessibility
    pub text_size: State<f64>,
    pub reduce_motion: State<bool>,
    pub increase_contrast: State<bool>,
    pub reduce_transparency: State<bool>,
    pub screen_reader: State<bool>,

    // Privacy & Permissions
    pub camera_access: State<bool>,
    pub mic_access: State<bool>,
    pub location_access: State<bool>,
    pub firewall_enabled: State<bool>,

    // Lock states from settings-daemon (e.g. NixOS-managed)
    pub lock_states: State<HashMap<SettingKey, LockState>>,
}

impl SettingsStore {
    /// Returns whether the given setting is locked (read-only).
    pub fn is_locked(&self, key: &SettingKey) -> bool {
        self.lock_states.read().get(key).map(|l| l.is_locked()).unwrap_or(false)
    }

    /// Returns whether the given setting is managed/locked specifically by NixOS.
    pub fn is_nix_locked(&self, key: &SettingKey) -> bool {
        self.lock_states.read().get(key).map(|l| l.is_nix_locked()).unwrap_or(false)
    }

    /// Returns the raw LockState for a key.
    pub fn lock_state(&self, key: &SettingKey) -> LockState {
        self.lock_states.read().get(key).cloned().unwrap_or(LockState::Unlocked)
    }

    /// Returns a human-friendly lock label if the key is restricted, e.g. "Managed by NixOS".
    pub fn lock_label(&self, key: &SettingKey) -> Option<String> {
        let locks = self.lock_states.read();
        locks.get(key).and_then(|l| {
            if l.is_locked() {
                if l.is_nix_locked() {
                    Some("Managed by NixOS".to_string())
                } else if let Some(r) = l.reason() {
                    Some(r.to_string())
                } else {
                    Some("Locked by policy".to_string())
                }
            } else {
                None
            }
        })
    }

    /// Mutates and applies a setting via the daemon IPC.
    /// If the setting is locked, the change is rejected and ignored.
    pub fn set(&self, key: SettingKey, val: impl Into<SettingValue>) {
        let val = val.into();
        if self.is_locked(&key) {
            return;
        }

        // Optimistically apply to local reactive state
        self.apply_to_local_state(&key, &val);

        let key_clone = key.clone();
        let val_clone = val.clone();

        std::thread::spawn(move || {
            match set_and_apply(SETTINGS_SOCKET_NAME, key_clone.clone(), val_clone) {
                Ok(Ok(())) => {}
                Ok(Err(denial)) => {
                    eprintln!("[SettingsStore] Mutation denied for {:?}: {:?}", key_clone, denial);
                }
                Err(e) => {
                    eprintln!("[SettingsStore] IPC error setting {:?}: {}", key_clone, e);
                }
            }
        });
    }

    /// Updates the local Freya reactive state for a key without sending IPC requests.
    pub fn apply_to_local_state(&self, key: &SettingKey, val: &SettingValue) {
        match key {
            SettingKey::WifiEnabled => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.wifi_power;
                    s.set(b);
                }
            }
            SettingKey::WifiNetwork => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.ask_to_join;
                    s.set(b);
                }
            }
            SettingKey::BluetoothEnabled => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.bt_power;
                    s.set(b);
                }
            }
            SettingKey::AudioVolume => {
                if let Some(i) = val.as_i64() {
                    let mut s = self.volume;
                    s.set(i as f64);
                }
            }
            SettingKey::AudioMuted => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.is_mute;
                    s.set(b);
                }
            }
            SettingKey::DisplayBrightness => {
                if let Some(f) = val.as_f64() {
                    let mut s = self.brightness;
                    s.set(f);
                }
            }
            SettingKey::DisplayAutoBrightness => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.auto_brightness;
                    s.set(b);
                }
            }
            SettingKey::DisplayNightShift => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.night_shift;
                    s.set(b);
                }
            }
            SettingKey::DisplayColorTemp => {
                if let Some(f) = val.as_f64() {
                    let mut s = self.color_temp;
                    s.set(f);
                }
            }
            SettingKey::DisplayResolution => {
                if let Some(s) = val.as_str() {
                    let mut choice = self.res_choice;
                    match s {
                        "MoreSpace" | "more_space" => choice.set(ResolutionChoice::MoreSpace),
                        "LargerText" | "larger_text" => choice.set(ResolutionChoice::LargerText),
                        _ => choice.set(ResolutionChoice::Default),
                    }
                }
            }
            SettingKey::TimeFormat24h => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.time_24h;
                    s.set(b);
                }
            }
            SettingKey::NtpEnabled => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.ntp_sync;
                    s.set(b);
                }
            }
            SettingKey::SystemAutoUpdates => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.auto_updates;
                    s.set(b);
                }
            }
            SettingKey::Wallpaper => {
                if let Some(i) = val.as_i64() {
                    let mut s = self.wallpaper_idx;
                    s.set(i as usize);
                    let mut wp = self.wallpaper;
                    wp.set(format!("preset:{i}"));
                } else if let Some(str_val) = val.as_str() {
                    let mut wp = self.wallpaper;
                    wp.set(str_val.to_string());
                    if let Some(idx) = str_val.strip_prefix("preset:").and_then(|p| p.parse::<usize>().ok()) {
                        let mut s = self.wallpaper_idx;
                        s.set(idx);
                    }
                }
            }
            SettingKey::Scrollbars => {
                if let Some(i) = val.as_i64() {
                    let mut s = self.scrollbar_pref;
                    s.set(i as usize);
                }
            }
            SettingKey::IconSize => {
                if let Some(i) = val.as_i64() {
                    let mut s = self.icon_size_pref;
                    s.set(i as usize);
                }
            }
            SettingKey::DoNotDisturb => {
                if let Some(s) = val.as_str() {
                    let mut fm = self.focus_mode;
                    match s {
                        "work" => fm.set("work"),
                        "personal" => fm.set("personal"),
                        "sleep" => fm.set("sleep"),
                        _ => fm.set("off"),
                    }
                }
            }
            SettingKey::PowerProfile => {
                if let Some(s) = val.as_str() {
                    let mut bm = self.battery_mode;
                    match s {
                        "performance" => bm.set("performance"),
                        "power-saver" | "powersave" => bm.set("powersave"),
                        _ => bm.set("balanced"),
                    }
                }
            }
            SettingKey::StorageEmptyTrashAuto => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.empty_trash_auto;
                    s.set(b);
                }
            }
            SettingKey::NotificationBanners => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.allow_notif;
                    s.set(b);
                }
            }
            SettingKey::NotificationSounds => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.silence_sleep;
                    s.set(b);
                }
            }
            SettingKey::ScreenTimeEnabled => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.screen_time_enabled;
                    s.set(b);
                    let mut s2 = self.downtime_enabled;
                    s2.set(b);
                }
            }
            SettingKey::AccessibilityReduceMotion => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.reduce_motion;
                    s.set(b);
                }
            }
            SettingKey::AccessibilityIncreaseContrast => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.increase_contrast;
                    s.set(b);
                }
            }
            SettingKey::AccessibilityReduceTransparency => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.reduce_transparency;
                    s.set(b);
                }
            }
            SettingKey::AccessibilityScreenReader => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.screen_reader;
                    s.set(b);
                }
            }
            SettingKey::AccessibilityTextSize => {
                if let Some(f) = val.as_f64() {
                    let mut s = self.text_size;
                    s.set(f);
                }
            }
            SettingKey::CameraAccess => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.camera_access;
                    s.set(b);
                }
            }
            SettingKey::MicAccess => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.mic_access;
                    s.set(b);
                }
            }
            SettingKey::LocationAccess => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.location_access;
                    s.set(b);
                }
            }
            SettingKey::FirewallEnabled => {
                if let Some(b) = val.as_bool() {
                    let mut s = self.firewall_enabled;
                    s.set(b);
                }
            }
            _ => {}
        }
    }

    /// Applies a batch of setting entries and updates lock states.
    pub fn apply_entries(&self, entries: &[SettingEntry]) {
        let mut locks = self.lock_states;
        for entry in entries {
            locks.write().insert(entry.key.clone(), entry.lock_state.clone());
            self.apply_to_local_state(&entry.key, &entry.value);
        }
    }
}

/// Initializes settings state, connects to the settings-daemon via IPC,
/// loads all initial entries, subscribes to real-time events, and provides context.
pub fn use_init_settings_store() -> SettingsStore {
    let audio_info = HyprlandBackend.get_audio_info();
    let power_info = HyprlandBackend.get_power_info();
    let initial_battery = power_info
        .capacity
        .trim()
        .trim_end_matches('%')
        .parse::<u8>()
        .unwrap_or(78);

    let wifi_power = use_state(|| HyprlandBackend.get_wifi_status());
    let ask_to_join = use_state(|| true);
    let limit_tracking = use_state(|| true);
    let bt_power = use_state(|| HyprlandBackend.get_bluetooth_status());
    let bt_discoverable = use_state(|| true);

    let volume = use_state(move || audio_info.volume);
    let is_mute = use_state(move || audio_info.is_muted);
    let feedback_on_change = use_state(|| true);

    let brightness = use_state(|| 72.0);
    let auto_brightness = use_state(|| true);
    let true_tone = use_state(|| true);
    let res_choice = use_state(|| ResolutionChoice::Default);
    let night_shift = use_state(|| false);
    let night_shift_mode = use_state(|| 0usize);
    let color_temp = use_state(|| 30.0);

    let wallpaper = use_state(|| "#1e1e2e".to_string());
    let wallpaper_idx = use_state(|| 0usize);
    let wallpaper_folder = use_state(|| String::new());
    let wallpaper_interval = use_state(|| 0usize);
    let scrollbar_pref = use_state(|| 0usize);
    let icon_size_pref = use_state(|| 1usize);
    let focus_mode = use_state(|| "off");

    let dock_position = use_state(|| 0usize);
    let dock_size = use_state(|| 52.0);
    let dock_autohide = use_state(|| true);
    let window_layout = use_state(|| 0usize);
    let workspace_gap = use_state(|| 12.0);

    let time_24h = use_state(|| true);
    let auto_updates = use_state(|| true);
    let ntp_sync = use_state(|| true);
    let battery_pct = use_state(move || initial_battery);
    let battery_mode = use_state(|| "balanced");
    let empty_trash_auto = use_state(|| true);
    let save_cloud = use_state(|| false);
    let opt_charging = use_state(|| true);

    let notif_messages = use_state(|| true);
    let notif_calendar = use_state(|| true);
    let notif_mail = use_state(|| false);
    let notif_photos = use_state(|| true);
    let notif_weather = use_state(|| false);
    let allow_notif = use_state(|| true);
    let notif_style = use_state(|| 0usize);
    let silence_sleep = use_state(|| true);

    let screen_time_enabled = use_state(|| true);
    let downtime_enabled = use_state(|| false);
    let app_limits_enabled = use_state(|| true);
    let downtime_from = use_state(|| "22:00".to_string());
    let downtime_to = use_state(|| "07:00".to_string());

    let text_size = use_state(|| 40.0);
    let reduce_motion = use_state(|| false);
    let increase_contrast = use_state(|| false);
    let reduce_transparency = use_state(|| false);
    let screen_reader = use_state(|| false);

    let camera_access = use_state(|| true);
    let mic_access = use_state(|| true);
    let location_access = use_state(|| true);
    let firewall_enabled = use_state(|| true);

    let lock_states = use_state(HashMap::<SettingKey, LockState>::new);

    let store = SettingsStore {
        wifi_power,
        ask_to_join,
        limit_tracking,
        bt_power,
        bt_discoverable,
        volume,
        is_mute,
        feedback_on_change,
        brightness,
        auto_brightness,
        true_tone,
        res_choice,
        night_shift,
        night_shift_mode,
        color_temp,
        wallpaper,
        wallpaper_idx,
        wallpaper_folder,
        wallpaper_interval,
        scrollbar_pref,
        icon_size_pref,
        focus_mode,
        dock_position,
        dock_size,
        dock_autohide,
        window_layout,
        workspace_gap,
        time_24h,
        auto_updates,
        ntp_sync,
        battery_pct,
        battery_mode,
        empty_trash_auto,
        save_cloud,
        opt_charging,
        notif_messages,
        notif_calendar,
        notif_mail,
        notif_photos,
        notif_weather,
        allow_notif,
        notif_style,
        silence_sleep,
        screen_time_enabled,
        downtime_enabled,
        app_limits_enabled,
        downtime_from,
        downtime_to,
        text_size,
        reduce_motion,
        increase_contrast,
        reduce_transparency,
        screen_reader,
        camera_access,
        mic_access,
        location_access,
        firewall_enabled,
        lock_states,
    };

    provide_context(store);

    // Initial query and live subscription background task
    use_hook(move || {
        let store = store;
        spawn(async move {
            // 1. Initial fetch from settings daemon
            if let Ok(Ok(entries)) = tokio::task::spawn_blocking(|| {
                get_all_settings(SETTINGS_SOCKET_NAME)
            })
            .await
            {
                store.apply_entries(&entries);
            }

            // 2. Subscribe to settings changes and lock updates
            if let Ok(mut rx) = subscribe_channel(SETTINGS_SOCKET_NAME, SubscriptionFilter::all()) {
                while let Some(event) = rx.recv().await {
                    match event {
                        ipsea::settings::SettingsEvent::Changed { key, value, .. } => {
                            store.apply_to_local_state(&key, &value);
                        }
                        ipsea::settings::SettingsEvent::LockChanged { key, lock_state } => {
                            let mut locks = store.lock_states;
                            locks.write().insert(key, lock_state);
                        }
                        ipsea::settings::SettingsEvent::Reloaded => {
                            if let Ok(Ok(entries)) = tokio::task::spawn_blocking(|| {
                                get_all_settings(SETTINGS_SOCKET_NAME)
                            })
                            .await
                            {
                                store.apply_entries(&entries);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
    });

    store
}

/// Retrieves the SettingsStore from the Freya context.
pub fn use_settings() -> SettingsStore {
    consume_context::<SettingsStore>()
}
