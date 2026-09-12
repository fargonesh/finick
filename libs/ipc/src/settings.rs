use {
    crate::send_command,
    serde::{Deserialize, Serialize},
    std::{
        fmt::{self, Display},
        path::PathBuf,
        str::FromStr,
        sync::mpsc,
    },
};

/// Default socket name for the finick settings daemon.
pub const SETTINGS_SOCKET_NAME: &str = "finickd";

// ============================================================================
// Setting Categories
// ============================================================================

/// Logical category grouping for system settings.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingCategory {
    Appearance,
    Connectivity,
    Sound,
    Displays,
    General,
    DateTime,
    Language,
    Power,
    Privacy,
    Storage,
    Notifications,
    ScreenTime,
    Desktop,
    Other,
}

impl SettingCategory {
    /// List of all standard categories.
    pub const fn all() -> &'static [SettingCategory] {
        &[
            SettingCategory::Appearance,
            SettingCategory::Connectivity,
            SettingCategory::Sound,
            SettingCategory::Displays,
            SettingCategory::General,
            SettingCategory::DateTime,
            SettingCategory::Language,
            SettingCategory::Power,
            SettingCategory::Privacy,
            SettingCategory::Storage,
            SettingCategory::Notifications,
            SettingCategory::ScreenTime,
            SettingCategory::Desktop,
            SettingCategory::Other,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SettingCategory::Appearance => "appearance",
            SettingCategory::Connectivity => "connectivity",
            SettingCategory::Sound => "sound",
            SettingCategory::Displays => "displays",
            SettingCategory::General => "general",
            SettingCategory::DateTime => "date_time",
            SettingCategory::Language => "language",
            SettingCategory::Power => "power",
            SettingCategory::Privacy => "privacy",
            SettingCategory::Storage => "storage",
            SettingCategory::Notifications => "notifications",
            SettingCategory::ScreenTime => "screen_time",
            SettingCategory::Desktop => "desktop",
            SettingCategory::Other => "other",
        }
    }
}

impl Display for SettingCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SettingCategory {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_lowercase().replace('-', "_");
        Ok(match clean.as_str() {
            "appearance" | "theme" => SettingCategory::Appearance,
            "connectivity" | "network" | "wifi" | "bluetooth" => SettingCategory::Connectivity,
            "sound" | "audio" => SettingCategory::Sound,
            "displays" | "display" | "monitors" => SettingCategory::Displays,
            "general" | "system" | "about" => SettingCategory::General,
            "date_time" | "datetime" | "time" | "date" => SettingCategory::DateTime,
            "language" | "locale" | "region" => SettingCategory::Language,
            "power" | "battery" => SettingCategory::Power,
            "privacy" | "security" | "firewall" => SettingCategory::Privacy,
            "storage" | "disks" | "disk" => SettingCategory::Storage,
            "notifications" | "focus" => SettingCategory::Notifications,
            "screen_time" | "screentime" => SettingCategory::ScreenTime,
            "desktop" | "dock" | "window_manager" | "hyprland" => SettingCategory::Desktop,
            _ => SettingCategory::Other,
        })
    }
}

// ============================================================================
// Setting Keys
// ============================================================================

/// Strongly typed identifiers for system settings keys.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SettingKey {
    // Appearance & Styling
    ThemeMode,
    AccentColor,
    Wallpaper,
    Scrollbars,
    IconSize,

    // Hyprland / Window Manager
    WindowGapsIn,
    WindowGapsOut,
    WindowBorderSize,

    // Connectivity & Network
    WifiEnabled,
    WifiNetwork,
    BluetoothEnabled,
    VpnEnabled,
    AirplaneMode,

    // Sound & Audio
    AudioVolume,
    AudioMuted,
    AudioDefaultSink,
    MicVolume,
    MicMuted,

    // Displays
    DisplayScale,
    DisplayResolution,
    DisplayRefreshRate,
    DisplayOrientation,
    DisplayBrightness,
    DisplayAutoBrightness,
    DisplayNightShift,
    DisplayColorTemp,

    // General & System
    Hostname,
    Locale,
    Timezone,
    TimeFormat24h,
    NtpEnabled,
    PowerProfile,
    ScreenTimeEnabled,
    SystemAutoUpdates,

    // Privacy & Security
    FirewallEnabled,
    CameraAccess,
    MicAccess,
    LocationAccess,

    // Accessibility
    AccessibilityReduceMotion,
    AccessibilityIncreaseContrast,
    AccessibilityReduceTransparency,
    AccessibilityScreenReader,
    AccessibilityTextSize,

    // Storage
    StorageEmptyTrashAuto,

    // Input & Keyboard
    KeyboardLayout,

    // Notifications
    DoNotDisturb,
    NotificationBanners,
    NotificationSounds,

    // Custom / Arbitrary Key fallback
    Custom(String),
}

impl SettingKey {
    /// Return the canonical string identifier for this setting key.
    pub fn as_str(&self) -> &str {
        match self {
            SettingKey::ThemeMode => "appearance.theme_mode",
            SettingKey::AccentColor => "appearance.accent_color",
            SettingKey::Wallpaper => "appearance.wallpaper",
            SettingKey::Scrollbars => "appearance.scrollbars",
            SettingKey::IconSize => "appearance.icon_size",

            SettingKey::WindowGapsIn => "hyprland.gaps_in",
            SettingKey::WindowGapsOut => "hyprland.gaps_out",
            SettingKey::WindowBorderSize => "hyprland.border_size",

            SettingKey::WifiEnabled => "connectivity.wifi_enabled",
            SettingKey::WifiNetwork => "connectivity.wifi_network",
            SettingKey::BluetoothEnabled => "connectivity.bluetooth_enabled",
            SettingKey::VpnEnabled => "connectivity.vpn_enabled",
            SettingKey::AirplaneMode => "connectivity.airplane_mode",

            SettingKey::AudioVolume => "sound.volume",
            SettingKey::AudioMuted => "sound.muted",
            SettingKey::AudioDefaultSink => "sound.default_sink",
            SettingKey::MicVolume => "sound.mic_volume",
            SettingKey::MicMuted => "sound.mic_muted",

            SettingKey::DisplayScale => "displays.scale",
            SettingKey::DisplayResolution => "displays.resolution",
            SettingKey::DisplayRefreshRate => "displays.refresh_rate",
            SettingKey::DisplayOrientation => "displays.orientation",
            SettingKey::DisplayBrightness => "displays.brightness",
            SettingKey::DisplayAutoBrightness => "displays.auto_brightness",
            SettingKey::DisplayNightShift => "displays.night_shift",
            SettingKey::DisplayColorTemp => "displays.color_temp",

            SettingKey::Hostname => "general.hostname",
            SettingKey::Locale => "general.locale",
            SettingKey::Timezone => "general.timezone",
            SettingKey::TimeFormat24h => "general.time_format_24h",
            SettingKey::NtpEnabled => "general.ntp_enabled",
            SettingKey::PowerProfile => "power.profile",
            SettingKey::ScreenTimeEnabled => "general.screen_time_enabled",
            SettingKey::SystemAutoUpdates => "general.auto_updates",

            SettingKey::FirewallEnabled => "privacy.firewall_enabled",
            SettingKey::CameraAccess => "privacy.camera_access",
            SettingKey::MicAccess => "privacy.mic_access",
            SettingKey::LocationAccess => "privacy.location_access",

            SettingKey::AccessibilityReduceMotion => "accessibility.reduce_motion",
            SettingKey::AccessibilityIncreaseContrast => "accessibility.increase_contrast",
            SettingKey::AccessibilityReduceTransparency => "accessibility.reduce_transparency",
            SettingKey::AccessibilityScreenReader => "accessibility.screen_reader",
            SettingKey::AccessibilityTextSize => "accessibility.text_size",

            SettingKey::StorageEmptyTrashAuto => "storage.empty_trash_auto",

            SettingKey::KeyboardLayout => "language.keyboard_layout",

            SettingKey::DoNotDisturb => "notifications.do_not_disturb",
            SettingKey::NotificationBanners => "notifications.banners",
            SettingKey::NotificationSounds => "notifications.sounds",

            SettingKey::Custom(ref s) => s.as_str(),
        }
    }

    /// Return the category this setting belongs to.
    pub fn category(&self) -> SettingCategory {
        match self {
            SettingKey::ThemeMode
            | SettingKey::AccentColor
            | SettingKey::Wallpaper
            | SettingKey::Scrollbars
            | SettingKey::IconSize => SettingCategory::Appearance,

            SettingKey::WindowGapsIn | SettingKey::WindowGapsOut | SettingKey::WindowBorderSize => {
                SettingCategory::Desktop
            }

            SettingKey::WifiEnabled
            | SettingKey::WifiNetwork
            | SettingKey::BluetoothEnabled
            | SettingKey::VpnEnabled
            | SettingKey::AirplaneMode => SettingCategory::Connectivity,

            SettingKey::AudioVolume
            | SettingKey::AudioMuted
            | SettingKey::AudioDefaultSink
            | SettingKey::MicVolume
            | SettingKey::MicMuted => SettingCategory::Sound,

            SettingKey::DisplayScale
            | SettingKey::DisplayResolution
            | SettingKey::DisplayRefreshRate
            | SettingKey::DisplayOrientation
            | SettingKey::DisplayBrightness
            | SettingKey::DisplayAutoBrightness
            | SettingKey::DisplayNightShift
            | SettingKey::DisplayColorTemp => SettingCategory::Displays,

            SettingKey::Hostname
            | SettingKey::ScreenTimeEnabled
            | SettingKey::SystemAutoUpdates
            | SettingKey::AccessibilityReduceMotion
            | SettingKey::AccessibilityIncreaseContrast
            | SettingKey::AccessibilityReduceTransparency
            | SettingKey::AccessibilityScreenReader
            | SettingKey::AccessibilityTextSize => SettingCategory::General,

            SettingKey::Locale | SettingKey::KeyboardLayout => SettingCategory::Language,

            SettingKey::Timezone | SettingKey::TimeFormat24h | SettingKey::NtpEnabled => {
                SettingCategory::DateTime
            }
            SettingKey::PowerProfile => SettingCategory::Power,

            SettingKey::StorageEmptyTrashAuto => SettingCategory::Storage,

            SettingKey::FirewallEnabled
            | SettingKey::CameraAccess
            | SettingKey::MicAccess
            | SettingKey::LocationAccess => SettingCategory::Privacy,

            SettingKey::DoNotDisturb
            | SettingKey::NotificationBanners
            | SettingKey::NotificationSounds => SettingCategory::Notifications,

            SettingKey::Custom(ref s) => {
                if let Some((cat_str, _)) = s.split_once('.') {
                    cat_str.parse().unwrap_or(SettingCategory::Other)
                } else {
                    SettingCategory::Other
                }
            }
        }
    }
}

impl Display for SettingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SettingKey {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_lowercase().replace('-', "_");
        Ok(match clean.as_str() {
            "personalization.appearance.mode" | "appearance.theme_mode" | "theme_mode" | "thememode" => SettingKey::ThemeMode,
            "personalization.appearance.accent_color" | "appearance.accent_color" | "accent_color" | "accent" => SettingKey::AccentColor,
            "personalization.appearance.wallpaper_idx" | "appearance.wallpaper" | "wallpaper" => SettingKey::Wallpaper,
            "personalization.appearance.scrollbar_pref" | "appearance.scrollbars" | "scrollbars" => SettingKey::Scrollbars,
            "personalization.appearance.icon_size_pref" | "appearance.icon_size" | "icon_size" => SettingKey::IconSize,

            "personalization.appearance.gaps_in" | "hyprland.gaps_in" | "gaps_in" => SettingKey::WindowGapsIn,
            "personalization.appearance.gaps_out" | "hyprland.gaps_out" | "gaps_out" => SettingKey::WindowGapsOut,
            "personalization.appearance.border_size" | "hyprland.border_size" | "border_size" => SettingKey::WindowBorderSize,

            "connectivity.wifi.enabled" | "connectivity.wifi_enabled" | "wifi_enabled" | "wifi" => SettingKey::WifiEnabled,
            "connectivity.wifi.ask_to_join" | "connectivity.wifi_network" | "wifi_network" => SettingKey::WifiNetwork,
            "connectivity.bluetooth.enabled" | "connectivity.bluetooth_enabled" | "bluetooth_enabled" | "bluetooth" => {
                SettingKey::BluetoothEnabled
            }
            "connectivity.vpn_enabled" | "vpn_enabled" | "vpn" => SettingKey::VpnEnabled,
            "connectivity.airplane_mode" | "airplane_mode" => SettingKey::AirplaneMode,

            "personalization.sound.volume" | "sound.volume" | "sound.audio_volume" | "volume" => SettingKey::AudioVolume,
            "personalization.sound.muted" | "sound.muted" | "sound.audio_muted" | "muted" => SettingKey::AudioMuted,
            "sound.default_sink" | "default_sink" => SettingKey::AudioDefaultSink,
            "sound.mic_volume" | "mic_volume" => SettingKey::MicVolume,
            "sound.mic_muted" | "mic_muted" => SettingKey::MicMuted,

            "displays.scale" | "display_scale" | "scale" => SettingKey::DisplayScale,
            "personalization.display.resolution_choice" | "displays.resolution" | "display_resolution" | "resolution" => {
                SettingKey::DisplayResolution
            }
            "displays.refresh_rate" | "display_refresh_rate" | "refresh_rate" => {
                SettingKey::DisplayRefreshRate
            }
            "displays.orientation" | "display_orientation" => SettingKey::DisplayOrientation,
            "personalization.display.brightness" | "displays.brightness" | "brightness" => SettingKey::DisplayBrightness,
            "personalization.display.auto_brightness" | "displays.auto_brightness" | "auto_brightness" => SettingKey::DisplayAutoBrightness,
            "personalization.display.night_shift" | "personalization.display.night_shift_mode" | "displays.night_shift" | "night_shift" => SettingKey::DisplayNightShift,
            "personalization.display.color_temp" | "displays.color_temp" | "color_temp" => SettingKey::DisplayColorTemp,

            "system.general.hostname" | "general.hostname" | "hostname" => SettingKey::Hostname,
            "system.language.primary_locale" | "general.locale" | "language.locale" | "locale" => SettingKey::Locale,
            "system.language.keyboard_layout" | "language.keyboard_layout" | "keyboard_layout" => SettingKey::KeyboardLayout,
            "system.date_time.timezone" | "general.timezone" | "date_time.timezone" | "timezone" => SettingKey::Timezone,
            "system.date_time.time_format_24h" | "system.general.time_24h" | "general.time_format_24h" | "time_format_24h" | "use_24h" => {
                SettingKey::TimeFormat24h
            }
            "system.date_time.ntp_sync" | "general.ntp_enabled" | "ntp_enabled" | "ntp" => SettingKey::NtpEnabled,
            "system.battery.mode" | "power.profile" | "power_profile" => SettingKey::PowerProfile,
            "general.screen_time_enabled" | "screen_time_enabled" => SettingKey::ScreenTimeEnabled,
            "system.general.auto_updates" | "general.auto_updates" | "auto_updates" => SettingKey::SystemAutoUpdates,

            "system.privacy.firewall_enabled" | "privacy.firewall_enabled" | "firewall_enabled" | "firewall" => {
                SettingKey::FirewallEnabled
            }
            "system.privacy.camera_enabled" | "privacy.camera_access" | "camera_access" => SettingKey::CameraAccess,
            "system.privacy.microphone_enabled" | "privacy.mic_access" | "mic_access" => SettingKey::MicAccess,
            "system.privacy.location_enabled" | "privacy.location_access" | "location_access" => SettingKey::LocationAccess,

            "system.accessibility.reduce_motion" | "accessibility.reduce_motion" => SettingKey::AccessibilityReduceMotion,
            "system.accessibility.increase_contrast" | "accessibility.increase_contrast" => SettingKey::AccessibilityIncreaseContrast,
            "system.accessibility.reduce_transparency" | "accessibility.reduce_transparency" => SettingKey::AccessibilityReduceTransparency,
            "system.accessibility.screen_reader" | "accessibility.screen_reader" => SettingKey::AccessibilityScreenReader,
            "system.accessibility.text_size" | "accessibility.text_size" => SettingKey::AccessibilityTextSize,

            "system.storage.empty_trash_auto" | "storage.empty_trash_auto" => SettingKey::StorageEmptyTrashAuto,

            "personalization.focus.mode" | "notifications.do_not_disturb" | "do_not_disturb" | "dnd" => SettingKey::DoNotDisturb,
            "personalization.notifications.enabled" | "notifications.banners" | "notification_banners" => SettingKey::NotificationBanners,
            "personalization.notifications.silence_during_sleep" | "notifications.sounds" | "notification_sounds" => SettingKey::NotificationSounds,

            _ => SettingKey::Custom(s.to_string()),
        })
    }
}

// ============================================================================
// Setting Values
// ============================================================================

/// Dynamic value representation for settings payloads.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<SettingValue>),
    Json(serde_json::Value),
}

impl SettingValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SettingValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            SettingValue::Int(i) => Some(*i),
            SettingValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            SettingValue::Float(f) => Some(*f),
            SettingValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            SettingValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            SettingValue::Json(j) => Some(j),
            _ => None,
        }
    }
}

impl Display for SettingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingValue::Bool(b) => write!(f, "{}", b),
            SettingValue::Int(i) => write!(f, "{}", i),
            SettingValue::Float(fl) => write!(f, "{}", fl),
            SettingValue::String(s) => write!(f, "{}", s),
            SettingValue::List(l) => {
                write!(f, "[")?;
                for (i, item) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            SettingValue::Json(j) => write!(f, "{}", j),
        }
    }
}

impl From<bool> for SettingValue {
    fn from(v: bool) -> Self {
        SettingValue::Bool(v)
    }
}

impl From<i64> for SettingValue {
    fn from(v: i64) -> Self {
        SettingValue::Int(v)
    }
}

impl From<i32> for SettingValue {
    fn from(v: i32) -> Self {
        SettingValue::Int(v as i64)
    }
}

impl From<u64> for SettingValue {
    fn from(v: u64) -> Self {
        SettingValue::Int(v as i64)
    }
}

impl From<u32> for SettingValue {
    fn from(v: u32) -> Self {
        SettingValue::Int(v as i64)
    }
}

impl From<f64> for SettingValue {
    fn from(v: f64) -> Self {
        SettingValue::Float(v)
    }
}

impl From<f32> for SettingValue {
    fn from(v: f32) -> Self {
        SettingValue::Float(v as f64)
    }
}

impl From<String> for SettingValue {
    fn from(v: String) -> Self {
        SettingValue::String(v)
    }
}

impl From<&str> for SettingValue {
    fn from(v: &str) -> Self {
        SettingValue::String(v.to_string())
    }
}

impl From<serde_json::Value> for SettingValue {
    fn from(v: serde_json::Value) -> Self {
        SettingValue::Json(v)
    }
}

// ============================================================================
// Lock State Concepts (Nix, Policy, Hardware)
// ============================================================================

/// Identifies the authoritative origin enforcing a lock on a setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum LockSource {
    /// Declaratively locked and managed by NixOS or Home Manager module.
    Nix,
    /// Enforced by administrator system policy or root file permissions.
    SystemPolicy,
    /// Hardware constraint or unchangeable platform limitation.
    Hardware,
    /// Controlled by an external system daemon or enterprise profile.
    ExternalService,
    /// Other designated lock source.
    Other(String),
}

impl Display for LockSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockSource::Nix => write!(f, "NixOS"),
            LockSource::SystemPolicy => write!(f, "System Policy"),
            LockSource::Hardware => write!(f, "Hardware"),
            LockSource::ExternalService => write!(f, "External Service"),
            LockSource::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Represents the lock state of a setting, indicating whether UI controls
/// should render as read-only and whether daemon mutations should be denied.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum LockState {
    /// The setting is mutable by the current user.
    Unlocked,
    /// The setting is read-only and locked by an authoritative source.
    Locked {
        source: LockSource,
        reason: String,
    },
}

impl Default for LockState {
    fn default() -> Self {
        LockState::Unlocked
    }
}

impl LockState {
    /// Create an unlocked state.
    pub fn unlocked() -> Self {
        LockState::Unlocked
    }

    /// Create a lock enforced by NixOS declarative configuration.
    pub fn nix(reason: impl Into<String>) -> Self {
        LockState::Locked {
            source: LockSource::Nix,
            reason: reason.into(),
        }
    }

    /// Create a lock enforced by system-wide administrator policy.
    pub fn policy(reason: impl Into<String>) -> Self {
        LockState::Locked {
            source: LockSource::SystemPolicy,
            reason: reason.into(),
        }
    }

    /// Create a lock enforced by hardware limitation.
    pub fn hardware(reason: impl Into<String>) -> Self {
        LockState::Locked {
            source: LockSource::Hardware,
            reason: reason.into(),
        }
    }

    /// Create a general lock with explicit source and explanation.
    pub fn locked(source: LockSource, reason: impl Into<String>) -> Self {
        LockState::Locked {
            source,
            reason: reason.into(),
        }
    }

    /// Returns `true` if the setting is locked.
    pub fn is_locked(&self) -> bool {
        matches!(self, LockState::Locked { .. })
    }

    /// Returns `true` if the setting is specifically locked by NixOS.
    pub fn is_nix_locked(&self) -> bool {
        matches!(self, LockState::Locked { source: LockSource::Nix, .. })
    }

    /// Returns the lock source, if locked.
    pub fn source(&self) -> Option<&LockSource> {
        match self {
            LockState::Locked { source, .. } => Some(source),
            LockState::Unlocked => None,
        }
    }

    /// Returns the human-readable lock reason or tooltip text, if locked.
    pub fn reason(&self) -> Option<&str> {
        match self {
            LockState::Locked { reason, .. } => Some(reason.as_str()),
            LockState::Unlocked => None,
        }
    }
}

impl Display for LockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockState::Unlocked => write!(f, "Unlocked"),
            LockState::Locked { source, reason } => {
                write!(f, "Locked ({source}): {reason}")
            }
        }
    }
}

// ============================================================================
// Setting Entry
// ============================================================================

/// Complete setting entry including current value, category, lock state, and metadata.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SettingEntry {
    pub key: SettingKey,
    pub value: SettingValue,
    pub lock_state: LockState,
    pub category: SettingCategory,
    pub description: Option<String>,
}

impl SettingEntry {
    /// Create a new unlocked setting entry with automatic category determination.
    pub fn new(key: SettingKey, value: impl Into<SettingValue>) -> Self {
        let category = key.category();
        Self {
            key,
            value: value.into(),
            lock_state: LockState::Unlocked,
            category,
            description: None,
        }
    }

    /// Builder method to specify lock state.
    pub fn with_lock(mut self, lock_state: LockState) -> Self {
        self.lock_state = lock_state;
        self
    }

    /// Builder method to specify category.
    pub fn with_category(mut self, category: SettingCategory) -> Self {
        self.category = category;
        self
    }

    /// Builder method to specify human-readable description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Check if this setting is locked.
    pub fn is_locked(&self) -> bool {
        self.lock_state.is_locked()
    }

    /// Check if this setting is locked by Nix.
    pub fn is_nix_locked(&self) -> bool {
        self.lock_state.is_nix_locked()
    }

    /// Human-readable explanation if locked.
    pub fn lock_reason(&self) -> Option<&str> {
        self.lock_state.reason()
    }
}

// ============================================================================
// Change Source & Denial Reasons
// ============================================================================

/// Origin of a setting mutation request or change event.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ChangeSource {
    /// Mutation originated from the Finick Settings GUI app.
    User,
    /// Mutation originated from command-line tooling (e.g. finickctl).
    Cli,
    /// Mutation originated from NixOS rebuild / declarative sync.
    Nix,
    /// Mutation synchronized from underlying system backend (e.g. nmcli, wpctl).
    SystemSync,
    /// Mutation originated internally from the daemon.
    Daemon,
    /// Mutation from an external integration.
    Other(String),
}

impl Display for ChangeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeSource::User => write!(f, "user"),
            ChangeSource::Cli => write!(f, "cli"),
            ChangeSource::Nix => write!(f, "nix"),
            ChangeSource::SystemSync => write!(f, "system_sync"),
            ChangeSource::Daemon => write!(f, "daemon"),
            ChangeSource::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Reason explaining why a set or apply operation was denied.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DenialReason {
    /// Setting is locked by an authoritative source (e.g. Nix).
    Locked(LockState),
    /// Proposed value is malformed or out of allowed bounds.
    InvalidValue(String),
    /// User or caller lacks required permissions.
    PermissionDenied(String),
    /// Underlying system backend command failed during application.
    ApplyFailed(String),
    /// Setting key is unrecognized.
    NotFound,
    /// Other reason.
    Other(String),
}

impl DenialReason {
    /// Convenience constructor for Nix lock rejection.
    pub fn locked_by_nix(reason: impl Into<String>) -> Self {
        DenialReason::Locked(LockState::nix(reason))
    }

    /// Returns `true` if the denial was due to a locked setting.
    pub fn is_locked(&self) -> bool {
        matches!(self, DenialReason::Locked(_))
    }

    /// Returns the associated `LockState` if locked.
    pub fn lock_state(&self) -> Option<&LockState> {
        match self {
            DenialReason::Locked(l) => Some(l),
            _ => None,
        }
    }
}

impl Display for DenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DenialReason::Locked(l) => write!(f, "Setting is locked: {}", l),
            DenialReason::InvalidValue(s) => write!(f, "Invalid value: {}", s),
            DenialReason::PermissionDenied(s) => write!(f, "Permission denied: {}", s),
            DenialReason::ApplyFailed(s) => write!(f, "Failed to apply setting: {}", s),
            DenialReason::NotFound => write!(f, "Setting not found"),
            DenialReason::Other(s) => write!(f, "{}", s),
        }
    }
}

// ============================================================================
// Alerts & Events
// ============================================================================

/// Severity level for system alerts broadcast by the daemon.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl Display for AlertLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertLevel::Info => write!(f, "INFO"),
            AlertLevel::Warning => write!(f, "WARN"),
            AlertLevel::Error => write!(f, "ERROR"),
            AlertLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Alert message emitted by the settings daemon for notifications or errors.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SettingsAlert {
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub key: Option<SettingKey>,
    pub timestamp: u64,
}

impl SettingsAlert {
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AlertLevel::Info, title, message)
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AlertLevel::Warning, title, message)
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AlertLevel::Error, title, message)
    }

    pub fn critical(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AlertLevel::Critical, title, message)
    }

    fn new(level: AlertLevel, title: impl Into<String>, message: impl Into<String>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            level,
            title: title.into(),
            message: message.into(),
            key: None,
            timestamp,
        }
    }

    pub fn with_key(mut self, key: SettingKey) -> Self {
        self.key = Some(key);
        self
    }
}

/// Real-time change event delivered to subscribed clients.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SettingsEvent {
    /// A setting's value was updated.
    Changed {
        key: SettingKey,
        value: SettingValue,
        source: ChangeSource,
    },
    /// A setting's lock state changed (e.g. after NixOS generation switch).
    LockChanged {
        key: SettingKey,
        lock_state: LockState,
    },
    /// An alert or notification issued by the settings daemon.
    Alert(SettingsAlert),
    /// All settings were reloaded from disk / Nix.
    Reloaded,
}

/// Subscription filter to selectively receive events.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct SubscriptionFilter {
    /// Restrict to settings belonging to these categories. `None` means all.
    pub categories: Option<Vec<SettingCategory>>,
    /// Restrict to specific setting keys. `None` means all.
    pub keys: Option<Vec<SettingKey>>,
    /// Whether to receive system alerts.
    pub include_alerts: bool,
}

impl SubscriptionFilter {
    /// Subscribe to all events, setting changes, and alerts.
    pub fn all() -> Self {
        Self {
            categories: None,
            keys: None,
            include_alerts: true,
        }
    }

    /// Subscribe only to a specific category.
    pub fn category(cat: SettingCategory) -> Self {
        Self {
            categories: Some(vec![cat]),
            keys: None,
            include_alerts: false,
        }
    }

    /// Subscribe to specific categories.
    pub fn categories(cats: impl IntoIterator<Item = SettingCategory>) -> Self {
        Self {
            categories: Some(cats.into_iter().collect()),
            keys: None,
            include_alerts: false,
        }
    }

    /// Subscribe to specific setting keys.
    pub fn keys(keys: impl IntoIterator<Item = SettingKey>) -> Self {
        Self {
            categories: None,
            keys: Some(keys.into_iter().collect()),
            include_alerts: false,
        }
    }

    /// Builder to enable or disable alerts.
    pub fn with_alerts(mut self, include_alerts: bool) -> Self {
        self.include_alerts = include_alerts;
        self
    }

    /// Test if a setting change matches this filter.
    pub fn matches_setting(&self, key: &SettingKey, category: &SettingCategory) -> bool {
        if let Some(ref cats) = self.categories {
            if !cats.contains(category) {
                return false;
            }
        }
        if let Some(ref keys) = self.keys {
            if !keys.contains(key) {
                return false;
            }
        }
        true
    }

    /// Test if alerts should be received.
    pub fn matches_alert(&self) -> bool {
        self.include_alerts
    }
}

// ============================================================================
// IPC Request & Response Contracts
// ============================================================================

/// Requests sent from clients (Settings app, Top Bar, Finickctl) to the Settings Daemon.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SettingsRequest {
    /// Health check ping. Daemon responds with `Pong`.
    Ping,

    /// Query a single setting by key.
    Get { key: SettingKey },

    /// Query all settings belonging to a category.
    GetCategory { category: SettingCategory },

    /// Query all settings known to the system.
    /// In response, daemon can stream individual `Setting` items (Index stream pattern).
    GetAll,

    /// Query lock states for settings.
    GetLocks { category: Option<SettingCategory> },

    /// Set a setting value (persists to state file).
    Set {
        key: SettingKey,
        value: SettingValue,
        source: ChangeSource,
    },

    /// Set a batch of settings.
    SetBatch {
        settings: Vec<(SettingKey, SettingValue)>,
        source: ChangeSource,
    },

    /// Apply an existing setting to the underlying system (e.g. hyprctl, nmcli).
    Apply { key: SettingKey },

    /// Apply all staged/current settings to the underlying system.
    ApplyAll,

    /// Convenience: Set value and immediately apply to system.
    SetAndApply {
        key: SettingKey,
        value: SettingValue,
        source: ChangeSource,
    },

    /// Convenience: Set batch and immediately apply to system.
    SetAndApplyBatch {
        settings: Vec<(SettingKey, SettingValue)>,
        source: ChangeSource,
    },

    /// Reset a setting to its system default value.
    Reset { key: SettingKey },

    /// Reset all settings (optionally restricted to a category) to defaults.
    ResetAll { category: Option<SettingCategory> },

    /// Subscribe to live setting changes and alerts.
    /// The daemon keeps the response channel open, streaming `SettingsEvent` items.
    Subscribe { filter: SubscriptionFilter },

    /// Force daemon to reload state from disk / Nix declarative config.
    ReloadFromDisk,
}

/// Responses emitted by the Settings Daemon back to the client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SettingsResponse {
    /// Command succeeded without specific payload.
    Ok,

    /// Single setting entry returned.
    Setting(SettingEntry),

    /// Batch setting entries returned.
    SettingsList(Vec<SettingEntry>),

    /// Lock state response for a key.
    LockState {
        key: SettingKey,
        lock_state: LockState,
    },

    /// Requested setting was not found.
    NotFound { key: SettingKey },

    /// Mutation or apply request was rejected (e.g. locked by Nix).
    Rejected {
        key: SettingKey,
        reason: DenialReason,
    },

    /// Live event broadcast (for subscribers).
    Event(SettingsEvent),

    /// Alert broadcast (for subscribers).
    Alert(SettingsAlert),

    /// Health check response.
    Pong,

    /// General daemon error message.
    Error(String),
}

// ============================================================================
// High-level Client Helper Functions (Re-using existing IPC stream patterns)
// ============================================================================

/// Send a raw `SettingsRequest` to the settings daemon socket and handle responses.
pub fn send_settings_request<H>(
    socket: impl Into<PathBuf> + Display,
    req: &SettingsRequest,
    handler: Option<H>,
) -> std::io::Result<()>
where
    H: Fn(SettingsResponse) + Send + 'static,
{
    send_command(socket, req, handler)
}

fn collect_responses(socket: impl Into<PathBuf> + Display, req: &SettingsRequest) -> std::io::Result<mpsc::Receiver<SettingsResponse>> {
    let (tx, rx) = mpsc::channel();
    send_settings_request(socket, req, Some(move |res| { let _ = tx.send(res); }))?;
    Ok(rx)
}

fn expect_ok(rx: mpsc::Receiver<SettingsResponse>) -> std::io::Result<Result<(), DenialReason>> {
    for res in rx {
        match res {
            SettingsResponse::Ok => return Ok(Ok(())),
            SettingsResponse::Rejected { reason, .. } => return Ok(Err(reason)),
            SettingsResponse::Error(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            _ => {}
        }
    }
    Ok(Ok(()))
}

/// Fetch a single setting entry from the daemon.
pub fn get_setting(socket: impl Into<PathBuf> + Display, key: SettingKey) -> std::io::Result<Option<SettingEntry>> {
    let rx = collect_responses(socket, &SettingsRequest::Get { key })?;
    for res in rx {
        match res {
            SettingsResponse::Setting(entry) => return Ok(Some(entry)),
            SettingsResponse::NotFound { .. } => return Ok(None),
            SettingsResponse::Error(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            _ => {}
        }
    }
    Ok(None)
}

/// Fetch all settings from the daemon, aggregating streamed responses.
pub fn get_all_settings(socket: impl Into<PathBuf> + Display) -> std::io::Result<Vec<SettingEntry>> {
    collect_entries(socket, SettingsRequest::GetAll)
}

/// Fetch all settings belonging to a specific category.
pub fn get_category_settings(socket: impl Into<PathBuf> + Display, category: SettingCategory) -> std::io::Result<Vec<SettingEntry>> {
    collect_entries(socket, SettingsRequest::GetCategory { category })
}

fn collect_entries(socket: impl Into<PathBuf> + Display, req: SettingsRequest) -> std::io::Result<Vec<SettingEntry>> {
    let rx = collect_responses(socket, &req)?;
    let mut entries = Vec::new();
    for res in rx {
        match res {
            SettingsResponse::Setting(entry) => entries.push(entry),
            SettingsResponse::SettingsList(list) => entries.extend(list),
            SettingsResponse::Error(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            _ => {}
        }
    }
    Ok(entries)
}

/// Set a setting value via the daemon. Returns `Ok(Ok(()))` on success,
/// or `Ok(Err(DenialReason))` if rejected (e.g. locked by Nix).
pub fn set_setting(socket: impl Into<PathBuf> + Display, key: SettingKey, value: impl Into<SettingValue>) -> std::io::Result<Result<(), DenialReason>> {
    let req = SettingsRequest::Set { key, value: value.into(), source: ChangeSource::User };
    expect_ok(collect_responses(socket, &req)?)
}

/// Apply a setting to the underlying system via the daemon.
pub fn apply_setting(socket: impl Into<PathBuf> + Display, key: SettingKey) -> std::io::Result<Result<(), DenialReason>> {
    expect_ok(collect_responses(socket, &SettingsRequest::Apply { key })?)
}

/// Set and immediately apply a setting. Returns `Ok(Ok(()))` or `Ok(Err(DenialReason))`.
pub fn set_and_apply(socket: impl Into<PathBuf> + Display, key: SettingKey, value: impl Into<SettingValue>) -> std::io::Result<Result<(), DenialReason>> {
    let req = SettingsRequest::SetAndApply { key, value: value.into(), source: ChangeSource::User };
    expect_ok(collect_responses(socket, &req)?)
}

/// Subscribe to live setting changes and alerts from the daemon.
/// Calls `handler` continuously as events arrive over the stream.
pub fn subscribe_settings<H>(
    socket: impl Into<PathBuf> + Display,
    filter: SubscriptionFilter,
    handler: H,
) -> std::io::Result<()>
where
    H: Fn(SettingsEvent) + Send + 'static,
{
    send_settings_request(
        socket,
        &SettingsRequest::Subscribe { filter },
        Some(move |res| match res {
            SettingsResponse::Event(evt) => handler(evt),
            SettingsResponse::Alert(alert) => handler(SettingsEvent::Alert(alert)),
            _ => {}
        }),
    )
}

/// Subscribe to events and stream them to an asynchronous tokio channel.
pub fn subscribe_channel(
    socket: impl Into<PathBuf> + Display,
    filter: SubscriptionFilter,
) -> std::io::Result<tokio::sync::mpsc::UnboundedReceiver<SettingsEvent>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let socket_str = socket.to_string();
    std::thread::spawn(move || {
        let _ = subscribe_settings(&socket_str, filter, move |evt| {
            let _ = tx.send(evt);
        });
    });
    Ok(rx)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::start_server;
    use std::{sync::mpsc::Sender, thread, time::Duration};

    #[test]
    fn test_lock_state_nix() {
        let lock = LockState::nix("Managed by NixOS configuration.nix");
        assert!(lock.is_locked());
        assert!(lock.is_nix_locked());
        assert_eq!(lock.source(), Some(&LockSource::Nix));
        assert_eq!(lock.reason(), Some("Managed by NixOS configuration.nix"));

        let unlocked = LockState::unlocked();
        assert!(!unlocked.is_locked());
        assert!(!unlocked.is_nix_locked());
        assert_eq!(unlocked.source(), None);
        assert_eq!(unlocked.reason(), None);
    }

    #[test]
    fn test_setting_entry_builders() {
        let entry = SettingEntry::new(SettingKey::ThemeMode, "dark")
            .with_lock(LockState::nix("Locked by /etc/nixos/settings.nix"))
            .with_description("Application theme mode");

        assert_eq!(entry.key, SettingKey::ThemeMode);
        assert_eq!(entry.category, SettingCategory::Appearance);
        assert_eq!(entry.value.as_str(), Some("dark"));
        assert!(entry.is_locked());
        assert!(entry.is_nix_locked());
        assert_eq!(entry.lock_reason(), Some("Locked by /etc/nixos/settings.nix"));
    }

    #[test]
    fn test_setting_key_parsing() {
        let k1: SettingKey = "appearance.theme_mode".parse().unwrap();
        assert_eq!(k1, SettingKey::ThemeMode);

        let k2: SettingKey = "volume".parse().unwrap();
        assert_eq!(k2, SettingKey::AudioVolume);

        let k3: SettingKey = "custom.my_key".parse().unwrap();
        assert_eq!(k3, SettingKey::Custom("custom.my_key".to_string()));
        assert_eq!(k3.category(), SettingCategory::Other);
    }

    #[test]
    fn test_setting_value_conversions() {
        let b = SettingValue::from(true);
        assert_eq!(b.as_bool(), Some(true));

        let i = SettingValue::from(42);
        assert_eq!(i.as_i64(), Some(42));

        let f = SettingValue::from(3.14);
        assert!((f.as_f64().unwrap() - 3.14).abs() < 1e-6);

        let s = SettingValue::from("coral");
        assert_eq!(s.as_str(), Some("coral"));
    }

    #[test]
    fn test_subscription_filter() {
        let filter = SubscriptionFilter::category(SettingCategory::Appearance);
        assert!(filter.matches_setting(&SettingKey::ThemeMode, &SettingCategory::Appearance));
        assert!(!filter.matches_setting(&SettingKey::AudioVolume, &SettingCategory::Sound));

        let filter_all = SubscriptionFilter::all();
        assert!(filter_all.matches_setting(&SettingKey::AudioVolume, &SettingCategory::Sound));
        assert!(filter_all.matches_alert());
    }

    #[test]
    fn test_denial_reason() {
        let denial = DenialReason::locked_by_nix("Managed by Nix module");
        assert!(denial.is_locked());
        assert!(denial.lock_state().unwrap().is_nix_locked());
    }

    #[test]
    fn test_ipc_settings_flow() {
        let socket = "test-settings-ipc";

        // Spawn mock settings server
        let _server = thread::spawn(move || {
            let _ = start_server(
                socket,
                |req: SettingsRequest, sender: Sender<SettingsResponse>| match req {
                    SettingsRequest::Ping => {
                        let _ = sender.send(SettingsResponse::Pong);
                    }
                    SettingsRequest::Get { key } => {
                        if key == SettingKey::ThemeMode {
                            let entry = SettingEntry::new(SettingKey::ThemeMode, "dark");
                            let _ = sender.send(SettingsResponse::Setting(entry));
                        } else {
                            let _ = sender.send(SettingsResponse::NotFound { key });
                        }
                    }
                    SettingsRequest::GetAll => {
                        // Stream multiple settings back (Index stream pattern)
                        let entries = vec![
                            SettingEntry::new(SettingKey::ThemeMode, "dark"),
                            SettingEntry::new(SettingKey::AudioVolume, 75),
                            SettingEntry::new(SettingKey::WifiEnabled, true)
                                .with_lock(LockState::nix("Locked by network.enable = true")),
                        ];
                        for entry in entries {
                            let _ = sender.send(SettingsResponse::Setting(entry));
                        }
                    }
                    SettingsRequest::SetAndApply { key, value, .. } => {
                        if key == SettingKey::WifiEnabled {
                            let _ = sender.send(SettingsResponse::Rejected {
                                key,
                                reason: DenialReason::locked_by_nix("Cannot change locked Wi-Fi"),
                            });
                        } else {
                            assert_eq!(value.as_str(), Some("light"));
                            let _ = sender.send(SettingsResponse::Ok);
                        }
                    }
                    SettingsRequest::Subscribe { .. } => {
                        // Stream events
                        let _ = sender.send(SettingsResponse::Event(SettingsEvent::Changed {
                            key: SettingKey::ThemeMode,
                            value: SettingValue::from("auto"),
                            source: ChangeSource::Daemon,
                        }));
                        let _ = sender.send(SettingsResponse::Alert(SettingsAlert::info(
                            "Battery Low",
                            "15% remaining",
                        )));
                    }
                    _ => {}
                },
            );
        });

        // Give server time to bind
        thread::sleep(Duration::from_millis(200));

        // Test Get
        let setting = get_setting(socket, SettingKey::ThemeMode).unwrap();
        assert!(setting.is_some());
        let entry = setting.unwrap();
        assert_eq!(entry.value.as_str(), Some("dark"));

        // Test GetAll (streamed aggregation)
        let all = get_all_settings(socket).unwrap();
        assert_eq!(all.len(), 3);
        assert!(all.iter().any(|e| e.key == SettingKey::WifiEnabled && e.is_nix_locked()));

        // Test SetAndApply on locked setting
        let res = set_and_apply(socket, SettingKey::WifiEnabled, false).unwrap();
        assert!(res.is_err());
        let denial = res.unwrap_err();
        assert!(denial.is_locked());
        assert!(denial.lock_state().unwrap().is_nix_locked());

        // Test SetAndApply on unlocked setting
        let res_ok = set_and_apply(socket, SettingKey::ThemeMode, "light").unwrap();
        assert!(res_ok.is_ok());

        // Test Subscribe stream
        let (tx, rx) = mpsc::channel();
        let _ = subscribe_settings(socket, SubscriptionFilter::all(), move |evt| {
            let _ = tx.send(evt);
        });

        let evts: Vec<SettingsEvent> = rx.into_iter().collect();
        assert_eq!(evts.len(), 2);
        match &evts[0] {
            SettingsEvent::Changed { key, value, .. } => {
                assert_eq!(key, &SettingKey::ThemeMode);
                assert_eq!(value.as_str(), Some("auto"));
            }
            _ => panic!("Expected Changed event"),
        }
        match &evts[1] {
            SettingsEvent::Alert(alert) => {
                assert_eq!(alert.title, "Battery Low");
            }
            _ => panic!("Expected Alert event"),
        }
    }

    #[test]
    fn test_live_finickd() {
        if std::path::Path::new("/tmp/finickd.sock").exists() {
            let res = get_setting("finickd", SettingKey::ThemeMode);
            assert!(res.is_ok(), "Failed to query live daemon: {:?}", res);
            let entry = res.unwrap();
            assert!(entry.is_some(), "ThemeMode should be present in live daemon");
            println!("Live daemon response: {:?}", entry);
        }
    }
}

