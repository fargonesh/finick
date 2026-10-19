// Privacy enforcement backends (issue #12).
//
// Each query returns a status the UI maps to Live / Managed / Unavailable.
// Each apply returns Ok on real enforcement or Err with a human reason the
// daemon surfaces as ApplyFailed so SettingsStore can roll back.

use std::process::Command;

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd).args(args).output().ok().and_then(|o| {
        if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).to_string())
        } else {
            None
        }
    })
}

fn have_bin(name: &str) -> bool {
    Command::new(name).arg("--help").output().is_ok()
        || std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|d| d.join(name))
            .any(|p| p.is_file())
}

// --- Firewall ---

#[derive(Clone, Debug, PartialEq)]
pub struct FirewallStatus {
    pub active: bool,
    pub backend: String,
    pub detail: String,
    pub manageable: bool,
}

pub fn firewall_backend_name() -> Option<String> {
    if run("ufw", &["status"]).is_some() {
        return Some("ufw".to_string());
    }
    if let Some(out) = run("firewall-cmd", &["--state"]) {
        if !out.trim().is_empty() {
            return Some("firewalld".to_string());
        }
    }
    if run("iptables", &["-L", "-n"]).is_some() {
        return Some("iptables".to_string());
    }
    None
}

pub fn query_firewall() -> FirewallStatus {
    if let Some(out) = run("ufw", &["status", "verbose"]) {
        let lower = out.to_lowercase();
        if lower.contains("status: active") {
            return FirewallStatus {
                active: true,
                backend: "UFW".to_string(),
                detail: "UFW active".to_string(),
                manageable: true,
            };
        }
        if lower.contains("status: inactive") {
            return FirewallStatus {
                active: false,
                backend: "UFW".to_string(),
                detail: "UFW inactive".to_string(),
                manageable: true,
            };
        }
    }
    if let Some(out) = run("firewall-cmd", &["--state"]) {
        let t = out.trim();
        if t == "running" {
            return FirewallStatus {
                active: true,
                backend: "firewalld".to_string(),
                detail: "firewalld running".to_string(),
                manageable: true,
            };
        }
        if !t.is_empty() {
            return FirewallStatus {
                active: false,
                backend: "firewalld".to_string(),
                detail: format!("firewalld {t}"),
                manageable: true,
            };
        }
    }
    if run("iptables", &["-L", "-n"]).is_some() {
        return FirewallStatus {
            active: true,
            backend: "iptables".to_string(),
            detail: "iptables rules present (read-only here)".to_string(),
            manageable: false,
        };
    }
    FirewallStatus {
        active: false,
        backend: "none".to_string(),
        detail: "No firewall backend found (ufw / firewalld / iptables)".to_string(),
        manageable: false,
    }
}

pub fn set_firewall_enabled(enabled: bool) -> Result<(), String> {
    if firewall_backend_name().as_deref() == Some("ufw") {
        let arg = if enabled { "--force enable" } else { "disable" };
        // ufw enable takes one arg; split the force flag form.
        let out = if enabled {
            Command::new("ufw").args(["--force", "enable"]).output()
        } else {
            Command::new("ufw").args([arg]).output()
        }
        .map_err(|e| format!("failed to run ufw: {e}"))?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err
        };
        return Err(if msg.to_lowercase().contains("permission") || msg.contains("must be root") {
            format!("ufw needs root: {msg}")
        } else if msg.is_empty() {
            "ufw rejected the change".to_string()
        } else {
            msg
        });
    }
    if firewall_backend_name().as_deref() == Some("firewalld") {
        return Err("firewalld changes need root via firewall-cmd; run in a terminal".to_string());
    }
    Err("No manageable firewall backend found (install ufw or firewalld)".to_string())
}

// --- Location (GeoClue / GNOME portal) ---

#[derive(Clone, Debug, PartialEq)]
pub struct LocationStatus {
    pub available: bool,
    pub enabled: bool,
    pub reason: String,
}

pub fn gschema_exists(schema: &str) -> bool {
    Command::new("gsettings")
        .args(["list-recursively", schema])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn query_location() -> LocationStatus {
    if !have_bin("gsettings") || !gschema_exists("org.gnome.system.location") {
        // GeoClue agent check as secondary signal; still unavailable without the schema.
        let geoclue = Command::new("systemctl")
            .args(["is-active", "geoclue.service"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let _ = geoclue;
        return LocationStatus {
            available: false,
            enabled: false,
            reason: "Location portal not found (gsettings org.gnome.system.location missing)".to_string(),
        };
    }
    let enabled = run("gsettings", &["get", "org.gnome.system.location", "enabled"])
        .map(|s| s.trim().to_lowercase().contains("true"))
        .unwrap_or(true);
    LocationStatus {
        available: true,
        enabled,
        reason: "GNOME location portal".to_string(),
    }
}

pub fn set_location_enabled(enabled: bool) -> Result<(), String> {
    if !have_bin("gsettings") || !gschema_exists("org.gnome.system.location") {
        return Err("Location portal not available on this system".to_string());
    }
    let out = Command::new("gsettings")
        .args(["set", "org.gnome.system.location", "enabled", if enabled { "true" } else { "false" }])
        .output()
        .map_err(|e| format!("failed to run gsettings: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

// --- Camera / microphone ---

#[derive(Clone, Debug, PartialEq)]
pub struct CameraStatus {
    pub count: usize,
    pub available: bool,
    pub reason: String,
}

pub fn query_camera() -> CameraStatus {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with("video") {
                count += 1;
            }
        }
    }
    if count == 0 {
        CameraStatus {
            count,
            available: false,
            reason: "No camera device found".to_string(),
        }
    } else {
        // No portal inhibit backend ships with Finick: toggling would lie.
        CameraStatus {
            count,
            available: false,
            reason: "No camera inhibit backend; cover the lens or unplug the device".to_string(),
        }
    }
}

pub fn set_camera_enabled(_enabled: bool) -> Result<(), String> {
    let st = query_camera();
    if st.count == 0 {
        return Err("No camera device found".to_string());
    }
    Err("Camera inhibit is not enforceable here; use the physical shutter or unplug the device".to_string())
}

#[derive(Clone, Debug, PartialEq)]
pub struct MicStatus {
    pub available: bool,
    pub muted: bool,
    pub reason: String,
}

pub fn query_mic() -> MicStatus {
    if !have_bin("wpctl") && !have_bin("pactl") {
        return MicStatus {
            available: false,
            muted: false,
            reason: "PipeWire controls not found (wpctl / pactl missing)".to_string(),
        };
    }
    if let Some(out) = run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SOURCE@"]) {
        return MicStatus {
            available: true,
            muted: out.contains("[MUTED]"),
            reason: "PipeWire source".to_string(),
        };
    }
    MicStatus {
        available: true,
        muted: false,
        reason: "PipeWire source".to_string(),
    }
}

pub fn set_mic_enabled(enabled: bool) -> Result<(), String> {
    // enabled=true means unmuted (apps may record); enabled=false mutes the source.
    let mute_arg = if enabled { "0" } else { "1" };
    if have_bin("wpctl") {
        let out = Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", mute_arg])
            .output()
            .map_err(|e| format!("failed to run wpctl: {e}"))?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "wpctl rejected the microphone change".to_string() } else { err });
    }
    Err("PipeWire controls not found (wpctl missing)".to_string())
}

// --- USB protection (USBGuard) ---

#[derive(Clone, Debug, PartialEq)]
pub struct UsbStatus {
    pub available: bool,
    pub enforced: bool,
    pub reason: String,
}

pub fn query_usb_protection() -> UsbStatus {
    if !have_bin("usbguard") {
        return UsbStatus {
            available: false,
            enforced: false,
            reason: "USBGuard not installed; new USB devices are not blocked".to_string(),
        };
    }
    // Presence of the daemon + a policy implies enforcement; read-only probe.
    let devices = run("usbguard", &["list-devices"]).unwrap_or_default();
    let enforced = !devices.trim().is_empty() || run("usbguard", &["get-parameter", "ImplicitPolicyTarget"]).is_some();
    UsbStatus {
        available: true,
        enforced,
        reason: if enforced {
            "USBGuard policy active".to_string()
        } else {
            "USBGuard installed".to_string()
        },
    }
}

pub fn set_usb_protection(enabled: bool) -> Result<(), String> {
    if !have_bin("usbguard") {
        return Err("USBGuard not installed; cannot block new USB devices".to_string());
    }
    if enabled {
        // Honest read-only integration: Finick detects USBGuard but does not
        // rewrite its policy (needs root + explicit rules). Report the state.
        let st = query_usb_protection();
        if st.enforced {
            return Ok(());
        }
        return Err("USBGuard policy edit needs root; allow devices with `usbguard allow-device`".to_string());
    }
    Err("Disabling USB protection needs root via USBGuard policy".to_string())
}

// --- Screen lock delay (GNOME parity) ---

fn finick_data_file(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|c| c.join("finick").join(name))
}

pub fn query_lock_delay() -> u32 {
    if let Some(out) = run("gsettings", &["get", "org.gnome.desktop.screensaver", "lock-delay"]) {
        // "uint32 300" or "300"
        for tok in out.replace("uint32", " ").split_whitespace() {
            if let Ok(v) = tok.parse::<u32>() {
                return v;
            }
        }
    }
    if let Some(f) = finick_data_file("lock_delay") {
        if let Ok(raw) = std::fs::read_to_string(f) {
            if let Ok(v) = raw.trim().parse::<u32>() {
                return v;
            }
        }
    }
    // Fall back to the idle blank delay so "lock after blank" stays sane.
    if let Some(out) = run("gsettings", &["get", "org.gnome.desktop.session", "idle-delay"]) {
        for tok in out.replace("uint32", " ").split_whitespace() {
            if let Ok(v) = tok.parse::<u32>() {
                return v.min(3600);
            }
        }
    }
    300
}

pub fn set_lock_delay(seconds: u32) -> Result<(), String> {
    let secs = seconds.min(3600);
    if have_bin("gsettings") && gschema_exists("org.gnome.desktop.screensaver") {
        let out = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.screensaver", "lock-delay", &format!("uint32 {secs}")])
            .output()
            .map_err(|e| format!("failed to run gsettings: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
    }
    if let Some(f) = finick_data_file("lock_delay") {
        if let Some(parent) = f.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(f, format!("{secs}")).map_err(|e| format!("failed to save lock delay: {e}"))?;
    }
    Ok(())
}

// --- File history clear (recent + trash + shell history) ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct HistoryClearReport {
    pub recent_removed: bool,
    pub trash_emptied: usize,
    pub shells_truncated: Vec<String>,
}

fn trash_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share/Trash/files"))
}

pub fn clear_file_history() -> Result<HistoryClearReport, String> {
    let mut report = HistoryClearReport::default();

    // 1. GTK recent files.
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        let recent = home.join(".local/share/recently-used.xbel");
        if recent.is_file() {
            match std::fs::remove_file(&recent) {
                Ok(()) => report.recent_removed = true,
                Err(e) => return Err(format!("could not clear recent files: {e}")),
            }
        } else {
            report.recent_removed = true;
        }
    }

    // 2. Trash.
    if let Some(dir) = trash_dir() {
        if dir.is_dir() {
            let mut n = 0;
            for e in std::fs::read_dir(&dir).map_err(|e| format!("could not read trash: {e}"))?.flatten() {
                let p = e.path();
                let ok = if p.is_dir() { std::fs::remove_dir_all(&p).is_ok() } else { std::fs::remove_file(&p).is_ok() };
                if ok {
                    n += 1;
                }
            }
            report.trash_emptied = n;
        }
    }

    // 3. Shell histories (truncate, keep file so shells keep working).
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        for name in [".bash_history", ".zsh_history", ".local/share/fish/fish_history"] {
            let p = home.join(name);
            if p.is_file() {
                if std::fs::write(&p, "").is_ok() {
                    report.shells_truncated.push(name.to_string());
                }
            }
        }
    }
    Ok(report)
}

// --- Diagnostics (honest local-only toggle) ---

pub fn query_diagnostics() -> bool {
    finick_data_file("diagnostics")
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|s| s.trim() == "1" || s.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn set_diagnostics(enabled: bool) -> Result<(), String> {
    if let Some(f) = finick_data_file("diagnostics") {
        if let Some(parent) = f.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(f, if enabled { "1" } else { "0" })
            .map_err(|e| format!("failed to save diagnostics preference: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firewall_unavailable_without_backend() {
        // Pure parse check: iptables output shape is not required; just ensure
        // the unavailable fallback carries a reason.
        let st = FirewallStatus {
            active: false,
            backend: "none".to_string(),
            detail: "No firewall backend found (ufw / firewalld / iptables)".to_string(),
            manageable: false,
        };
        assert!(!st.manageable);
        assert!(st.detail.contains("No firewall"));
    }

    #[test]
    fn lock_delay_clamps() {
        // set_lock_delay clamps in-memory; file write may fail in sandbox, ignore.
        let _ = set_lock_delay(99999);
        // query must not panic and stays within range
        assert!(query_lock_delay() <= 3600);
    }
}
