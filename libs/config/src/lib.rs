use {
    anyhow::{anyhow, Context, Result},
    serde::{de::DeserializeOwned, Serialize},
    std::{
        fs::{self, OpenOptions},
        io::Write,
        path::{Path, PathBuf},
    },
};

pub mod ty;
pub use ty::*;

/// Resolves the Finick configuration directory root.
///
/// Priority:
/// 1. `FINICK_CONFIG_DIR` environment variable.
/// 2. `$HOME/.config/.finick`.
/// 3. `$XDG_CONFIG_HOME/finick`.
/// 4. `/tmp/.finick` fallback if home cannot be resolved.
pub fn finick_root() -> PathBuf {
    let path = if let Ok(custom) = std::env::var("FINICK_CONFIG_DIR") {
        PathBuf::from(custom)
    } else if let Ok(home) = std::env::var("HOME") {
        Path::new(&home).join(".config").join(".finick")
    } else if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        Path::new(&xdg).join("finick")
    } else {
        PathBuf::from("/tmp/.finick")
    };

    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }

    path
}

/// Resolves the file path for the Settings-centric configuration payload.
///
/// Resolution order:
/// 1. `FINICK_SETTINGS_PATH` or `FINICK_SETTINGS_FILE` environment override.
/// 2. Declarative NixOS module state file (`/etc/finick-settings.json`) if it exists.
/// 3. Canonical user config file (`~/.config/.finick/settings.json`).
/// 4. Legacy path (`~/.config/.finick/Settings`) if it exists.
pub fn settings_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("FINICK_SETTINGS_PATH").or_else(|_| std::env::var("FINICK_SETTINGS_FILE")) {
        return PathBuf::from(override_path);
    }

    let nix_state = Path::new("/etc/finick-settings.json");
    if nix_state.exists() {
        return nix_state.to_path_buf();
    }

    let standard = finick_root().join("settings.json");
    if standard.exists() {
        return standard;
    }

    let legacy = finick_root().join(App::Settings.to_string());
    if legacy.exists() {
        return legacy;
    }

    standard
}

/// Resolves the path to the configuration file for a given App identity.
pub fn config_path_for(app: &App) -> PathBuf {
    match app {
        App::Settings | App::SettingsDaemon => settings_path(),
        _ => {
            let legacy = finick_root().join(app.to_string());
            if legacy.exists() && !legacy.is_dir() {
                legacy
            } else {
                finick_root().join(app.config_file_name())
            }
        }
    }
}

/// Writes JSON data atomically to the specified path via a temporary file and rename.
fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory {:?}", parent))?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
    {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .with_context(|| format!("Failed to open temp config file {:?}", tmp_path))?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, value)
            .with_context(|| format!("Failed to serialize config to {:?}", tmp_path))?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }

    if let Err(_rename_err) = fs::rename(&tmp_path, path) {
        // Fallback for environments where atomic rename may fail across mount boundaries
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to open destination config file {:?}", path))?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, value)?;
        writer.flush()?;
        let _ = fs::remove_file(&tmp_path);
    }

    Ok(())
}

/// Reads a configuration payload for the specified application identity.
///
/// If the configuration file does not exist, writes a default payload atomically and returns it.
pub fn get_config<T: DeserializeOwned + Serialize + Default>(app: App) -> Result<T> {
    let path = config_path_for(&app);
    if !path.exists() {
        let default_val = T::default();
        atomic_write_json(&path, &default_val)?;
        Ok(default_val)
    } else {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file {:?}", path))?;
        serde_json::from_str(&content).map_err(|e| anyhow!(e))
    }
}

/// Writes a configuration payload for the specified application identity atomically.
pub fn write_config<T: Serialize>(app: App, value: T) -> Result<()> {
    let path = config_path_for(&app);
    atomic_write_json(&path, &value)
}

/// Loads the Settings configuration payload.
///
/// If the settings file does not yet exist, creates it with default settings, persists it atomically,
/// and returns the default payload.
pub fn load_settings() -> Result<SettingsPayload> {
    let path = settings_path();
    if !path.exists() {
        let default_settings = SettingsPayload::default();
        save_settings(&default_settings)?;
        Ok(default_settings)
    } else {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read settings from {:?}", path))?;
        let settings = serde_json::from_str::<SettingsPayload>(&content)
            .with_context(|| format!("Failed to parse settings JSON from {:?}", path))?;
        Ok(settings)
    }
}

/// Saves the Settings configuration payload atomically to the resolved settings path.
pub fn save_settings(settings: &SettingsPayload) -> Result<()> {
    let path = settings_path();
    atomic_write_json(&path, settings)
}

/// Loads the current Settings payload, executes the modifying closure `f`,
/// and saves the mutated settings atomically.
pub fn update_settings<F>(f: F) -> Result<SettingsPayload>
where
    F: FnOnce(&mut SettingsPayload) -> Result<()>,
{
    let mut settings = load_settings()?;
    f(&mut settings)?;
    save_settings(&settings)?;
    Ok(settings)
}

/// Checks whether a given setting key path is currently locked.
pub fn is_setting_locked(key_path: &str) -> Result<bool> {
    let settings = load_settings()?;
    Ok(settings.is_key_locked(key_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_app_variants_and_file_names() {
        assert_eq!(App::Settings.as_str(), "Settings");
        assert_eq!(App::SettingsDaemon.as_str(), "SettingsDaemon");
        assert_eq!(App::TopBar.as_str(), "TopBar");
        assert_eq!(App::IndexService.as_str(), "IndexService");
        assert_eq!(App::Scan.as_str(), "Scan");
        assert_eq!(App::Files.as_str(), "Files");

        assert_eq!(App::Settings.config_file_name(), "Settings.json");
        let path: PathBuf = App::Settings.into();
        assert_eq!(path, PathBuf::from("Settings"));
    }

    #[test]
    fn test_setting_wrapper_unlocked() {
        let mut setting = Setting::new(42);
        assert!(!setting.is_locked());
        assert!(setting.is_unlocked());
        assert_eq!(*setting, 42);
        assert_eq!(setting.get(), &42);

        // Can mutate through get_mut when unlocked
        if let Some(val) = setting.get_mut() {
            *val = 100;
        }
        assert_eq!(*setting, 100);

        // Can mutate through set()
        assert!(setting.set(200).is_ok());
        assert_eq!(*setting, 200);

        // Unlocked serializes as plain value
        let json = serde_json::to_string(&setting).unwrap();
        assert_eq!(json, "200");

        // Deserializes cleanly from plain value
        let deserialized: Setting<i32> = serde_json::from_str("300").unwrap();
        assert_eq!(*deserialized, 300);
        assert!(deserialized.is_unlocked());
    }

    #[test]
    fn test_setting_wrapper_locked() {
        let mut setting = Setting::new_nix_locked(true, "Configured in configuration.nix");
        assert!(setting.is_locked());
        assert!(!setting.is_unlocked());
        assert!(setting.is_nix_locked());
        assert_eq!(*setting, true);
        assert_eq!(setting.lock_message(), Some("Configured in configuration.nix"));

        // get_mut returns None when locked
        assert!(setting.get_mut().is_none());

        // set returns SettingLockedError
        let err = setting.set(false).unwrap_err();
        assert!(err.to_string().contains("Nix"));

        // Value remains unchanged
        assert_eq!(*setting, true);

        // Serializes with lock metadata
        let json = serde_json::to_string(&setting).unwrap();
        assert!(json.contains("\"value\":true"));
        assert!(json.contains("\"locked\":true"));
        assert!(json.contains("configuration.nix"));

        // Roundtrip deserialization preserves lock state
        let deserialized: Setting<bool> = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_locked());
        assert_eq!(*deserialized, true);
        assert!(deserialized.is_nix_locked());

        // Unlocking allows mutations again
        setting.unlock();
        assert!(setting.is_unlocked());
        assert!(setting.set(false).is_ok());
        assert_eq!(*setting, false);
    }

    #[test]
    fn test_setting_lock_registry() {
        let mut registry = SettingLockRegistry::new();
        assert!(registry.is_empty());

        registry.lock_nix("connectivity.wifi.enabled", "Disabled by Nix flake");
        assert_eq!(registry.len(), 1);
        assert!(registry.is_locked("connectivity.wifi.enabled"));
        assert!(registry.is_nix_locked("connectivity.wifi.enabled"));
        assert!(!registry.is_locked("personalization.sound.volume"));

        assert!(registry.unlock("connectivity.wifi.enabled"));
        assert!(!registry.is_locked("connectivity.wifi.enabled"));
    }

    #[test]
    fn test_settings_payload_defaults_and_locking() {
        let mut payload = SettingsPayload::new();
        assert_eq!(payload.schema_version, 1);
        assert_eq!(*payload.personalization.appearance.mode, ThemeMode::Auto);
        assert_eq!(*payload.connectivity.wifi.enabled, true);
        assert_eq!(*payload.personalization.sound.volume, 65);

        // Lock a setting via path
        payload.lock_nix("connectivity.wifi.enabled", "NixOS network configuration");
        assert!(payload.is_key_locked("connectivity.wifi.enabled"));
        assert!(payload.is_nix_locked("connectivity.wifi.enabled"));
        assert!(payload.connectivity.wifi.enabled.is_locked());

        let lock = payload.get_lock("connectivity.wifi.enabled").unwrap();
        assert!(lock.is_locked());
        assert_eq!(lock.message.as_deref(), Some("NixOS network configuration"));

        // Serialization roundtrip
        let json = serde_json::to_string_pretty(&payload).unwrap();
        let roundtrip: SettingsPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.schema_version, 1);
        assert!(roundtrip.is_key_locked("connectivity.wifi.enabled"));

        // Unlock key
        payload.unlock_key("connectivity.wifi.enabled");
        assert!(!payload.is_key_locked("connectivity.wifi.enabled"));
        assert!(payload.connectivity.wifi.enabled.is_unlocked());
    }

    #[test]
    fn test_atomic_persistence() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("finick_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let test_settings_file = temp_dir.join("test_settings.json");
        std::env::set_var("FINICK_SETTINGS_PATH", &test_settings_file);

        // Test loading default settings when file doesn't exist
        let settings = load_settings().expect("Failed to load settings");
        assert_eq!(settings.schema_version, 1);
        assert!(test_settings_file.exists());

        // Test update_settings
        let updated = update_settings(|s| {
            s.personalization.appearance.mode = Setting::new(ThemeMode::Dark);
            s.lock_nix("personalization.appearance.mode", "Enforced dark mode");
            Ok(())
        })
        .expect("Failed to update settings");

        assert_eq!(*updated.personalization.appearance.mode, ThemeMode::Dark);
        assert!(updated.is_key_locked("personalization.appearance.mode"));

        // Reload from disk to verify atomic persistence
        let reloaded = load_settings().expect("Failed to reload settings");
        assert_eq!(*reloaded.personalization.appearance.mode, ThemeMode::Dark);
        assert!(reloaded.is_key_locked("personalization.appearance.mode"));

        // Clean up
        std::env::remove_var("FINICK_SETTINGS_PATH");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
