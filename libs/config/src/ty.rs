use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, ops::Deref, path::PathBuf};

/// Maps accent name or hex to normalized 6-digit hex (without #), e.g. "Indigo" -> "5B5FE9".
pub fn accent_to_hex(s: &str) -> String {
    match s.trim().trim_start_matches('#').to_lowercase().as_str() {
        "indigo" => "5B5FE9".to_string(),
        "coral" => "FF6952".to_string(),
        "amber" => "E3A23D".to_string(),
        "teal" => "2CA6A0".to_string(),
        "rose" => "E85A88".to_string(),
        "slate" => "7B7F87".to_string(),
        other if (other.len() == 6 || other.len() == 8) && other.chars().all(|c| c.is_ascii_hexdigit()) => other.to_uppercase(),
        _ => "5B5FE9".to_string(),
    }
}

/// Finick application and service identities.
#[derive(strum::Display, strum::EnumString, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum App {
    Files,
    IndexService,
    Settings,
    Finickd,
    TopBar,
    Other(String),
}

impl App {
    /// Returns the string representation of the app identity.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Files => "Files",
            Self::IndexService => "IndexService",
            Self::Settings => "Settings",
            Self::Finickd => "Finickd",
            Self::TopBar => "TopBar",
            Self::Other(name) => name.as_str(),
        }
    }

    /// Returns the default configuration file name (e.g., "Settings.json").
    pub fn config_file_name(&self) -> String {
        format!("{}.json", self)
    }
}

impl From<App> for PathBuf {
    fn from(app: App) -> Self {
        PathBuf::from(app.to_string())
    }
}

impl From<&App> for PathBuf {
    fn from(app: &App) -> Self {
        PathBuf::from(app.to_string())
    }
}

/// Reason why a setting key is locked / read-only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum LockReason {
    /// Restricted declaratively by Nix / NixOS configuration or Flake definition.
    #[strum(serialize = "Managed declaratively by Nix / NixOS")]
    NixManaged,
    /// Restricted by system administrator or security policy.
    #[strum(serialize = "Restricted by system policy")]
    SystemPolicy,
    /// The backing configuration file is on a read-only filesystem.
    #[strum(serialize = "Configuration filesystem is read-only")]
    ReadOnlyFileSystem,
    /// Custom restriction explanation.
    #[strum(serialize = "{0}")]
    #[serde(untagged)]
    Custom(String),
}

impl LockReason {
    /// Returns true if this restriction is originating from Nix configuration.
    pub fn is_nix(&self) -> bool {
        matches!(self, Self::NixManaged)
    }
}

/// Explicit lock metadata for a setting.
///
/// In Finick, settings can be locked by declarative system definitions (such as Nix Flakes
/// or NixOS modules) or administrator policy. When locked, user interfaces should display
/// the setting as read-only (with lock indicators / tooltips) and backends must reject
/// modification attempts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SettingLock {
    /// Whether this setting is currently locked (read-only).
    pub locked: bool,
    /// The categorized reason for the restriction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<LockReason>,
    /// Human-readable explanation for why the setting is restricted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Originating configuration source (e.g. "/etc/nixos/configuration.nix").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl SettingLock {
    /// Creates an unlocked (read-write) metadata instance.
    pub fn unlocked() -> Self {
        Self {
            locked: false,
            reason: None,
            message: None,
            source: None,
        }
    }

    /// Creates a lock instance marking the key as managed and locked by Nix.
    pub fn nix_locked(message: impl Into<String>) -> Self {
        Self {
            locked: true,
            reason: Some(LockReason::NixManaged),
            message: Some(message.into()),
            source: Some("nix".to_string()),
        }
    }

    /// Creates a lock instance with an explicit reason and message.
    pub fn locked(reason: LockReason, message: impl Into<String>) -> Self {
        Self {
            locked: true,
            reason: Some(reason),
            message: Some(message.into()),
            source: None,
        }
    }

    /// Returns whether this key is currently locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Returns whether this key is currently unlocked and editable.
    pub fn is_unlocked(&self) -> bool {
        !self.locked
    }

    /// Returns whether this key is locked specifically due to Nix management.
    pub fn is_nix_locked(&self) -> bool {
        self.locked && matches!(self.reason, Some(LockReason::NixManaged))
    }

    /// Marks this metadata as locked with the given reason and optional message.
    pub fn lock(&mut self, reason: LockReason, message: Option<String>) {
        self.locked = true;
        self.reason = Some(reason);
        self.message = message;
    }

    /// Unlocks this setting.
    pub fn unlock(&mut self) {
        self.locked = false;
        self.reason = None;
        self.message = None;
        self.source = None;
    }
}

/// Error returned when an operation attempts to mutate a locked setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingLockedError {
    pub key: Option<String>,
    pub lock: SettingLock,
}

impl fmt::Display for SettingLockedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_str = self.key.as_deref().unwrap_or("Setting");
        let reason_str = self
            .lock
            .reason
            .as_ref()
            .map(|r| r.to_string())
            .unwrap_or_else(|| "read-only".to_string());
        if let Some(msg) = &self.lock.message {
            write!(f, "Cannot modify {}: {} ({})", key_str, reason_str, msg)
        } else {
            write!(f, "Cannot modify {}: {}", key_str, reason_str)
        }
    }
}

impl std::error::Error for SettingLockedError {}

/// Wrapper type for a setting value that attaches explicit lock metadata.
///
/// Implements `Deref<Target = T>` for transparent read access, while guarding
/// mutations with `set` or providing `get_mut` that returns `None` if locked.
///
/// Serialization is optimized:
/// - When unlocked, it serializes directly as the value `T` for clean JSON payloads.
/// - When locked, it serializes as an object containing both `value` and `lock`.
/// - Deserialization gracefully accepts either plain `T` or `{ value, lock }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting<T> {
    pub value: T,
    pub lock: SettingLock,
}

impl<T> Setting<T> {
    /// Creates a new unlocked setting.
    pub fn new(value: T) -> Self {
        Self {
            value,
            lock: SettingLock::unlocked(),
        }
    }

    /// Creates a new locked setting.
    pub fn new_locked(value: T, reason: LockReason, message: impl Into<String>) -> Self {
        Self {
            value,
            lock: SettingLock::locked(reason, message),
        }
    }

    /// Creates a new setting locked by Nix configuration.
    pub fn new_nix_locked(value: T, message: impl Into<String>) -> Self {
        Self {
            value,
            lock: SettingLock::nix_locked(message),
        }
    }

    /// Returns true if this setting is read-only / locked.
    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }

    /// Returns true if this setting is unlocked and editable.
    pub fn is_unlocked(&self) -> bool {
        self.lock.is_unlocked()
    }

    /// Returns true if this setting is restricted by Nix.
    pub fn is_nix_locked(&self) -> bool {
        self.lock.is_nix_locked()
    }

    /// Returns the lock reason, if locked.
    pub fn lock_reason(&self) -> Option<&LockReason> {
        self.lock.reason.as_ref()
    }

    /// Returns the lock message, if present.
    pub fn lock_message(&self) -> Option<&str> {
        self.lock.message.as_deref()
    }

    /// Locks this setting with an explicit reason.
    pub fn lock_with(&mut self, reason: LockReason, message: impl Into<String>) {
        self.lock.lock(reason, Some(message.into()));
    }

    /// Locks this setting specifically as Nix-managed.
    pub fn lock_nix(&mut self, message: impl Into<String>) {
        self.lock = SettingLock::nix_locked(message);
    }

    /// Unlocks this setting.
    pub fn unlock(&mut self) {
        self.lock.unlock();
    }

    /// Returns an immutable reference to the setting value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the setting value if unlocked, or `None` if locked.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.is_locked() {
            None
        } else {
            Some(&mut self.value)
        }
    }

    /// Safely updates the value if unlocked, or returns `SettingLockedError` if locked.
    pub fn set(&mut self, new_value: T) -> Result<(), SettingLockedError> {
        if self.is_locked() {
            Err(SettingLockedError {
                key: None,
                lock: self.lock.clone(),
            })
        } else {
            self.value = new_value;
            Ok(())
        }
    }

    /// Forcibly updates the value regardless of lock state.
    pub fn set_force(&mut self, new_value: T) {
        self.value = new_value;
    }

    /// Consumes the wrapper and returns the inner value.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Maps the inner value to a new type while preserving the lock state.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Setting<U> {
        Setting {
            value: f(self.value),
            lock: self.lock,
        }
    }
}

impl<T> Deref for Setting<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> From<T> for Setting<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Default> Default for Setting<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: fmt::Display> fmt::Display for Setting<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: Serialize> Serialize for Setting<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.lock.is_locked() {
            #[derive(Serialize)]
            struct LockedSettingRef<'a, V> {
                value: &'a V,
                lock: &'a SettingLock,
            }
            LockedSettingRef {
                value: &self.value,
                lock: &self.lock,
            }
            .serialize(serializer)
        } else {
            self.value.serialize(serializer)
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Setting<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SettingRepr<V> {
            Detailed {
                value: V,
                #[serde(default)]
                lock: SettingLock,
            },
            Simple(V),
        }

        match SettingRepr::<T>::deserialize(deserializer)? {
            SettingRepr::Detailed { value, lock } => Ok(Setting { value, lock }),
            SettingRepr::Simple(value) => Ok(Setting {
                value,
                lock: SettingLock::unlocked(),
            }),
        }
    }
}

/// Registry of key-level setting locks.
///
/// Useful for managing lock metadata across hierarchical paths (e.g. "connectivity.wifi.enabled")
/// without requiring every field in arbitrary payloads to be individually wrapped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SettingLockRegistry {
    #[serde(default)]
    pub locks: HashMap<String, SettingLock>,
}

impl SettingLockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.locks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.locks.len()
    }

    pub fn is_locked(&self, key: &str) -> bool {
        self.locks.get(key).map(|l| l.is_locked()).unwrap_or(false)
    }

    pub fn is_nix_locked(&self, key: &str) -> bool {
        self.locks.get(key).map(|l| l.is_nix_locked()).unwrap_or(false)
    }

    pub fn get(&self, key: &str) -> Option<&SettingLock> {
        self.locks.get(key)
    }

    pub fn lock(&mut self, key: impl Into<String>, lock: SettingLock) {
        self.locks.insert(key.into(), lock);
    }

    pub fn lock_nix(&mut self, key: impl Into<String>, message: impl Into<String>) {
        self.locks.insert(key.into(), SettingLock::nix_locked(message));
    }

    pub fn unlock(&mut self, key: &str) -> bool {
        self.locks.remove(key).is_some()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.locks.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &SettingLock)> {
        self.locks.iter()
    }
}

// ---------------------------------------------------------------------------
// Settings Schemas & Categories
// ---------------------------------------------------------------------------

/// UI Theme Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, strum::Display, strum::EnumString)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Auto,
    Light,
    Dark,
}

// Default helper functions for serde field attributes
fn default_true_setting() -> Setting<bool> {
    Setting::new(true)
}
fn default_false_setting() -> Setting<bool> {
    Setting::new(false)
}
fn default_theme_mode() -> Setting<ThemeMode> {
    Setting::new(ThemeMode::Auto)
}
fn default_accent_color() -> Setting<String> {
    Setting::new("Indigo".to_string())
}
fn default_icon_size_pref() -> Setting<usize> {
    Setting::new(1)
}
fn default_gaps_in() -> Setting<u32> {
    Setting::new(5)
}
fn default_gaps_out() -> Setting<u32> {
    Setting::new(10)
}
fn default_border_size() -> Setting<u32> {
    Setting::new(2)
}
fn default_brightness() -> Setting<f32> {
    Setting::new(72.0)
}
fn default_color_temp() -> Setting<f32> {
    Setting::new(30.0)
}
fn default_res_choice() -> Setting<String> {
    Setting::new("Default".to_string())
}
fn default_volume() -> Setting<u32> {
    Setting::new(65)
}
fn default_focus_mode() -> Setting<String> {
    Setting::new("off".to_string())
}
fn default_timezone() -> Setting<String> {
    Setting::new("UTC".to_string())
}
fn default_battery_mode() -> Setting<String> {
    Setting::new("balanced".to_string())
}
fn default_text_size() -> Setting<f32> {
    Setting::new(40.0)
}
fn default_locale() -> Setting<String> {
    Setting::new("en_US.UTF-8".to_string())
}
fn default_keyboard_layout() -> Setting<String> {
    Setting::new("us".to_string())
}

/// Appearance & desktop configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceSettings {
    #[serde(default = "default_theme_mode")]
    pub mode: Setting<ThemeMode>,
    #[serde(default = "default_accent_color")]
    pub accent_color: Setting<String>,
    #[serde(default = "default_wallpaper")]
    pub wallpaper: Setting<String>,
    #[serde(default)]
    pub wallpaper_idx: Setting<usize>,
    #[serde(default)]
    pub scrollbar_pref: Setting<usize>,
    #[serde(default = "default_icon_size_pref")]
    pub icon_size_pref: Setting<usize>,
    // Hyprland integration
    #[serde(default = "default_gaps_in")]
    pub gaps_in: Setting<u32>,
    #[serde(default = "default_gaps_out")]
    pub gaps_out: Setting<u32>,
    #[serde(default = "default_border_size")]
    pub border_size: Setting<u32>,
}

fn default_wallpaper() -> Setting<String> {
    Setting::new("#1e1e2e".to_string())
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            mode: default_theme_mode(),
            accent_color: default_accent_color(),
            wallpaper: default_wallpaper(),
            wallpaper_idx: Setting::new(0),
            scrollbar_pref: Setting::new(0),
            icon_size_pref: default_icon_size_pref(),
            gaps_in: default_gaps_in(),
            gaps_out: default_gaps_out(),
            border_size: default_border_size(),
        }
    }
}

/// Wi-Fi configuration settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiSettings {
    #[serde(default = "default_true_setting")]
    pub enabled: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub ask_to_join: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub limit_tracking: Setting<bool>,
}

impl Default for WifiSettings {
    fn default() -> Self {
        Self {
            enabled: Setting::new(true),
            ask_to_join: Setting::new(true),
            limit_tracking: Setting::new(true),
        }
    }
}

/// Bluetooth configuration settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothSettings {
    #[serde(default = "default_true_setting")]
    pub enabled: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub discoverable: Setting<bool>,
}

impl Default for BluetoothSettings {
    fn default() -> Self {
        Self {
            enabled: Setting::new(true),
            discoverable: Setting::new(true),
        }
    }
}

/// Display configuration settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplaySettings {
    #[serde(default = "default_brightness")]
    pub brightness: Setting<f32>,
    #[serde(default = "default_true_setting")]
    pub auto_brightness: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub true_tone: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub night_shift: Setting<bool>,
    #[serde(default)]
    pub night_shift_mode: Setting<usize>,
    #[serde(default = "default_color_temp")]
    pub color_temp: Setting<f32>,
    #[serde(default = "default_res_choice")]
    pub resolution_choice: Setting<String>,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            brightness: Setting::new(72.0),
            auto_brightness: Setting::new(true),
            true_tone: Setting::new(true),
            night_shift: Setting::new(false),
            night_shift_mode: Setting::new(0),
            color_temp: Setting::new(30.0),
            resolution_choice: Setting::new("Default".to_string()),
        }
    }
}

/// Sound and volume configuration settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundSettings {
    #[serde(default = "default_volume")]
    pub volume: Setting<u32>,
    #[serde(default = "default_false_setting")]
    pub muted: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub feedback_on_change: Setting<bool>,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            volume: Setting::new(65),
            muted: Setting::new(false),
            feedback_on_change: Setting::new(true),
        }
    }
}

/// Focus and Do Not Disturb settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusSettings {
    #[serde(default = "default_focus_mode")]
    pub mode: Setting<String>,
    #[serde(default = "default_true_setting")]
    pub work_schedule: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub sleep_schedule: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub share_across_devices: Setting<bool>,
}

impl Default for FocusSettings {
    fn default() -> Self {
        Self {
            mode: Setting::new("off".to_string()),
            work_schedule: Setting::new(true),
            sleep_schedule: Setting::new(true),
            share_across_devices: Setting::new(true),
        }
    }
}

/// Notification preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationSettings {
    #[serde(default = "default_true_setting")]
    pub enabled: Setting<bool>,
    #[serde(default)]
    pub style: Setting<usize>,
    #[serde(default = "default_true_setting")]
    pub silence_during_sleep: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub messages: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub calendar: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub mail: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub photos: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub weather: Setting<bool>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: Setting::new(true),
            style: Setting::new(0),
            silence_during_sleep: Setting::new(true),
            messages: Setting::new(true),
            calendar: Setting::new(true),
            mail: Setting::new(false),
            photos: Setting::new(true),
            weather: Setting::new(false),
        }
    }
}

/// General operating system settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default = "default_true_setting")]
    pub time_24h: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub auto_updates: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub screen_time_enabled: Setting<bool>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            time_24h: Setting::new(true),
            auto_updates: Setting::new(true),
            screen_time_enabled: Setting::new(true),
        }
    }
}

/// Date and time settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateTimeSettings {
    #[serde(default = "default_true_setting")]
    pub automatic_timezone: Setting<bool>,
    #[serde(default = "default_timezone")]
    pub timezone: Setting<String>,
    #[serde(default = "default_true_setting")]
    pub ntp_sync: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub time_format_24h: Setting<bool>,
}

impl Default for DateTimeSettings {
    fn default() -> Self {
        Self {
            automatic_timezone: Setting::new(true),
            timezone: Setting::new("UTC".to_string()),
            ntp_sync: Setting::new(true),
            time_format_24h: Setting::new(true),
        }
    }
}

/// Storage preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageSettings {
    #[serde(default = "default_true_setting")]
    pub empty_trash_auto: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub save_to_cloud: Setting<bool>,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            empty_trash_auto: Setting::new(true),
            save_to_cloud: Setting::new(false),
        }
    }
}

/// Power and battery preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatterySettings {
    #[serde(default = "default_battery_mode")]
    pub mode: Setting<String>,
    #[serde(default = "default_true_setting")]
    pub optimized_charging: Setting<bool>,
}

impl Default for BatterySettings {
    fn default() -> Self {
        Self {
            mode: Setting::new("balanced".to_string()),
            optimized_charging: Setting::new(true),
        }
    }
}

/// Accessibility preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    #[serde(default = "default_text_size")]
    pub text_size: Setting<f32>,
    #[serde(default = "default_false_setting")]
    pub reduce_motion: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub increase_contrast: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub reduce_transparency: Setting<bool>,
    #[serde(default = "default_false_setting")]
    pub screen_reader: Setting<bool>,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            text_size: Setting::new(40.0),
            reduce_motion: Setting::new(false),
            increase_contrast: Setting::new(false),
            reduce_transparency: Setting::new(false),
            screen_reader: Setting::new(false),
        }
    }
}

/// Privacy and security settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacySettings {
    #[serde(default = "default_true_setting")]
    pub camera_enabled: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub microphone_enabled: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub location_enabled: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub app_sandboxing: Setting<bool>,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            camera_enabled: Setting::new(true),
            microphone_enabled: Setting::new(true),
            location_enabled: Setting::new(true),
            app_sandboxing: Setting::new(true),
        }
    }
}

/// Regional language & input settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageSettings {
    #[serde(default = "default_locale")]
    pub primary_locale: Setting<String>,
    #[serde(default = "default_keyboard_layout")]
    pub keyboard_layout: Setting<String>,
    #[serde(default = "default_true_setting")]
    pub spell_check: Setting<bool>,
    #[serde(default = "default_true_setting")]
    pub autocorrect: Setting<bool>,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            primary_locale: Setting::new("en_US.UTF-8".to_string()),
            keyboard_layout: Setting::new("us".to_string()),
            spell_check: Setting::new(true),
            autocorrect: Setting::new(true),
        }
    }
}

/// Connectivity group configuration (Wi-Fi, Bluetooth).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConnectivityConfig {
    #[serde(default)]
    pub wifi: WifiSettings,
    #[serde(default)]
    pub bluetooth: BluetoothSettings,
}

/// Personalization group configuration (Appearance, Display, Sound, Focus, Notifications).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalizationConfig {
    #[serde(default)]
    pub appearance: AppearanceSettings,
    #[serde(default)]
    pub display: DisplaySettings,
    #[serde(default)]
    pub sound: SoundSettings,
    #[serde(default)]
    pub focus: FocusSettings,
    #[serde(default)]
    pub notifications: NotificationSettings,
}

/// System group configuration (General, Date & Time, Storage, Battery, Accessibility, Privacy, Language).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SystemConfig {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub date_time: DateTimeSettings,
    #[serde(default)]
    pub storage: StorageSettings,
    #[serde(default)]
    pub battery: BatterySettings,
    #[serde(default)]
    pub accessibility: AccessibilitySettings,
    #[serde(default)]
    pub privacy: PrivacySettings,
    #[serde(default)]
    pub language: LanguageSettings,
}

/// Current canonical schema version for SettingsPayload.
pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    CURRENT_SETTINGS_SCHEMA_VERSION
}

/// Canonical root Settings configuration payload.
///
/// Encapsulates all categories managed by Finick Settings and Settings Daemon,
/// alongside schema versioning and key-level lock metadata for Nix-managed keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsPayload {
    /// Schema version for forward/backward compatibility migrations.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Connectivity settings (Wi-Fi, Bluetooth).
    #[serde(default)]
    pub connectivity: ConnectivityConfig,
    /// Personalization settings (Appearance, Display, Sound, Focus, Notifications).
    #[serde(default)]
    pub personalization: PersonalizationConfig,
    /// System settings (General, Date & Time, Storage, Battery, Accessibility, Privacy, Language).
    #[serde(default)]
    pub system: SystemConfig,
    /// Key-level lock metadata registry.
    #[serde(default, skip_serializing_if = "SettingLockRegistry::is_empty")]
    pub locks: SettingLockRegistry,
    /// Arbitrary custom key-value settings.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for SettingsPayload {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
            connectivity: ConnectivityConfig::default(),
            personalization: PersonalizationConfig::default(),
            system: SystemConfig::default(),
            locks: SettingLockRegistry::default(),
            custom: HashMap::new(),
        }
    }
}

impl SettingsPayload {
    /// Creates a new default settings payload with schema version 1.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a setting key path is locked (e.g. "connectivity.wifi.enabled").
    ///
    /// Checks both the top-level lock registry and the specific typed field lock.
    pub fn is_key_locked(&self, key: &str) -> bool {
        if self.locks.is_locked(key) {
            return true;
        }
        self.field_lock(key).map(|l| l.is_locked()).unwrap_or(false)
    }

    /// Checks if a setting key path is specifically locked by Nix management.
    pub fn is_nix_locked(&self, key: &str) -> bool {
        if self.locks.is_nix_locked(key) {
            return true;
        }
        self.field_lock(key).map(|l| l.is_nix_locked()).unwrap_or(false)
    }

    /// Retrieves lock metadata for a setting key path, if locked.
    pub fn get_lock(&self, key: &str) -> Option<SettingLock> {
        if let Some(l) = self.locks.get(key) {
            if l.is_locked() {
                return Some(l.clone());
            }
        }
        self.field_lock(key).cloned()
    }

    /// Locks a key path with explicit lock metadata, synchronizing both
    /// the lock registry and the typed field (if recognized).
    pub fn lock_key(&mut self, key: impl Into<String>, lock: SettingLock) {
        let key_str = key.into();
        if let Some(field) = self.field_lock_mut(&key_str) {
            *field = lock.clone();
        }
        self.locks.lock(key_str, lock);
    }

    /// Convenience method to lock a setting as Nix-managed.
    pub fn lock_nix(&mut self, key: impl Into<String>, message: impl Into<String>) {
        self.lock_key(key, SettingLock::nix_locked(message));
    }

    /// Unlocks a setting key path.
    pub fn unlock_key(&mut self, key: &str) {
        if let Some(field) = self.field_lock_mut(key) {
            field.unlock();
        }
        self.locks.unlock(key);
    }

    // Single source of truth for dot-path → lock field. One table drives field_lock, field_lock_mut and all_locked_keys.
    const LOCK_FIELDS: &'static str = "connectivity.wifi.enabled\0connectivity.wifi.ask_to_join\0connectivity.wifi.limit_tracking\0connectivity.bluetooth.enabled\0connectivity.bluetooth.discoverable\0personalization.appearance.mode\0personalization.appearance.accent_color\0personalization.appearance.wallpaper\0personalization.appearance.wallpaper_idx\0personalization.appearance.scrollbar_pref\0personalization.appearance.icon_size_pref\0personalization.appearance.gaps_in\0personalization.appearance.gaps_out\0personalization.appearance.border_size\0personalization.display.brightness\0personalization.display.auto_brightness\0personalization.display.true_tone\0personalization.display.night_shift\0personalization.display.night_shift_mode\0personalization.display.color_temp\0personalization.display.resolution_choice\0personalization.sound.volume\0personalization.sound.muted\0personalization.sound.feedback_on_change\0personalization.focus.mode\0personalization.focus.work_schedule\0personalization.focus.sleep_schedule\0personalization.focus.share_across_devices\0personalization.notifications.enabled\0personalization.notifications.style\0personalization.notifications.silence_during_sleep\0personalization.notifications.messages\0personalization.notifications.calendar\0personalization.notifications.mail\0personalization.notifications.photos\0personalization.notifications.weather\0system.general.time_24h\0system.general.auto_updates\0system.date_time.automatic_timezone\0system.date_time.timezone\0system.date_time.ntp_sync\0system.date_time.time_format_24h\0system.storage.empty_trash_auto\0system.storage.save_to_cloud\0system.battery.mode\0system.battery.optimized_charging\0system.accessibility.text_size\0system.accessibility.reduce_motion\0system.accessibility.increase_contrast\0system.accessibility.reduce_transparency\0system.accessibility.screen_reader\0system.privacy.camera_enabled\0system.privacy.microphone_enabled\0system.privacy.location_enabled\0system.privacy.app_sandboxing\0system.language.primary_locale\0system.language.keyboard_layout\0system.language.spell_check\0system.language.autocorrect\0";

    /// Returns a list of all setting keys that are currently locked.
    pub fn all_locked_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.locks.iter().filter(|(_, l)| l.is_locked()).map(|(k, _)| k.clone()).collect();
        for key in Self::LOCK_FIELDS.split('\0').filter(|s| !s.is_empty()) {
            if !keys.iter().any(|k| k == key) && self.is_key_locked(key) {
                keys.push(key.to_string());
            }
        }
        keys
    }

    fn field_lock(&self, key: &str) -> Option<&SettingLock> {
        match key {
            "connectivity.wifi.enabled" => Some(&self.connectivity.wifi.enabled.lock),
            "connectivity.wifi.ask_to_join" => Some(&self.connectivity.wifi.ask_to_join.lock),
            "connectivity.wifi.limit_tracking" => Some(&self.connectivity.wifi.limit_tracking.lock),
            "connectivity.bluetooth.enabled" => Some(&self.connectivity.bluetooth.enabled.lock),
            "connectivity.bluetooth.discoverable" => Some(&self.connectivity.bluetooth.discoverable.lock),
            "personalization.appearance.mode" => Some(&self.personalization.appearance.mode.lock),
            "personalization.appearance.accent_color" => Some(&self.personalization.appearance.accent_color.lock),
            "personalization.appearance.wallpaper" => Some(&self.personalization.appearance.wallpaper.lock),
            "personalization.appearance.wallpaper_idx" => Some(&self.personalization.appearance.wallpaper_idx.lock),
            "personalization.appearance.scrollbar_pref" => Some(&self.personalization.appearance.scrollbar_pref.lock),
            "personalization.appearance.icon_size_pref" => Some(&self.personalization.appearance.icon_size_pref.lock),
            "personalization.appearance.gaps_in" => Some(&self.personalization.appearance.gaps_in.lock),
            "personalization.appearance.gaps_out" => Some(&self.personalization.appearance.gaps_out.lock),
            "personalization.appearance.border_size" => Some(&self.personalization.appearance.border_size.lock),
            "personalization.display.brightness" => Some(&self.personalization.display.brightness.lock),
            "personalization.display.auto_brightness" => Some(&self.personalization.display.auto_brightness.lock),
            "personalization.display.true_tone" => Some(&self.personalization.display.true_tone.lock),
            "personalization.display.night_shift" => Some(&self.personalization.display.night_shift.lock),
            "personalization.display.night_shift_mode" => Some(&self.personalization.display.night_shift_mode.lock),
            "personalization.display.color_temp" => Some(&self.personalization.display.color_temp.lock),
            "personalization.display.resolution_choice" => Some(&self.personalization.display.resolution_choice.lock),
            "personalization.sound.volume" => Some(&self.personalization.sound.volume.lock),
            "personalization.sound.muted" => Some(&self.personalization.sound.muted.lock),
            "personalization.sound.feedback_on_change" => Some(&self.personalization.sound.feedback_on_change.lock),
            "personalization.focus.mode" => Some(&self.personalization.focus.mode.lock),
            "personalization.focus.work_schedule" => Some(&self.personalization.focus.work_schedule.lock),
            "personalization.focus.sleep_schedule" => Some(&self.personalization.focus.sleep_schedule.lock),
            "personalization.focus.share_across_devices" => Some(&self.personalization.focus.share_across_devices.lock),
            "personalization.notifications.enabled" => Some(&self.personalization.notifications.enabled.lock),
            "personalization.notifications.style" => Some(&self.personalization.notifications.style.lock),
            "personalization.notifications.silence_during_sleep" => Some(&self.personalization.notifications.silence_during_sleep.lock),
            "personalization.notifications.messages" => Some(&self.personalization.notifications.messages.lock),
            "personalization.notifications.calendar" => Some(&self.personalization.notifications.calendar.lock),
            "personalization.notifications.mail" => Some(&self.personalization.notifications.mail.lock),
            "personalization.notifications.photos" => Some(&self.personalization.notifications.photos.lock),
            "personalization.notifications.weather" => Some(&self.personalization.notifications.weather.lock),
            "system.general.time_24h" => Some(&self.system.general.time_24h.lock),
            "system.general.auto_updates" => Some(&self.system.general.auto_updates.lock),
            "system.date_time.automatic_timezone" => Some(&self.system.date_time.automatic_timezone.lock),
            "system.date_time.timezone" => Some(&self.system.date_time.timezone.lock),
            "system.date_time.ntp_sync" => Some(&self.system.date_time.ntp_sync.lock),
            "system.date_time.time_format_24h" => Some(&self.system.date_time.time_format_24h.lock),
            "system.storage.empty_trash_auto" => Some(&self.system.storage.empty_trash_auto.lock),
            "system.storage.save_to_cloud" => Some(&self.system.storage.save_to_cloud.lock),
            "system.battery.mode" => Some(&self.system.battery.mode.lock),
            "system.battery.optimized_charging" => Some(&self.system.battery.optimized_charging.lock),
            "system.accessibility.text_size" => Some(&self.system.accessibility.text_size.lock),
            "system.accessibility.reduce_motion" => Some(&self.system.accessibility.reduce_motion.lock),
            "system.accessibility.increase_contrast" => Some(&self.system.accessibility.increase_contrast.lock),
            "system.accessibility.reduce_transparency" => Some(&self.system.accessibility.reduce_transparency.lock),
            "system.accessibility.screen_reader" => Some(&self.system.accessibility.screen_reader.lock),
            "system.privacy.camera_enabled" => Some(&self.system.privacy.camera_enabled.lock),
            "system.privacy.microphone_enabled" => Some(&self.system.privacy.microphone_enabled.lock),
            "system.privacy.location_enabled" => Some(&self.system.privacy.location_enabled.lock),
            "system.privacy.app_sandboxing" => Some(&self.system.privacy.app_sandboxing.lock),
            "system.language.primary_locale" => Some(&self.system.language.primary_locale.lock),
            "system.language.keyboard_layout" => Some(&self.system.language.keyboard_layout.lock),
            "system.language.spell_check" => Some(&self.system.language.spell_check.lock),
            "system.language.autocorrect" => Some(&self.system.language.autocorrect.lock),
            _ => None,
        }
    }
    fn field_lock_mut(&mut self, key: &str) -> Option<&mut SettingLock> {
        match key {
            "connectivity.wifi.enabled" => Some(&mut self.connectivity.wifi.enabled.lock),
            "connectivity.wifi.ask_to_join" => Some(&mut self.connectivity.wifi.ask_to_join.lock),
            "connectivity.wifi.limit_tracking" => Some(&mut self.connectivity.wifi.limit_tracking.lock),
            "connectivity.bluetooth.enabled" => Some(&mut self.connectivity.bluetooth.enabled.lock),
            "connectivity.bluetooth.discoverable" => Some(&mut self.connectivity.bluetooth.discoverable.lock),
            "personalization.appearance.mode" => Some(&mut self.personalization.appearance.mode.lock),
            "personalization.appearance.accent_color" => Some(&mut self.personalization.appearance.accent_color.lock),
            "personalization.appearance.wallpaper" => Some(&mut self.personalization.appearance.wallpaper.lock),
            "personalization.appearance.wallpaper_idx" => Some(&mut self.personalization.appearance.wallpaper_idx.lock),
            "personalization.appearance.scrollbar_pref" => Some(&mut self.personalization.appearance.scrollbar_pref.lock),
            "personalization.appearance.icon_size_pref" => Some(&mut self.personalization.appearance.icon_size_pref.lock),
            "personalization.appearance.gaps_in" => Some(&mut self.personalization.appearance.gaps_in.lock),
            "personalization.appearance.gaps_out" => Some(&mut self.personalization.appearance.gaps_out.lock),
            "personalization.appearance.border_size" => Some(&mut self.personalization.appearance.border_size.lock),
            "personalization.display.brightness" => Some(&mut self.personalization.display.brightness.lock),
            "personalization.display.auto_brightness" => Some(&mut self.personalization.display.auto_brightness.lock),
            "personalization.display.true_tone" => Some(&mut self.personalization.display.true_tone.lock),
            "personalization.display.night_shift" => Some(&mut self.personalization.display.night_shift.lock),
            "personalization.display.night_shift_mode" => Some(&mut self.personalization.display.night_shift_mode.lock),
            "personalization.display.color_temp" => Some(&mut self.personalization.display.color_temp.lock),
            "personalization.display.resolution_choice" => Some(&mut self.personalization.display.resolution_choice.lock),
            "personalization.sound.volume" => Some(&mut self.personalization.sound.volume.lock),
            "personalization.sound.muted" => Some(&mut self.personalization.sound.muted.lock),
            "personalization.sound.feedback_on_change" => Some(&mut self.personalization.sound.feedback_on_change.lock),
            "personalization.focus.mode" => Some(&mut self.personalization.focus.mode.lock),
            "personalization.focus.work_schedule" => Some(&mut self.personalization.focus.work_schedule.lock),
            "personalization.focus.sleep_schedule" => Some(&mut self.personalization.focus.sleep_schedule.lock),
            "personalization.focus.share_across_devices" => Some(&mut self.personalization.focus.share_across_devices.lock),
            "personalization.notifications.enabled" => Some(&mut self.personalization.notifications.enabled.lock),
            "personalization.notifications.style" => Some(&mut self.personalization.notifications.style.lock),
            "personalization.notifications.silence_during_sleep" => Some(&mut self.personalization.notifications.silence_during_sleep.lock),
            "personalization.notifications.messages" => Some(&mut self.personalization.notifications.messages.lock),
            "personalization.notifications.calendar" => Some(&mut self.personalization.notifications.calendar.lock),
            "personalization.notifications.mail" => Some(&mut self.personalization.notifications.mail.lock),
            "personalization.notifications.photos" => Some(&mut self.personalization.notifications.photos.lock),
            "personalization.notifications.weather" => Some(&mut self.personalization.notifications.weather.lock),
            "system.general.time_24h" => Some(&mut self.system.general.time_24h.lock),
            "system.general.auto_updates" => Some(&mut self.system.general.auto_updates.lock),
            "system.date_time.automatic_timezone" => Some(&mut self.system.date_time.automatic_timezone.lock),
            "system.date_time.timezone" => Some(&mut self.system.date_time.timezone.lock),
            "system.date_time.ntp_sync" => Some(&mut self.system.date_time.ntp_sync.lock),
            "system.date_time.time_format_24h" => Some(&mut self.system.date_time.time_format_24h.lock),
            "system.storage.empty_trash_auto" => Some(&mut self.system.storage.empty_trash_auto.lock),
            "system.storage.save_to_cloud" => Some(&mut self.system.storage.save_to_cloud.lock),
            "system.battery.mode" => Some(&mut self.system.battery.mode.lock),
            "system.battery.optimized_charging" => Some(&mut self.system.battery.optimized_charging.lock),
            "system.accessibility.text_size" => Some(&mut self.system.accessibility.text_size.lock),
            "system.accessibility.reduce_motion" => Some(&mut self.system.accessibility.reduce_motion.lock),
            "system.accessibility.increase_contrast" => Some(&mut self.system.accessibility.increase_contrast.lock),
            "system.accessibility.reduce_transparency" => Some(&mut self.system.accessibility.reduce_transparency.lock),
            "system.accessibility.screen_reader" => Some(&mut self.system.accessibility.screen_reader.lock),
            "system.privacy.camera_enabled" => Some(&mut self.system.privacy.camera_enabled.lock),
            "system.privacy.microphone_enabled" => Some(&mut self.system.privacy.microphone_enabled.lock),
            "system.privacy.location_enabled" => Some(&mut self.system.privacy.location_enabled.lock),
            "system.privacy.app_sandboxing" => Some(&mut self.system.privacy.app_sandboxing.lock),
            "system.language.primary_locale" => Some(&mut self.system.language.primary_locale.lock),
            "system.language.keyboard_layout" => Some(&mut self.system.language.keyboard_layout.lock),
            "system.language.spell_check" => Some(&mut self.system.language.spell_check.lock),
            "system.language.autocorrect" => Some(&mut self.system.language.autocorrect.lock),
            _ => None,
        }
    }
}

