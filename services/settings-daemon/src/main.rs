use {
    config::{self, SettingsPayload},
    ipsea::{
        settings::{
            ChangeSource, DenialReason, LockSource, LockState, SettingEntry, SettingKey, SettingValue, SettingsEvent,
            SettingsRequest, SettingsResponse, SubscriptionFilter, SETTINGS_SOCKET_NAME,
        },
        start_server,
    },
    std::{
        sync::{mpsc::Sender, Arc, Mutex},
        time::Duration,
    },
    system::{HyprlandBackend, SystemBackend},
    tokio::time,
};

pub mod notifications;

/// All standard setting keys managed and exposed by the Finick Settings Daemon.
pub const ALL_SETTING_KEYS: &[SettingKey] = &[
    SettingKey::ThemeMode,
    SettingKey::AccentColor,
    SettingKey::Wallpaper,
    SettingKey::Scrollbars,
    SettingKey::IconSize,
    SettingKey::WindowGapsIn,
    SettingKey::WindowGapsOut,
    SettingKey::WindowBorderSize,
    SettingKey::WifiEnabled,
    SettingKey::WifiNetwork,
    SettingKey::BluetoothEnabled,
    SettingKey::AudioVolume,
    SettingKey::AudioMuted,
    SettingKey::AudioDefaultSink,
    SettingKey::DisplayResolution,
    SettingKey::DisplayOrientation,
    SettingKey::DisplayScale,
    SettingKey::DisplayBrightness,
    SettingKey::DisplayAutoBrightness,
    SettingKey::DisplayNightShift,
    SettingKey::DisplayColorTemp,
    SettingKey::Hostname,
    SettingKey::Locale,
    SettingKey::Timezone,
    SettingKey::TimeFormat24h,
    SettingKey::NtpEnabled,
    SettingKey::PowerProfile,
    SettingKey::SystemAutoUpdates,
    SettingKey::CameraAccess,
    SettingKey::MicAccess,
    SettingKey::LocationAccess,
    SettingKey::AccessibilityReduceMotion,
    SettingKey::AccessibilityIncreaseContrast,
    SettingKey::AccessibilityReduceTransparency,
    SettingKey::AccessibilityScreenReader,
    SettingKey::AccessibilityTextSize,
    SettingKey::StorageEmptyTrashAuto,
    SettingKey::KeyboardLayout,
    SettingKey::DoNotDisturb,
    SettingKey::NotificationBanners,
    SettingKey::NotificationSounds,
];

/// Resolves a `SettingKey` to its canonical dot-separated configuration path within `SettingsPayload`.
pub fn setting_key_to_config_path(key: &SettingKey) -> &str {
    match key {
        SettingKey::ThemeMode => "personalization.appearance.mode",
        SettingKey::AccentColor => "personalization.appearance.accent_color",
        SettingKey::Wallpaper => "personalization.appearance.wallpaper",
        SettingKey::Scrollbars => "personalization.appearance.scrollbar_pref",
        SettingKey::IconSize => "personalization.appearance.icon_size_pref",
        SettingKey::WindowGapsIn => "personalization.appearance.gaps_in",
        SettingKey::WindowGapsOut => "personalization.appearance.gaps_out",
        SettingKey::WindowBorderSize => "personalization.appearance.border_size",
        SettingKey::WifiEnabled => "connectivity.wifi.enabled",
        SettingKey::WifiNetwork => "connectivity.wifi.ask_to_join",
        SettingKey::BluetoothEnabled => "connectivity.bluetooth.enabled",
        SettingKey::VpnEnabled => "connectivity.vpn_enabled",
        SettingKey::AirplaneMode => "connectivity.airplane_mode",
        SettingKey::AudioVolume => "personalization.sound.volume",
        SettingKey::AudioMuted => "personalization.sound.muted",
        SettingKey::AudioDefaultSink => "sound.default_sink",
        SettingKey::MicVolume => "sound.mic_volume",
        SettingKey::MicMuted => "sound.mic_muted",
        SettingKey::DisplayScale => "displays.scale",
        SettingKey::DisplayResolution => "personalization.display.resolution_choice",
        SettingKey::DisplayRefreshRate => "displays.refresh_rate",
        SettingKey::DisplayOrientation => "displays.orientation",
        SettingKey::DisplayBrightness => "personalization.display.brightness",
        SettingKey::DisplayAutoBrightness => "personalization.display.auto_brightness",
        SettingKey::DisplayNightShift => "personalization.display.night_shift",
        SettingKey::DisplayColorTemp => "personalization.display.color_temp",
        SettingKey::Hostname => "system.general.hostname",
        SettingKey::Locale => "system.language.primary_locale",
        SettingKey::Timezone => "system.date_time.timezone",
        SettingKey::TimeFormat24h => "system.date_time.time_format_24h",
        SettingKey::NtpEnabled => "system.date_time.ntp_sync",
        SettingKey::PowerProfile => "system.battery.mode",
        SettingKey::ScreenTimeEnabled => "general.screen_time_enabled",
        SettingKey::SystemAutoUpdates => "system.general.auto_updates",
        SettingKey::FirewallEnabled => "system.privacy.firewall_enabled",
        SettingKey::CameraAccess => "system.privacy.camera_enabled",
        SettingKey::MicAccess => "system.privacy.microphone_enabled",
        SettingKey::LocationAccess => "system.privacy.location_enabled",
        SettingKey::AccessibilityReduceMotion => "system.accessibility.reduce_motion",
        SettingKey::AccessibilityIncreaseContrast => "system.accessibility.increase_contrast",
        SettingKey::AccessibilityReduceTransparency => "system.accessibility.reduce_transparency",
        SettingKey::AccessibilityScreenReader => "system.accessibility.screen_reader",
        SettingKey::AccessibilityTextSize => "system.accessibility.text_size",
        SettingKey::StorageEmptyTrashAuto => "system.storage.empty_trash_auto",
        SettingKey::KeyboardLayout => "system.language.keyboard_layout",
        SettingKey::DoNotDisturb => "personalization.focus.mode",
        SettingKey::NotificationBanners => "personalization.notifications.enabled",
        SettingKey::NotificationSounds => "personalization.notifications.silence_during_sleep",
        SettingKey::Custom(s) => s.as_str(),
    }
}

/// Checks whether a setting is specifically locked and managed by Nix/NixOS.
pub fn is_nix_locked(payload: &SettingsPayload, key: &SettingKey) -> bool {
    let path = setting_key_to_config_path(key);
    payload.is_nix_locked(path) || payload.is_nix_locked(key.as_str())
}

/// Checks whether a setting is locked by any authoritative source (Nix, Policy, etc).
pub fn is_locked(payload: &SettingsPayload, key: &SettingKey) -> bool {
    let path = setting_key_to_config_path(key);
    payload.is_key_locked(path) || payload.is_key_locked(key.as_str())
}

/// Retrieves the typed `LockState` for a setting key.
pub fn get_lock_state(payload: &SettingsPayload, key: &SettingKey) -> LockState {
    let path = setting_key_to_config_path(key);
    let lock = payload.get_lock(path).or_else(|| payload.get_lock(key.as_str()));

    if let Some(l) = lock {
        if l.is_locked() {
            let reason = l.message.as_deref().unwrap_or("Setting is locked by system policy").to_string();
            if l.is_nix_locked() {
                return LockState::nix(reason);
            } else if let Some(config::LockReason::SystemPolicy) = l.reason {
                return LockState::policy(reason);
            } else {
                return LockState::locked(LockSource::Other("Config".to_string()), reason);
            }
        }
    }
    LockState::unlocked()
}

/// Constructs a full `SettingEntry` from the in-memory `SettingsPayload` and system status.
pub fn get_setting_entry(payload: &SettingsPayload, backend: &HyprlandBackend, key: &SettingKey) -> Option<SettingEntry> {
    let lock_state = get_lock_state(payload, key);
    let val: Option<SettingValue> = match key {
        SettingKey::ThemeMode => {
            let mode_str = match *payload.personalization.appearance.mode {
                config::ThemeMode::Auto => "auto",
                config::ThemeMode::Light => "light",
                config::ThemeMode::Dark => "dark",
            };
            Some(SettingValue::String(mode_str.to_string()))
        }
        SettingKey::AccentColor => Some(SettingValue::String((*payload.personalization.appearance.accent_color).clone())),
        SettingKey::Wallpaper => {
            let wp = &payload.personalization.appearance.wallpaper;
            if !wp.is_empty() {
                Some(SettingValue::String((**wp).clone()))
            } else {
                Some(SettingValue::Int(*payload.personalization.appearance.wallpaper_idx as i64))
            }
        }
        SettingKey::Scrollbars => Some(SettingValue::Int(*payload.personalization.appearance.scrollbar_pref as i64)),
        SettingKey::IconSize => Some(SettingValue::Int(*payload.personalization.appearance.icon_size_pref as i64)),
        SettingKey::WindowGapsIn => Some(SettingValue::Int(*payload.personalization.appearance.gaps_in as i64)),
        SettingKey::WindowGapsOut => Some(SettingValue::Int(*payload.personalization.appearance.gaps_out as i64)),
        SettingKey::WindowBorderSize => Some(SettingValue::Int(*payload.personalization.appearance.border_size as i64)),
        SettingKey::WifiEnabled => Some(SettingValue::Bool(*payload.connectivity.wifi.enabled)),
        SettingKey::WifiNetwork => Some(SettingValue::Bool(*payload.connectivity.wifi.ask_to_join)),
        SettingKey::BluetoothEnabled => Some(SettingValue::Bool(*payload.connectivity.bluetooth.enabled)),
        SettingKey::AudioVolume => {
            let info = backend.get_audio_info();
            Some(SettingValue::Int(info.volume.round() as i64))
        }
        SettingKey::AudioMuted => {
            let info = backend.get_audio_info();
            Some(SettingValue::Bool(info.is_muted))
        }
        SettingKey::AudioDefaultSink => Some(SettingValue::String(backend.get_audio_info().default_sink_name)),
        SettingKey::DisplayResolution => {
            Some(SettingValue::String((*payload.personalization.display.resolution_choice).clone()))
        }
        SettingKey::DisplayOrientation => {
            payload.custom.get("displays.orientation").and_then(|v| v.as_str().map(|s| SettingValue::String(s.to_string())))
        }
        SettingKey::DisplayScale => payload.custom.get("displays.scale").and_then(|v| {
            v.as_f64().map(SettingValue::Float).or_else(|| v.as_str().map(|s| SettingValue::String(s.to_string())))
        }),
        SettingKey::DisplayBrightness => Some(SettingValue::Float(*payload.personalization.display.brightness as f64)),
        SettingKey::DisplayAutoBrightness => Some(SettingValue::Bool(*payload.personalization.display.auto_brightness)),
        SettingKey::DisplayNightShift => Some(SettingValue::Bool(*payload.personalization.display.night_shift)),
        SettingKey::DisplayColorTemp => Some(SettingValue::Float(*payload.personalization.display.color_temp as f64)),
        SettingKey::Hostname => Some(SettingValue::String(backend.get_host_info().hostname)),
        SettingKey::Locale => Some(SettingValue::String((*payload.system.language.primary_locale).clone())),
        SettingKey::Timezone => Some(SettingValue::String((*payload.system.date_time.timezone).clone())),
        SettingKey::TimeFormat24h => Some(SettingValue::Bool(*payload.system.date_time.time_format_24h)),
        SettingKey::NtpEnabled => Some(SettingValue::Bool(*payload.system.date_time.ntp_sync)),
        SettingKey::PowerProfile => Some(SettingValue::String((*payload.system.battery.mode).clone())),
        SettingKey::SystemAutoUpdates => Some(SettingValue::Bool(*payload.system.general.auto_updates)),
        SettingKey::CameraAccess => Some(SettingValue::Bool(*payload.system.privacy.camera_enabled)),
        SettingKey::MicAccess => Some(SettingValue::Bool(*payload.system.privacy.microphone_enabled)),
        SettingKey::LocationAccess => Some(SettingValue::Bool(*payload.system.privacy.location_enabled)),
        SettingKey::AccessibilityReduceMotion => Some(SettingValue::Bool(*payload.system.accessibility.reduce_motion)),
        SettingKey::AccessibilityIncreaseContrast => {
            Some(SettingValue::Bool(*payload.system.accessibility.increase_contrast))
        }
        SettingKey::AccessibilityReduceTransparency => {
            Some(SettingValue::Bool(*payload.system.accessibility.reduce_transparency))
        }
        SettingKey::AccessibilityScreenReader => Some(SettingValue::Bool(*payload.system.accessibility.screen_reader)),
        SettingKey::AccessibilityTextSize => Some(SettingValue::Float(*payload.system.accessibility.text_size as f64)),
        SettingKey::StorageEmptyTrashAuto => Some(SettingValue::Bool(*payload.system.storage.empty_trash_auto)),
        SettingKey::KeyboardLayout => Some(SettingValue::String((*payload.system.language.keyboard_layout).clone())),
        SettingKey::DoNotDisturb => Some(SettingValue::String((*payload.personalization.focus.mode).clone())),
        SettingKey::NotificationBanners => Some(SettingValue::Bool(*payload.personalization.notifications.enabled)),
        SettingKey::NotificationSounds => {
            Some(SettingValue::Bool(*payload.personalization.notifications.silence_during_sleep))
        }
        SettingKey::Custom(k) => payload.custom.get(k).and_then(|v| {
            if let Some(b) = v.as_bool() {
                Some(SettingValue::Bool(b))
            } else if let Some(i) = v.as_i64() {
                Some(SettingValue::Int(i))
            } else if let Some(f) = v.as_f64() {
                Some(SettingValue::Float(f))
            } else if let Some(s) = v.as_str() {
                Some(SettingValue::String(s.to_string()))
            } else {
                None
            }
        }),
        _ => None,
    };
    val.map(|v| SettingEntry::new(key.clone(), v).with_lock(lock_state))
}

/// Mutates the given `SettingsPayload` with a new value for `key`.
pub fn set_setting_value(payload: &mut SettingsPayload, key: &SettingKey, value: &SettingValue) -> Result<(), DenialReason> {
    match key {
        SettingKey::ThemeMode => {
            let mode = match value {
                SettingValue::String(s) => match s.to_lowercase().as_str() {
                    "auto" => config::ThemeMode::Auto,
                    "light" => config::ThemeMode::Light,
                    "dark" => config::ThemeMode::Dark,
                    _ => return Err(DenialReason::InvalidValue(format!("Invalid theme mode: {}", s))),
                },
                SettingValue::Int(0) => config::ThemeMode::Auto,
                SettingValue::Int(1) => config::ThemeMode::Light,
                SettingValue::Int(2) => config::ThemeMode::Dark,
                _ => return Err(DenialReason::InvalidValue("Invalid theme mode value".to_string())),
            };
            payload.personalization.appearance.mode.set(mode).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AccentColor => {
            let s =
                value.as_str().ok_or_else(|| DenialReason::InvalidValue("Expected string for accent color".to_string()))?;
            payload
                .personalization
                .appearance
                .accent_color
                .set(s.to_string())
                .map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::Wallpaper => {
            if let Some(s) = value.as_str() {
                if let Some(idx) = s.strip_prefix("preset:").and_then(|p| p.parse::<usize>().ok()) {
                    let _ = payload.personalization.appearance.wallpaper_idx.set(idx);
                    let color_hex = system::WALLPAPER_COLOR_PRESETS.get(idx).map(|(_, c)| *c).unwrap_or("#1e1e2e");
                    payload
                        .personalization
                        .appearance
                        .wallpaper
                        .set(color_hex.to_string())
                        .map_err(|e| DenialReason::Other(e.to_string()))?;
                } else {
                    payload
                        .personalization
                        .appearance
                        .wallpaper
                        .set(s.to_string())
                        .map_err(|e| DenialReason::Other(e.to_string()))?;
                    if let Some(idx) = system::WALLPAPER_COLOR_PRESETS.iter().position(|(_, hex)| *hex == s) {
                        let _ = payload.personalization.appearance.wallpaper_idx.set(idx);
                    }
                }
            } else if let Some(i) = value.as_i64() {
                let idx = i as usize;
                payload.personalization.appearance.wallpaper_idx.set(idx).map_err(|e| DenialReason::Other(e.to_string()))?;
                let color_hex = system::WALLPAPER_COLOR_PRESETS.get(idx).map(|(_, c)| *c).unwrap_or("#1e1e2e");
                payload
                    .personalization
                    .appearance
                    .wallpaper
                    .set(color_hex.to_string())
                    .map_err(|e| DenialReason::Other(e.to_string()))?;
            } else {
                return Err(DenialReason::InvalidValue("Expected string (path or color) or int for wallpaper".to_string()));
            }
            Ok(())
        }
        SettingKey::Scrollbars => {
            let i = value
                .as_i64()
                .ok_or_else(|| DenialReason::InvalidValue("Expected int for scrollbar preference".to_string()))?;
            payload
                .personalization
                .appearance
                .scrollbar_pref
                .set(i as usize)
                .map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::IconSize => {
            let i = value.as_i64().ok_or_else(|| DenialReason::InvalidValue("Expected int for icon size".to_string()))?;
            payload
                .personalization
                .appearance
                .icon_size_pref
                .set(i as usize)
                .map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::WindowGapsIn => {
            let i = value.as_i64().ok_or_else(|| DenialReason::InvalidValue("Expected int for gaps_in".to_string()))?;
            payload.personalization.appearance.gaps_in.set(i as u32).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::WindowGapsOut => {
            let i = value.as_i64().ok_or_else(|| DenialReason::InvalidValue("Expected int for gaps_out".to_string()))?;
            payload.personalization.appearance.gaps_out.set(i as u32).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::WindowBorderSize => {
            let i = value.as_i64().ok_or_else(|| DenialReason::InvalidValue("Expected int for border_size".to_string()))?;
            payload.personalization.appearance.border_size.set(i as u32).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::WifiEnabled => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for wifi_enabled".to_string()))?;
            payload.connectivity.wifi.enabled.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::WifiNetwork => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for wifi ask_to_join".to_string()))?;
            payload.connectivity.wifi.ask_to_join.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::BluetoothEnabled => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for bluetooth_enabled".to_string()))?;
            payload.connectivity.bluetooth.enabled.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AudioVolume => {
            let i = value.as_i64().ok_or_else(|| DenialReason::InvalidValue("Expected int for audio volume".to_string()))?;
            payload
                .personalization
                .sound
                .volume
                .set(i.clamp(0, 100) as u32)
                .map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AudioMuted => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for audio muted".to_string()))?;
            payload.personalization.sound.muted.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DisplayResolution => {
            let s = value
                .as_str()
                .ok_or_else(|| DenialReason::InvalidValue("Expected string for display resolution".to_string()))?;
            payload
                .personalization
                .display
                .resolution_choice
                .set(s.to_string())
                .map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DisplayBrightness => {
            let f = value
                .as_f64()
                .ok_or_else(|| DenialReason::InvalidValue("Expected float for display brightness".to_string()))?;
            payload.personalization.display.brightness.set(f as f32).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DisplayAutoBrightness => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for display auto brightness".to_string()))?;
            payload.personalization.display.auto_brightness.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DisplayNightShift => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for display night shift".to_string()))?;
            payload.personalization.display.night_shift.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DisplayColorTemp => {
            let f = value.as_f64().ok_or_else(|| DenialReason::InvalidValue("Expected float for color temp".to_string()))?;
            payload.personalization.display.color_temp.set(f as f32).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::Locale => {
            let s = value.as_str().ok_or_else(|| DenialReason::InvalidValue("Expected string for locale".to_string()))?;
            payload.system.language.primary_locale.set(s.to_string()).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::Timezone => {
            let s = value.as_str().ok_or_else(|| DenialReason::InvalidValue("Expected string for timezone".to_string()))?;
            payload.system.date_time.timezone.set(s.to_string()).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::TimeFormat24h => {
            let b = value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for 24h format".to_string()))?;
            payload.system.date_time.time_format_24h.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::NtpEnabled => {
            let b = value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for NTP sync".to_string()))?;
            payload.system.date_time.ntp_sync.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::PowerProfile => {
            let s =
                value.as_str().ok_or_else(|| DenialReason::InvalidValue("Expected string for power profile".to_string()))?;
            payload.system.battery.mode.set(s.to_string()).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::SystemAutoUpdates => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for auto updates".to_string()))?;
            payload.system.general.auto_updates.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::CameraAccess => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for camera access".to_string()))?;
            payload.system.privacy.camera_enabled.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::MicAccess => {
            let b = value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for mic access".to_string()))?;
            payload.system.privacy.microphone_enabled.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::LocationAccess => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for location access".to_string()))?;
            payload.system.privacy.location_enabled.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AccessibilityReduceMotion => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for reduce motion".to_string()))?;
            payload.system.accessibility.reduce_motion.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AccessibilityIncreaseContrast => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for increase contrast".to_string()))?;
            payload.system.accessibility.increase_contrast.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AccessibilityReduceTransparency => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for reduce transparency".to_string()))?;
            payload.system.accessibility.reduce_transparency.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AccessibilityScreenReader => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for screen reader".to_string()))?;
            payload.system.accessibility.screen_reader.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::AccessibilityTextSize => {
            let f = value.as_f64().ok_or_else(|| DenialReason::InvalidValue("Expected float for text size".to_string()))?;
            payload.system.accessibility.text_size.set(f as f32).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::StorageEmptyTrashAuto => {
            let b =
                value.as_bool().ok_or_else(|| DenialReason::InvalidValue("Expected bool for empty trash".to_string()))?;
            payload.system.storage.empty_trash_auto.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::KeyboardLayout => {
            let s = value
                .as_str()
                .ok_or_else(|| DenialReason::InvalidValue("Expected string for keyboard layout".to_string()))?;
            payload.system.language.keyboard_layout.set(s.to_string()).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DoNotDisturb => {
            let s =
                value.as_str().ok_or_else(|| DenialReason::InvalidValue("Expected string for focus mode".to_string()))?;
            payload.personalization.focus.mode.set(s.to_string()).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::NotificationBanners => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for notification banners".to_string()))?;
            payload.personalization.notifications.enabled.set(b).map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::NotificationSounds => {
            let b = value
                .as_bool()
                .ok_or_else(|| DenialReason::InvalidValue("Expected bool for notification sounds".to_string()))?;
            payload
                .personalization
                .notifications
                .silence_during_sleep
                .set(b)
                .map_err(|e| DenialReason::Other(e.to_string()))?;
            Ok(())
        }
        SettingKey::DisplayOrientation => {
            if let Some(s) = value.as_str() {
                payload.custom.insert("displays.orientation".to_string(), serde_json::Value::String(s.to_string()));
                Ok(())
            } else {
                Err(DenialReason::InvalidValue("Expected string for display orientation".to_string()))
            }
        }
        SettingKey::DisplayScale => {
            if let Some(f) = value.as_f64() {
                if let Ok(val) = serde_json::to_value(f) {
                    payload.custom.insert("displays.scale".to_string(), val);
                    return Ok(());
                }
            } else if let Some(s) = value.as_str() {
                payload.custom.insert("displays.scale".to_string(), serde_json::Value::String(s.to_string()));
                return Ok(());
            }
            Err(DenialReason::InvalidValue("Expected float or string for display scale".to_string()))
        }
        SettingKey::Custom(key_str) => {
            let json_val = match value {
                SettingValue::Bool(b) => serde_json::Value::Bool(*b),
                SettingValue::Int(i) => serde_json::Value::Number((*i).into()),
                SettingValue::Float(f) => serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::Null),
                SettingValue::String(s) => serde_json::Value::String(s.clone()),
                _ => serde_json::Value::Null,
            };
            payload.custom.insert(key_str.clone(), json_val);
            Ok(())
        }
        _ => Err(DenialReason::NotFound),
    }
}

pub fn accent_to_hex(s: &str) -> String { config::ty::accent_to_hex(s) }

/// Routes setting changes to the operating system using `HyprlandBackend` and system tools.
pub fn apply_setting(backend: &HyprlandBackend, payload: &SettingsPayload, key: &SettingKey) -> Result<(), String> {
    match key {
        SettingKey::WifiEnabled => {
            let enabled = *payload.connectivity.wifi.enabled;
            backend.set_wifi_status(enabled);
            Ok(())
        }
        SettingKey::BluetoothEnabled => {
            let enabled = *payload.connectivity.bluetooth.enabled;
            backend.set_bluetooth_status(enabled);
            Ok(())
        }
        SettingKey::AudioVolume => {
            let vol = *payload.personalization.sound.volume as i32;
            backend.set_volume(vol);
            Ok(())
        }
        SettingKey::AudioMuted => {
            let target_muted = *payload.personalization.sound.muted;
            backend.set_mute(target_muted);
            Ok(())
        }
        SettingKey::AccentColor => {
            let color_str = &payload.personalization.appearance.accent_color;
            let hex = accent_to_hex(color_str);
            HyprlandBackend::hyprctl_set_config(
                &format!("hl.config({{ general = {{ col = {{ active_border = {{ colors = {{ \"0xff{hex}\" }} }} }} }} }})"),
                "general:col.active_border",
                &format!("0xff{hex}"),
            )
        }
        SettingKey::WindowGapsIn => {
            let gaps = *payload.personalization.appearance.gaps_in;
            HyprlandBackend::hyprctl_set_config(
                &format!("hl.config({{ general = {{ gaps_in = {gaps} }} }})"),
                "general:gaps_in",
                &gaps.to_string(),
            )
        }
        SettingKey::WindowGapsOut => {
            let gaps = *payload.personalization.appearance.gaps_out;
            HyprlandBackend::hyprctl_set_config(
                &format!("hl.config({{ general = {{ gaps_out = {gaps} }} }})"),
                "general:gaps_out",
                &gaps.to_string(),
            )
        }
        SettingKey::WindowBorderSize => {
            let border = *payload.personalization.appearance.border_size;
            HyprlandBackend::hyprctl_set_config(
                &format!("hl.config({{ general = {{ border_size = {border} }} }})"),
                "general:border_size",
                &border.to_string(),
            )
        }
        SettingKey::NtpEnabled => {
            let ntp = *payload.system.date_time.ntp_sync;
            let status =
                std::process::Command::new("timedatectl").args(["set-ntp", if ntp { "true" } else { "false" }]).status();
            match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => Err(format!("timedatectl failed with exit code {:?}", s.code())),
                Err(e) => Err(format!("Failed to execute timedatectl: {}", e)),
            }
        }
        SettingKey::Timezone => {
            let tz = &payload.system.date_time.timezone;
            let status = std::process::Command::new("timedatectl").args(["set-timezone", tz]).status();
            match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => Err(format!("timedatectl failed with exit code {:?}", s.code())),
                Err(e) => Err(format!("Failed to execute timedatectl: {}", e)),
            }
        }
        SettingKey::KeyboardLayout => {
            let layout = &payload.system.language.keyboard_layout;
            HyprlandBackend::hyprctl_set_config(
                &format!("hl.config({{ input = {{ kb_layout = \"{layout}\" }} }})"),
                "input:kb_layout",
                layout,
            )
        }
        SettingKey::Wallpaper => {
            let wp = &payload.personalization.appearance.wallpaper;
            let target = if !wp.is_empty() {
                (**wp).clone()
            } else {
                format!("preset:{}", *payload.personalization.appearance.wallpaper_idx)
            };
            backend.set_wallpaper(&target).map_err(|e| format!("Failed to apply wallpaper: {e}"))
        }
        SettingKey::StorageEmptyTrashAuto => {
            if *payload.system.storage.empty_trash_auto {
                let _ = backend.empty_trash();
            }
            Ok(())
        }
        SettingKey::DisplayBrightness => {
            backend.set_brightness(*payload.personalization.display.brightness as u32);
            Ok(())
        }
        SettingKey::DisplayNightShift | SettingKey::DisplayColorTemp => {
            let enabled = *payload.personalization.display.night_shift;
            let ct = *payload.personalization.display.color_temp;
            let _ = backend.set_night_shift(enabled, ct);
            Ok(())
        }
        SettingKey::DisplayResolution | SettingKey::DisplayOrientation | SettingKey::DisplayScale => {
            backend.restore_display_configs()
        }
        SettingKey::Custom(key_str) => {
            if key_str == "desktop.layout" {
                if let Some(layout_val) = payload.custom.get("desktop.layout").and_then(|v| v.as_str()) {
                    let _ = HyprlandBackend::hyprctl_set_config(
                        &format!("hl.config({{ general = {{ layout = \"{layout_val}\" }} }})"),
                        "general:layout",
                        layout_val,
                    );
                }
            } else if key_str == "desktop.workspace_gap" {
                if let Some(gap_val) = payload
                    .custom
                    .get("desktop.workspace_gap")
                    .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                {
                    let gap_int = gap_val as i32;
                    let _ = HyprlandBackend::hyprctl_set_config(
                        &format!("hl.config({{ general = {{ gaps_out = {gap_int} }} }})"),
                        "general:gaps_out",
                        &gap_int.to_string(),
                    );
                }
            }
            Ok(())
        }
        _ => {
            // Unbound settings apply cleanly without hardware side effects
            Ok(())
        }
    }
}

/// Applies all active system settings to the OS environment.
pub fn apply_all_settings(backend: &HyprlandBackend, payload: &SettingsPayload) {
    backend.set_wifi_status(*payload.connectivity.wifi.enabled);
    backend.set_bluetooth_status(*payload.connectivity.bluetooth.enabled);
    backend.set_volume(*payload.personalization.sound.volume as i32);
    let cur_audio = backend.get_audio_info();
    if cur_audio.is_muted != *payload.personalization.sound.muted {
        backend.set_mute(*payload.personalization.sound.muted);
    }
    let accent_hex = accent_to_hex(&payload.personalization.appearance.accent_color);
    let _ = HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ general = {{ col = {{ active_border = {{ colors = {{ \"0xff{accent_hex}\" }} }} }} }} }})"),
        "general:col.active_border",
        &format!("0xff{accent_hex}"),
    );
    let gaps_in = payload.personalization.appearance.gaps_in.to_string();
    let _ = HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ general = {{ gaps_in = {gaps_in} }} }})"),
        "general:gaps_in",
        &gaps_in,
    );
    let gaps_out = payload.personalization.appearance.gaps_out.to_string();
    let _ = HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ general = {{ gaps_out = {gaps_out} }} }})"),
        "general:gaps_out",
        &gaps_out,
    );
    let border_size = payload.personalization.appearance.border_size.to_string();
    let _ = HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ general = {{ border_size = {border_size} }} }})"),
        "general:border_size",
        &border_size,
    );
    let _ = std::process::Command::new("timedatectl")
        .args(["set-ntp", if *payload.system.date_time.ntp_sync { "true" } else { "false" }])
        .output();
    let _ = std::process::Command::new("timedatectl").args(["set-timezone", &payload.system.date_time.timezone]).output();
    let kb = &payload.system.language.keyboard_layout;
    let _ = HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ input = {{ kb_layout = \"{kb}\" }} }})"),
        "input:kb_layout",
        kb,
    );
    let wp = if !payload.personalization.appearance.wallpaper.is_empty() {
        payload.personalization.appearance.wallpaper.to_string()
    } else {
        format!("preset:{}", *payload.personalization.appearance.wallpaper_idx)
    };
    let _ = backend.set_wallpaper(&wp);
    let _ = backend.restore_display_configs();
    let _ =
        backend.set_night_shift(*payload.personalization.display.night_shift, *payload.personalization.display.color_temp);

    if let Some(layout_val) = payload.custom.get("desktop.layout").and_then(|v| v.as_str()) {
        let _ = HyprlandBackend::hyprctl_set_config(
            &format!("hl.config({{ general = {{ layout = \"{layout_val}\" }} }})"),
            "general:layout",
            layout_val,
        );
    }
    if let Some(gap_val) =
        payload.custom.get("desktop.workspace_gap").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
    {
        let gap_int = gap_val as i32;
        let _ = HyprlandBackend::hyprctl_set_config(
            &format!("hl.config({{ general = {{ gaps_out = {gap_int} }} }})"),
            "general:gaps_out",
            &gap_int.to_string(),
        );
    }
}

/// Internal shared state of the Finick Settings Daemon.
pub struct DaemonState {
    pub payload: SettingsPayload,
    pub backend: HyprlandBackend,
    pub subscribers: Vec<(SubscriptionFilter, Sender<SettingsResponse>)>,
}

impl DaemonState {
    /// Broadcasts an event to all subscribed client channels whose filter criteria match.
    pub fn broadcast_event(&mut self, event: SettingsEvent) {
        self.subscribers.retain(|(filter, sender)| match &event {
            SettingsEvent::Changed { key, .. } => {
                let cat = key.category();
                if filter.matches_setting(key, &cat) {
                    sender.send(SettingsResponse::Event(event.clone())).is_ok()
                } else {
                    true
                }
            }
            SettingsEvent::LockChanged { key, .. } => {
                let cat = key.category();
                if filter.matches_setting(key, &cat) {
                    sender.send(SettingsResponse::Event(event.clone())).is_ok()
                } else {
                    true
                }
            }
            SettingsEvent::Alert(alert) => {
                if filter.matches_alert() {
                    sender.send(SettingsResponse::Alert(alert.clone())).is_ok()
                } else {
                    true
                }
            }
            SettingsEvent::Reloaded => sender.send(SettingsResponse::Event(event.clone())).is_ok(),
        });
    }
}

/// Processes an incoming `SettingsRequest` and replies via `sender`.
pub fn handle_request(state: &Arc<Mutex<DaemonState>>, req: SettingsRequest, sender: Sender<SettingsResponse>) {
    match req {
        SettingsRequest::Ping => {
            let _ = sender.send(SettingsResponse::Pong);
        }
        SettingsRequest::Get { key } => {
            let state = state.lock().unwrap();
            if let Some(entry) = get_setting_entry(&state.payload, &state.backend, &key) {
                let _ = sender.send(SettingsResponse::Setting(entry));
            } else {
                let _ = sender.send(SettingsResponse::NotFound { key });
            }
        }
        SettingsRequest::GetCategory { category } => {
            let state = state.lock().unwrap();
            for key in ALL_SETTING_KEYS {
                if key.category() == category {
                    if let Some(entry) = get_setting_entry(&state.payload, &state.backend, key) {
                        let _ = sender.send(SettingsResponse::Setting(entry));
                    }
                }
            }
        }
        SettingsRequest::GetAll => {
            let state = state.lock().unwrap();
            for key in ALL_SETTING_KEYS {
                if let Some(entry) = get_setting_entry(&state.payload, &state.backend, key) {
                    let _ = sender.send(SettingsResponse::Setting(entry));
                }
            }
            for k in state.payload.custom.keys() {
                let key = SettingKey::Custom(k.clone());
                if let Some(entry) = get_setting_entry(&state.payload, &state.backend, &key) {
                    let _ = sender.send(SettingsResponse::Setting(entry));
                }
            }
        }
        SettingsRequest::GetLocks { category } => {
            let state = state.lock().unwrap();
            for key in ALL_SETTING_KEYS {
                if category.map_or(true, |c| key.category() == c) {
                    let lock_state = get_lock_state(&state.payload, key);
                    if lock_state.is_locked() {
                        let _ = sender.send(SettingsResponse::LockState { key: key.clone(), lock_state });
                    }
                }
            }
        }
        SettingsRequest::Set { key, value, source } => {
            let mut state = state.lock().unwrap();
            if is_nix_locked(&state.payload, &key) {
                let lock_state = get_lock_state(&state.payload, &key);
                let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::Locked(lock_state) });
                return;
            }
            if is_locked(&state.payload, &key) {
                let lock_state = get_lock_state(&state.payload, &key);
                let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::Locked(lock_state) });
                return;
            }

            match set_setting_value(&mut state.payload, &key, &value) {
                Ok(()) => {
                    if let Err(e) = config::save_settings(&state.payload) {
                        let _ = sender.send(SettingsResponse::Error(format!("Failed to save settings: {}", e)));
                        return;
                    }
                    state.broadcast_event(SettingsEvent::Changed { key, value, source });
                    let _ = sender.send(SettingsResponse::Ok);
                }
                Err(denial) => {
                    let _ = sender.send(SettingsResponse::Rejected { key, reason: denial });
                }
            }
        }
        SettingsRequest::SetBatch { settings, source } => {
            let mut state = state.lock().unwrap();
            for (key, _) in &settings {
                if is_nix_locked(&state.payload, key) || is_locked(&state.payload, key) {
                    let lock_state = get_lock_state(&state.payload, key);
                    let _ = sender
                        .send(SettingsResponse::Rejected { key: key.clone(), reason: DenialReason::Locked(lock_state) });
                    return;
                }
            }

            let mut changed = Vec::new();
            for (key, value) in settings {
                match set_setting_value(&mut state.payload, &key, &value) {
                    Ok(()) => changed.push((key, value)),
                    Err(denial) => {
                        let _ = sender.send(SettingsResponse::Rejected { key, reason: denial });
                        return;
                    }
                }
            }

            if let Err(e) = config::save_settings(&state.payload) {
                let _ = sender.send(SettingsResponse::Error(format!("Failed to save settings: {}", e)));
                return;
            }

            for (key, value) in changed {
                state.broadcast_event(SettingsEvent::Changed { key, value, source: source.clone() });
            }
            let _ = sender.send(SettingsResponse::Ok);
        }
        SettingsRequest::Apply { key } => {
            let state = state.lock().unwrap();
            match apply_setting(&state.backend, &state.payload, &key) {
                Ok(()) => {
                    let _ = sender.send(SettingsResponse::Ok);
                }
                Err(err) => {
                    let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::ApplyFailed(err) });
                }
            }
        }
        SettingsRequest::ApplyAll => {
            let state = state.lock().unwrap();
            apply_all_settings(&state.backend, &state.payload);
            let _ = sender.send(SettingsResponse::Ok);
        }
        SettingsRequest::SetAndApply { key, value, source } => {
            let mut state = state.lock().unwrap();
            if is_nix_locked(&state.payload, &key) {
                let lock_state = get_lock_state(&state.payload, &key);
                let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::Locked(lock_state) });
                return;
            }
            if is_locked(&state.payload, &key) {
                let lock_state = get_lock_state(&state.payload, &key);
                let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::Locked(lock_state) });
                return;
            }

            match set_setting_value(&mut state.payload, &key, &value) {
                Ok(()) => {
                    if let Err(e) = config::save_settings(&state.payload) {
                        let _ = sender.send(SettingsResponse::Error(format!("Failed to save settings: {}", e)));
                        return;
                    }
                    if let Err(err) = apply_setting(&state.backend, &state.payload, &key) {
                        let _ = sender
                            .send(SettingsResponse::Rejected { key: key.clone(), reason: DenialReason::ApplyFailed(err) });
                        return;
                    }
                    state.broadcast_event(SettingsEvent::Changed { key, value, source });
                    let _ = sender.send(SettingsResponse::Ok);
                }
                Err(denial) => {
                    let _ = sender.send(SettingsResponse::Rejected { key, reason: denial });
                }
            }
        }
        SettingsRequest::SetAndApplyBatch { settings, source } => {
            let mut state = state.lock().unwrap();
            for (key, _) in &settings {
                if is_nix_locked(&state.payload, key) || is_locked(&state.payload, key) {
                    let lock_state = get_lock_state(&state.payload, key);
                    let _ = sender
                        .send(SettingsResponse::Rejected { key: key.clone(), reason: DenialReason::Locked(lock_state) });
                    return;
                }
            }

            let mut changed = Vec::new();
            for (key, value) in settings {
                match set_setting_value(&mut state.payload, &key, &value) {
                    Ok(()) => {
                        if let Err(err) = apply_setting(&state.backend, &state.payload, &key) {
                            let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::ApplyFailed(err) });
                            return;
                        }
                        changed.push((key, value));
                    }
                    Err(denial) => {
                        let _ = sender.send(SettingsResponse::Rejected { key, reason: denial });
                        return;
                    }
                }
            }

            if let Err(e) = config::save_settings(&state.payload) {
                let _ = sender.send(SettingsResponse::Error(format!("Failed to save settings: {}", e)));
                return;
            }

            for (key, value) in changed {
                state.broadcast_event(SettingsEvent::Changed { key, value, source: source.clone() });
            }
            let _ = sender.send(SettingsResponse::Ok);
        }
        SettingsRequest::Reset { key } => {
            let mut state = state.lock().unwrap();
            if is_nix_locked(&state.payload, &key) || is_locked(&state.payload, &key) {
                let lock_state = get_lock_state(&state.payload, &key);
                let _ = sender.send(SettingsResponse::Rejected { key, reason: DenialReason::Locked(lock_state) });
                return;
            }
            let default_payload = config::SettingsPayload::default();
            if let Some(default_entry) = get_setting_entry(&default_payload, &state.backend, &key) {
                let _ = set_setting_value(&mut state.payload, &key, &default_entry.value);
                let _ = config::save_settings(&state.payload);
                state.broadcast_event(SettingsEvent::Changed {
                    key,
                    value: default_entry.value,
                    source: ChangeSource::Daemon,
                });
                let _ = sender.send(SettingsResponse::Ok);
            } else {
                let _ = sender.send(SettingsResponse::NotFound { key });
            }
        }
        SettingsRequest::ResetAll { category } => {
            let mut state = state.lock().unwrap();
            let default_payload = config::SettingsPayload::default();
            for key in ALL_SETTING_KEYS {
                if category.map_or(true, |c| key.category() == c) && !is_locked(&state.payload, key) {
                    if let Some(default_entry) = get_setting_entry(&default_payload, &state.backend, key) {
                        let _ = set_setting_value(&mut state.payload, key, &default_entry.value);
                    }
                }
            }
            let _ = config::save_settings(&state.payload);
            state.broadcast_event(SettingsEvent::Reloaded);
            let _ = sender.send(SettingsResponse::Ok);
        }
        SettingsRequest::Subscribe { filter } => {
            let mut state = state.lock().unwrap();
            state.subscribers.push((filter, sender));
        }
        SettingsRequest::ReloadFromDisk => {
            let mut state = state.lock().unwrap();
            match config::load_settings() {
                Ok(payload) => {
                    apply_all_settings(&state.backend, &payload);
                    state.payload = payload;
                    state.broadcast_event(SettingsEvent::Reloaded);
                    let _ = sender.send(SettingsResponse::Ok);
                }
                Err(e) => {
                    let _ = sender.send(SettingsResponse::Error(format!("Failed to reload settings: {}", e)));
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Finick Settings Daemon...");

    let payload = config::load_settings().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load existing settings ({}). Initializing defaults.", e);
        config::SettingsPayload::default()
    });

    let backend = HyprlandBackend;
    apply_all_settings(&backend, &payload);
    let state = Arc::new(Mutex::new(DaemonState { payload, backend, subscribers: Vec::new() }));

    // Mock tracking loop for active window and screen time
    tokio::spawn(async {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            // In a real implementation, we would query Hyprland for the active window
            // using `hyprctl activewindow -j` and log the time spent per application.
            // We'd also manage the system's indexing processes for Spotlight-like search.
            println!("Tracking screen time... (mock tick)");
        }
    });

    // 1. Notification broadcaster and IPC server
    let notif_broadcaster = ipsea::notifications::NotificationBroadcaster::new();
    let notif_broadcaster_clone = notif_broadcaster.clone();
    let notif_socket = ipsea::notifications::NOTIFICATIONS_SOCKET_NAME;
    println!("Binding Notifications IPC server on socket '{}'...", notif_socket);

    let (close_tx, mut close_rx) = tokio::sync::mpsc::unbounded_channel::<u32>();

    let notif_ipc_task = tokio::task::spawn_blocking(move || {
        ipsea::notifications::start_notification_server(
            notif_socket,
            notif_broadcaster_clone,
            Some(move |id: u32| {
                let _ = close_tx.send(id);
            }),
        )
    });

    // 2. DBus Notifications Server (org.freedesktop.Notifications)
    let notif_server = notifications::NotificationServer::new(notif_broadcaster);
    match notifications::start_dbus_notifications_server(notif_server).await {
        Ok(dbus_conn) => {
            println!("Started org.freedesktop.Notifications DBus service on session bus.");
            tokio::spawn(async move {
                while let Some(id) = close_rx.recv().await {
                    let _ = notifications::emit_notification_closed(&dbus_conn, id, 2).await;
                }
            });
        }
        Err(e) => {
            eprintln!("Warning: Failed to register org.freedesktop.Notifications on DBus: {e}");
        }
    }

    // 3. Screen Time Tracker background task
    let state_st = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let enabled = {
                let s = state_st.lock().unwrap();
                *s.payload.system.general.screen_time_enabled.get()
            };
            if !enabled {
                continue;
            }
            let active_app = tokio::task::spawn_blocking(|| {
                if let Ok(output) = std::process::Command::new("hyprctl").args(["activewindow", "-j"]).output() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                        return v.get("class").and_then(|c| c.as_str()).map(|s| s.to_string());
                    }
                }
                None
            })
            .await
            .unwrap_or(None);

            if let Some(app) = active_app {
                if !app.is_empty() {
                    if let Ok(home) = std::env::var("HOME") {
                        let path = std::path::PathBuf::from(&home).join(".local/share/finick/screen_time.json");
                        let _ = std::fs::create_dir_all(path.parent().unwrap());
                        let mut map: std::collections::HashMap<String, u64> =
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                serde_json::from_str(&content).unwrap_or_default()
                            } else {
                                std::collections::HashMap::new()
                            };
                        *map.entry(app).or_insert(0) += 5;
                        if let Ok(serialized) = serde_json::to_string_pretty(&map) {
                            let _ = std::fs::write(&path, serialized);
                        }
                    }
                }
            }
        }
    });

    // 4. Settings IPC server
    let state_clone = Arc::clone(&state);
    let socket_name = SETTINGS_SOCKET_NAME;
    println!("Binding Settings IPC server on socket '{}'...", socket_name);

    let settings_ipc_task = tokio::task::spawn_blocking(move || {
        start_server(socket_name, move |req: SettingsRequest, sender: Sender<SettingsResponse>| {
            handle_request(&state_clone, req, sender);
        })
    });

    tokio::select! {
        res = settings_ipc_task => {
            if let Err(e) = res {
                eprintln!("Settings IPC task failed: {e}");
            }
        }
        res = notif_ipc_task => {
            if let Err(e) = res {
                eprintln!("Notifications IPC task failed: {e}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        ipsea::settings::{get_all_settings, get_setting, set_setting},
        std::thread,
    };

    #[test]
    fn test_nix_policy_checks() {
        let mut payload = SettingsPayload::new();
        assert!(!is_nix_locked(&payload, &SettingKey::WifiEnabled));

        payload.lock_nix("connectivity.wifi.enabled", "Managed declaratively by NixOS");
        assert!(is_nix_locked(&payload, &SettingKey::WifiEnabled));
        assert!(is_locked(&payload, &SettingKey::WifiEnabled));

        let lock_state = get_lock_state(&payload, &SettingKey::WifiEnabled);
        assert!(lock_state.is_locked());
        assert!(lock_state.is_nix_locked());
        assert_eq!(lock_state.reason(), Some("Managed declaratively by NixOS"));
    }

    #[test]
    fn test_set_setting_value_mutations() {
        let mut payload = SettingsPayload::new();

        // Theme mode mutation
        assert!(set_setting_value(&mut payload, &SettingKey::ThemeMode, &SettingValue::String("dark".to_string())).is_ok());
        assert_eq!(*payload.personalization.appearance.mode, config::ThemeMode::Dark);

        // Volume mutation with clamping
        assert!(set_setting_value(&mut payload, &SettingKey::AudioVolume, &SettingValue::Int(85)).is_ok());
        assert_eq!(*payload.personalization.sound.volume, 85);

        // Wallpaper mutation with preset and hex sync
        assert!(set_setting_value(&mut payload, &SettingKey::Wallpaper, &SettingValue::Int(2)).is_ok());
        assert_eq!(*payload.personalization.appearance.wallpaper_idx, 2);
        assert_eq!(&*payload.personalization.appearance.wallpaper, system::WALLPAPER_COLOR_PRESETS[2].1);

        assert!(
            set_setting_value(&mut payload, &SettingKey::Wallpaper, &SettingValue::String("preset:3".to_string())).is_ok()
        );
        assert_eq!(*payload.personalization.appearance.wallpaper_idx, 3);
        assert_eq!(&*payload.personalization.appearance.wallpaper, system::WALLPAPER_COLOR_PRESETS[3].1);

        // Display orientation & scale persistence in custom
        assert!(
            set_setting_value(&mut payload, &SettingKey::DisplayOrientation, &SettingValue::String("1".to_string())).is_ok()
        );
        assert_eq!(payload.custom.get("displays.orientation"), Some(&serde_json::Value::String("1".to_string())));

        // Custom dock and desktop keys
        assert!(set_setting_value(
            &mut payload,
            &SettingKey::Custom("desktop.layout".to_string()),
            &SettingValue::String("dwindle".to_string())
        )
        .is_ok());
        assert_eq!(payload.custom.get("desktop.layout"), Some(&serde_json::Value::String("dwindle".to_string())));

        // Invalid theme value
        assert!(set_setting_value(&mut payload, &SettingKey::ThemeMode, &SettingValue::String("neon-green".to_string()))
            .is_err());
    }

    #[test]
    fn test_ipc_server_end_to_end() {
        let socket = format!("test-settings-daemon-{}", std::process::id());
        let mut payload = SettingsPayload::new();
        payload.lock_nix("connectivity.wifi.enabled", "Locked by NixOS flake");

        let backend = HyprlandBackend;
        let state = Arc::new(Mutex::new(DaemonState { payload, backend, subscribers: Vec::new() }));

        let state_clone = Arc::clone(&state);
        let socket_clone = socket.clone();
        thread::spawn(move || {
            let _ = start_server(socket_clone, move |req: SettingsRequest, sender: Sender<SettingsResponse>| {
                handle_request(&state_clone, req, sender);
            });
        });

        // Allow server to bind
        thread::sleep(Duration::from_millis(150));

        // Test Get
        let theme_entry =
            get_setting(&socket, SettingKey::ThemeMode).expect("IPC get_setting failed").expect("ThemeMode entry not found");
        assert_eq!(theme_entry.key, SettingKey::ThemeMode);

        // Test Nix policy rejection on locked setting
        let set_res = set_setting(&socket, SettingKey::WifiEnabled, false).expect("IPC set_setting failed");
        assert!(set_res.is_err());
        let denial = set_res.unwrap_err();
        assert!(denial.is_locked());
        assert!(denial.lock_state().unwrap().is_nix_locked());

        // Test successful Set on unlocked setting
        let ok_res = set_setting(&socket, SettingKey::ThemeMode, "light").expect("IPC set_setting failed");
        assert!(ok_res.is_ok());

        // Verify mutated value persisted in memory
        let updated_theme =
            get_setting(&socket, SettingKey::ThemeMode).expect("IPC get_setting failed").expect("ThemeMode entry not found");
        assert_eq!(updated_theme.value.as_str(), Some("light"));

        // Test GetAll
        let all = get_all_settings(&socket).expect("IPC get_all_settings failed");
        assert!(!all.is_empty());
        assert!(all.iter().any(|e| e.key == SettingKey::WifiEnabled && e.is_nix_locked()));
    }
}
