use {
    serde::{Deserialize, Serialize},
    std::{
        collections::{HashMap, HashSet},
        process::Command,
    },
};
pub mod clipboard;
pub use clipboard::*;
pub mod capture;
pub use capture::*;
pub mod privacy;
pub use privacy::*;
pub mod shortcuts;
pub use shortcuts::*;
pub mod hypr;
pub use hypr::{hypr_bind, hypr_bring_to_top, hypr_close_window, hypr_exec, hypr_focus_window, hypr_workspace, lua_escape};
pub mod desktop_apps;
pub use desktop_apps::*;
pub mod auth;
pub use auth::{
    PamReply, PamRequest, authenticate_user, current_username, pam_authenticate, pam_interactive, pam_service_available,
    verify_password,
};
pub mod lock;
pub use lock::{locker_binary, locker_locked, locker_lockfile, locker_process_alive, trigger_lock};
pub mod store;
pub mod store_flatpak;
pub mod store_hm;
pub mod store_meta;
pub mod store_nix;
pub mod store_opts;

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayInfo {
    pub name: String,
    pub description: String,
    pub resolution: String,
    pub refresh_rate: String,
    pub scale: String,
    pub transform: u8,
    pub x: i32,
    pub y: i32,
    pub available_modes: Vec<String>,
    pub is_focused: bool,
}

pub fn parse_display_dimensions(resolution: &str) -> (u32, u32) {
    let trimmed = resolution.trim();
    if trimmed.is_empty() {
        return (1920, 1080);
    }
    let mut parts = trimmed.split(|c| c == 'x' || c == 'X');
    let w = parts.next().and_then(|s| s.trim().parse::<u32>().ok()).filter(|&v| v > 0).unwrap_or(1920);
    let h = parts.next().and_then(|s| s.trim().split('@').next()?.parse::<u32>().ok()).filter(|&v| v > 0).unwrap_or(1080);
    (w, h)
}

pub fn parse_display_width(resolution: &str) -> u32 {
    let (w, _) = parse_display_dimensions(resolution);
    w
}

pub fn build_display_positions(ordered: &[(String, String)]) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(ordered.len());
    let mut x: u32 = 0;
    for (name, resolution) in ordered {
        out.push((name.clone(), format!("{x}x0")));
        x = x.saturating_add(parse_display_width(resolution));
    }
    out
}

pub fn build_monitor_position_args(ordered: &[(String, String)]) -> Vec<(String, String)> {
    build_display_positions(ordered)
}

pub fn build_positions_from_names(ordered_names: &[String], lookup: &HashMap<String, String>) -> Vec<(String, String)> {
    let ordered: Vec<(String, String)> =
        ordered_names.iter().map(|n| (n.clone(), lookup.get(n).cloned().unwrap_or_default())).collect();
    build_display_positions(&ordered)
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RawHyprMonitor {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub refresh_rate: Option<Box<serde_json::value::RawValue>>,
    #[serde(default)]
    pub scale: Option<Box<serde_json::value::RawValue>>,
    #[serde(default)]
    pub transform: Option<u8>,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default)]
    pub available_modes: Vec<String>,
    #[serde(default)]
    pub focused: bool,
}

pub fn parse_hyprctl_monitors(out: &str) -> Vec<DisplayInfo> {
    if let Ok(raw_list) = serde_json::from_str::<Vec<RawHyprMonitor>>(out) {
        return raw_list
            .into_iter()
            .map(|raw| {
                let resolution = match (raw.width, raw.height) {
                    (Some(w), Some(h)) if w > 0 && h > 0 => format!("{w}x{h}"),
                    _ => String::new(),
                };
                let refresh_rate = match raw.refresh_rate {
                    Some(r) => r.get().trim_matches('"').to_string(),
                    None => String::new(),
                };
                let scale = match raw.scale {
                    Some(r) => r.get().trim_matches('"').to_string(),
                    None => "1.00".to_string(),
                };
                DisplayInfo {
                    name: raw.name,
                    description: raw.description,
                    resolution,
                    refresh_rate,
                    scale,
                    transform: raw.transform.unwrap_or(0),
                    x: raw.x.unwrap_or(0),
                    y: raw.y.unwrap_or(0),
                    available_modes: raw.available_modes,
                    is_focused: raw.focused,
                }
            })
            .collect();
    }
    Vec::new()
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct InputDevice {
    pub name: String,
    pub layout_or_type: String,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InputDevices {
    pub mice: Vec<InputDevice>,
    pub keyboards: Vec<InputDevice>,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AudioInfo {
    pub volume: f64,
    pub is_muted: bool,
    pub default_sink_name: String,
    pub default_source_name: String,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn parse_wpctl_devices(output: &str) -> (Vec<AudioDevice>, Vec<AudioDevice>) {
    let mut sinks = Vec::new();
    let mut sources = Vec::new();
    let mut section = "";
    for line in output.lines() {
        let trimmed =
            line.trim_start_matches(|c: char| c.is_whitespace() || c == '│' || c == '├' || c == '└' || c == '─').trim_end();
        if trimmed.ends_with("Sinks:") {
            section = "sinks";
            continue;
        }
        if trimmed.ends_with("Sources:") {
            section = "sources";
            continue;
        }
        if trimmed.ends_with("Filters:") || trimmed.ends_with("Streams:") || trimmed.ends_with("Devices:") {
            section = "";
            continue;
        }
        if section.is_empty() {
            continue;
        }
        let body = trimmed.strip_prefix('*').map(str::trim).unwrap_or(trimmed);
        let is_default = body.len() != trimmed.len();
        let Some(dot) = body.find('.') else { continue };
        let (id, rest) = body.split_at(dot);
        let id = id.trim();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let name = rest[1..].split('[').next().unwrap_or_default().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let device = AudioDevice { id: id.to_string(), name, is_default };
        if section == "sinks" {
            sinks.push(device);
        } else {
            sources.push(device);
        }
    }
    (sinks, sources)
}
#[derive(Clone, Debug, PartialEq)]
pub struct PowerInfo {
    pub capacity: String,
    pub status: String,
    pub health_percent: Option<u8>,
    pub cycle_count: Option<u32>,
}

pub fn compute_battery_health(full: u64, design: u64) -> Option<u8> {
    if design == 0 || full == 0 {
        return None;
    }
    let pct = full * 100 / design;
    Some(pct.min(100) as u8)
}

fn read_sysfs_u64(dir: &std::path::Path, names: &[&str]) -> Option<u64> {
    names.iter().find_map(|n| std::fs::read_to_string(dir.join(n)).ok().and_then(|s| s.trim().parse::<u64>().ok()))
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralInfo {
    pub locale: String,
    pub timezone: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HostInfo {
    pub hostname: String,
    pub os_name: String,
    pub kernel: String,
    pub uptime: String,
    pub memory: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DiskInfo {
    pub filesystem: String,
    pub mount_point: String,
    pub total: String,
    pub used: String,
    pub available: String,
    pub use_percent: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StorageInfo {
    pub total: String,
    pub used: String,
    pub available: String,
    pub use_percent: f64,
    pub disks: Vec<DiskInfo>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct UserInfo {
    pub username: String,
    pub uid: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WifiNetworkDetail {
    pub ssid: String,
    pub signal: u8,
    pub security: String,
    pub is_connected: bool,
    pub is_saved: bool,
}

fn split_escaped_colon(s: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                cur.push(n);
            }
            continue;
        }
        if c == ':' {
            fields.push(std::mem::take(&mut cur));
            continue;
        }
        cur.push(c);
    }
    fields.push(cur);
    fields
}

pub fn parse_nmcli_wifi_line(line: &str) -> Option<(bool, String, u8, String)> {
    let fields = split_escaped_colon(line);
    if fields.len() < 4 {
        return None;
    }
    let active = fields[0].trim() == "yes";
    let ssid = fields[1].trim().to_string();
    if ssid.is_empty() || ssid == "--" {
        return None;
    }
    let signal = fields[2].trim().parse::<u8>().unwrap_or(0);
    let sec = fields[3].trim().to_string();
    Some((active, ssid, signal, if sec.is_empty() { "Open".to_string() } else { sec }))
}

pub fn parse_ip_route_get(output: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = output.split_whitespace().collect();
    let mut iface = None;
    let mut src = None;
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "dev" && i + 1 < parts.len() {
            iface = Some(parts[i + 1].to_string());
        }
        if parts[i] == "src" && i + 1 < parts.len() {
            src = Some(parts[i + 1].to_string());
        }
        i += 1;
    }
    match (iface, src) {
        (Some(iface), Some(src)) => Some((iface, src)),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WiredInfo {
    pub connected: bool,
    pub interface: String,
    pub ip_address: String,
    pub profile: String,
}

pub fn parse_nmcli_dev_line(line: &str) -> Option<(String, String, String)> {
    let fields = split_escaped_colon(line);
    if fields.len() < 3 {
        return None;
    }
    let device = fields[0].trim().to_string();
    if device.is_empty() {
        return None;
    }
    Some((device, fields[1].trim().to_string(), fields[2].trim().to_string()))
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CurrentWifiInfo {
    pub connected: bool,
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
    pub security: Option<String>,
    pub interface: Option<String>,
}

// --- Issue #6: Mouse & Touchpad ---
#[derive(Clone, Debug, PartialEq)]
pub struct PointerSettings {
    pub speed: f64,
    pub accel_profile: String,
    pub natural_scroll: bool,
    pub tap_to_click: bool,
    pub disable_while_typing: bool,
    pub left_handed: bool,
}

impl Default for PointerSettings {
    fn default() -> Self {
        Self {
            speed: 0.0,
            accel_profile: "adaptive".to_string(),
            natural_scroll: false,
            tap_to_click: true,
            disable_while_typing: true,
            left_handed: false,
        }
    }
}

fn finick_data_file(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|c| c.join("finick").join(name))
}

fn hyprctl_get_keyword_lines() -> String {
    Command::new("hyprctl").args(["getoption", "input:sensitivity", "-j"]).output().map(|o| {
        String::from_utf8_lossy(&o.stdout).to_string()
    }).unwrap_or_default()
}

pub fn load_pointer_settings() -> PointerSettings {
    let mut s = PointerSettings::default();
    if let Some(f) = finick_data_file("pointer.json") {
        if let Ok(raw) = std::fs::read_to_string(f) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(x) = v.get("speed").and_then(|x| x.as_f64()) { s.speed = x.clamp(-1.0, 1.0); }
                if let Some(p) = v.get("accel_profile").and_then(|x| x.as_str()) { s.accel_profile = p.to_string(); }
                if let Some(b) = v.get("natural_scroll").and_then(|x| x.as_bool()) { s.natural_scroll = b; }
                if let Some(b) = v.get("tap_to_click").and_then(|x| x.as_bool()) { s.tap_to_click = b; }
                if let Some(b) = v.get("disable_while_typing").and_then(|x| x.as_bool()) { s.disable_while_typing = b; }
                if let Some(b) = v.get("left_handed").and_then(|x| x.as_bool()) { s.left_handed = b; }
                return s;
            }
        }
    }
    let out = hyprctl_get_keyword_lines();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
        if let Some(x) = v.get("float").and_then(|x| x.as_f64()) { s.speed = x.clamp(-1.0, 1.0); }
    }
    s
}

fn persist_pointer_settings(s: &PointerSettings) {
    if let Some(f) = finick_data_file("pointer.json") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        let v = serde_json::json!({
            "speed": s.speed, "accel_profile": s.accel_profile,
            "natural_scroll": s.natural_scroll, "tap_to_click": s.tap_to_click,
            "disable_while_typing": s.disable_while_typing, "left_handed": s.left_handed,
        });
        let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default());
    }
}

fn hypr_input_keyword(key: &str, val: &str) -> Result<(), String> {
    // Lua-first (Hyprland >=0.55 rejects `keyword` on Lua configs), legacy fallback.
    let lua_val = if val == "true" || val == "false" {
        val.to_string()
    } else if val.parse::<f64>().is_ok() {
        val.to_string()
    } else {
        format!("\"{}\"", val.replace('\\', "\\\\").replace('"', "\\\""))
    };
    let lua = format!("hl.config({{ input = {{ {key} = {lua_val} }} }})");
    HyprlandBackend::hyprctl_set_config(&lua, &format!("input:{key}"), val)
}

pub fn apply_pointer_speed(speed: f64) -> Result<(), String> {
    let clamped = speed.clamp(-1.0, 1.0);
    hypr_input_keyword("sensitivity", &format!("{clamped:.2}"))?;
    let mut s = load_pointer_settings();
    s.speed = clamped;
    persist_pointer_settings(&s);
    Ok(())
}

pub fn apply_pointer_accel(profile: &str) -> Result<(), String> {
    let v = match profile {
        "flat" => "flat",
        _ => "adaptive",
    };
    hypr_input_keyword("accel_profile", v)?;
    let mut s = load_pointer_settings();
    s.accel_profile = v.to_string();
    persist_pointer_settings(&s);
    Ok(())
}

pub fn apply_input_bool(key: &str, enabled: bool) -> Result<(), String> {
    hypr_input_keyword(key, if enabled { "true" } else { "false" })?;
    let mut s = load_pointer_settings();
    match key {
        "natural_scroll" => s.natural_scroll = enabled,
        "tap_to_click" => s.tap_to_click = enabled,
        "disable_while_typing" => s.disable_while_typing = enabled,
        "left_handed" => s.left_handed = enabled,
        _ => {}
    }
    persist_pointer_settings(&s);
    Ok(())
}

// --- Issue #8: Sound streams ---
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AudioStream {
    pub id: u32,
    pub app_name: String,
    pub volume: i32,
    pub muted: bool,
}

pub fn parse_pactl_sink_inputs(output: &str) -> Vec<AudioStream> {
    let mut streams = Vec::new();
    let mut cur_id: Option<u32> = None;
    let mut cur_app = String::new();
    let mut cur_vol: i32 = 100;
    let mut cur_mute = false;
    let mut in_block = false;
    for line in output.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Sink Input #") {
            if in_block && cur_id.is_some() {
                streams.push(AudioStream { id: cur_id.unwrap_or(0), app_name: if cur_app.is_empty() { "Unknown app".to_string() } else { cur_app.clone() }, volume: cur_vol, muted: cur_mute });
            }
            cur_id = rest.split_whitespace().next().and_then(|s| s.parse().ok());
            cur_app = String::new();
            cur_vol = 100;
            cur_mute = false;
            in_block = true;
        } else if in_block {
            if let Some(rest) = t.strip_prefix("application.name =") {
                cur_app = rest.trim().trim_matches('"').to_string();
            } else if t.starts_with("Mute:") {
                cur_mute = t.contains("yes");
            } else if t.starts_with("Volume:") {
                if let Some(pct) = t.split('/').nth(1) {
                    if let Some(num) = pct.trim().split('%').next() {
                        if let Ok(v) = num.trim().parse::<i32>() { cur_vol = v.clamp(0, 150); }
                    }
                }
            }
        }
    }
    if in_block && cur_id.is_some() {
        streams.push(AudioStream { id: cur_id.unwrap_or(0), app_name: if cur_app.is_empty() { "Unknown app".to_string() } else { cur_app }, volume: cur_vol, muted: cur_mute });
    }
    streams
}

pub fn list_sink_inputs() -> Vec<AudioStream> {
    if let Ok(o) = Command::new("pactl").args(["list", "sink-inputs"]).output() {
        let parsed = parse_pactl_sink_inputs(&String::from_utf8_lossy(&o.stdout));
        if !parsed.is_empty() { return parsed; }
    }
    Vec::new()
}

pub fn set_sink_input_volume(stream_id: u32, percent: i32) -> Result<(), String> {
    let p = percent.clamp(0, 150);
    let out = Command::new("pactl").args(["set-sink-input-volume", &stream_id.to_string(), &format!("{p}%")]).output()
        .map_err(|e| format!("failed to run pactl: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn query_source_volume() -> i32 {
    if let Ok(o) = Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"]).output() {
        let out = String::from_utf8_lossy(&o.stdout);
        if let Some(v) = out.split_whitespace().nth(1) {
            if let Ok(f) = v.parse::<f64>() { return (f * 100.0).round() as i32; }
        }
    }
    70
}

pub fn apply_source_volume(percent: i32) -> Result<(), String> {
    let p = percent.clamp(0, 150);
    let frac = p as f64 / 100.0;
    let out = Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", &format!("{frac:.2}")]).output()
        .map_err(|e| format!("failed to run wpctl: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn query_sink_balance() -> f64 {
    if let Some(f) = finick_data_file("audio_balance") {
        if let Ok(raw) = std::fs::read_to_string(f) {
            if let Ok(v) = raw.trim().parse::<f64>() { return v.clamp(-1.0, 1.0); }
        }
    }
    0.0
}

pub fn apply_sink_balance(balance: f64) -> Result<(), String> {
    let b = balance.clamp(-1.0, 1.0);
    let (l, r) = if b < 0.0 { (1.0, 1.0 + b) } else { (1.0 - b, 1.0) };
    let li = (l * 100.0).round() as i32;
    let ri = (r * 100.0).round() as i32;
    let out = Command::new("pactl").args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{li}%"), &format!("{ri}%")]).output();
    if let Ok(o) = out {
        if !o.status.success() {
            let _ = Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{:.2}", (li.max(ri) as f64)/100.0)]).output();
        }
    }
    if let Some(f) = finick_data_file("audio_balance") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(f, format!("{b:.2}"));
    }
    Ok(())
}

pub fn query_input_peak() -> f64 {
    if let Ok(o) = Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"]).output() {
        let out = String::from_utf8_lossy(&o.stdout);
        if out.contains("[MUTED]") { return 0.0; }
        if let Some(v) = out.split_whitespace().nth(1) {
            if let Ok(f) = v.parse::<f64>() { return (f * 100.0).clamp(0.0, 100.0); }
        }
    }
    0.0
}

// --- Issue #9: Power ---
#[derive(Clone, Debug, PartialEq)]
pub struct IdleSettings {
    pub dim_seconds: u32,
    pub blank_seconds: u32,
    pub suspend_seconds: u32,
}

impl Default for IdleSettings {
    fn default() -> Self { Self { dim_seconds: 120, blank_seconds: 300, suspend_seconds: 900 } }
}

pub fn parse_gsettings_u32(output: &str) -> Option<u32> {
    output.trim().parse::<u32>().ok()
}

pub fn load_idle_settings() -> IdleSettings {
    let mut s = IdleSettings::default();
    if let Some(f) = finick_data_file("idle.json") {
        if let Ok(raw) = std::fs::read_to_string(f) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(x) = v.get("dim_seconds").and_then(|x| x.as_u64()) { s.dim_seconds = x as u32; }
                if let Some(x) = v.get("blank_seconds").and_then(|x| x.as_u64()) { s.blank_seconds = x as u32; }
                if let Some(x) = v.get("suspend_seconds").and_then(|x| x.as_u64()) { s.suspend_seconds = x as u32; }
                return s;
            }
        }
    }
    if let Ok(o) = Command::new("gsettings").args(["get", "org.gnome.desktop.session", "idle-delay"]).output() {
        if let Some(v) = parse_gsettings_u32(&String::from_utf8_lossy(&o.stdout).replace("uint32", "").trim()) {
            if v > 0 { s.blank_seconds = v; }
        }
    }
    s
}

pub fn apply_idle_delay(kind: &str, seconds: u32) -> Result<(), String> {
    let mut s = load_idle_settings();
    match kind {
        "dim" => s.dim_seconds = seconds,
        "blank" => {
            s.blank_seconds = seconds;
            let _ = Command::new("gsettings").args(["set", "org.gnome.desktop.session", "idle-delay", &format!("uint32 {seconds}")]).output();
        }
        "suspend" => {
            s.suspend_seconds = seconds;
            let _ = Command::new("gsettings").args(["set", "org.gnome.settings-daemon.plugins.power", "sleep-inactive-ac-timeout", &seconds.to_string()]).output();
        }
        _ => return Err(format!("unknown idle kind: {kind}")),
    }
    if let Some(f) = finick_data_file("idle.json") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        let v = serde_json::json!({ "dim_seconds": s.dim_seconds, "blank_seconds": s.blank_seconds, "suspend_seconds": s.suspend_seconds });
        let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default());
    }
    Ok(())
}

pub fn load_lid_action() -> String {
    for path in [lid_dropin_path(), std::path::PathBuf::from("/etc/systemd/logind.conf")] {
        if let Ok(raw) = std::fs::read_to_string(&path) {
            for line in raw.lines() {
                let t = line.trim();
                if t.starts_with("HandleLidSwitch") && !t.starts_with('#') {
                    if let Some(v) = t.split('=').nth(1) {
                        let v = v.trim().to_string();
                        if !v.is_empty() {
                            return v;
                        }
                    }
                }
            }
        }
    }
    if let Some(f) = finick_data_file("lid_action") {
        if let Ok(raw) = std::fs::read_to_string(f) {
            let v = raw.trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
    }
    "suspend".to_string()
}

pub fn lid_dropin_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/etc/systemd/logind.conf.d/10-finick-lid.conf")
}

pub fn lid_dropin_body(action: &str) -> String {
    format!("[Login]\nHandleLidSwitch={action}\nHandleLidSwitchExternalPower={action}\nHandleLidSwitchDocked={action}\n")
}

/// Why the lid switch is system-managed (NixOS / admin policy), if it is.
pub fn lid_managed_reason() -> Option<String> {
    if lid_dropin_path().exists() {
        return Some("Managed by /etc/systemd/logind.conf.d/10-finick-lid.conf".to_string());
    }
    if let Ok(raw) = std::fs::read_to_string("/etc/systemd/logind.conf") {
        let pinned = raw.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#') && t.starts_with("HandleLidSwitch")
        });
        if pinned {
            return Some("Managed by system logind.conf — needs admin to change".to_string());
        }
    }
    None
}

/// Why lid-close control is unavailable on this machine, if it is.
pub fn lid_unavailable_reason() -> Option<String> {
    if std::process::Command::new("systemctl").arg("--version").output().is_err() {
        return Some("Unavailable: systemd-logind not found".to_string());
    }
    None
}

pub fn apply_lid_action(action: &str) -> Result<(), String> {
    let allowed = ["suspend", "hibernate", "ignore", "poweroff", "lock"];
    if !allowed.contains(&action) { return Err(format!("unsupported lid action: {action}")); }
    if let Some(f) = finick_data_file("lid_action") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(f, action);
    }
    let body = lid_dropin_body(action);
    if let Some(parent) = lid_dropin_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(lid_dropin_path(), body) {
        Ok(()) => {
            let _ = Command::new("systemctl").args(["restart", "systemd-logind"]).output();
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            Ok(())
        }
        Err(e) => Err(format!("failed to write logind drop-in (need admin): {e}")),
    }
}

/// Parse `upower --dump` charge history into 7 recent samples (0-100).
/// Returns None when no usable history exists so callers show an honest empty state.
pub fn parse_upower_dump_history(output: &str) -> Option<[f32; 7]> {
    let mut in_charge_history = false;
    let mut samples: Vec<f32> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        let lower = t.to_lowercase();
        if lower.starts_with("history (charge)") {
            in_charge_history = true;
            continue;
        }
        if lower.starts_with("history (rate)") || lower.starts_with("history (time") {
            if in_charge_history {
                break;
            }
            in_charge_history = false;
            continue;
        }
        if !in_charge_history {
            continue;
        }
        if t.is_empty() {
            if !samples.is_empty() {
                break;
            }
            continue;
        }
        let parts: Vec<&str> = t.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        if let Ok(pct) = parts[1].trim_end_matches('%').parse::<f32>() {
            if (0.0..=100.0).contains(&pct) {
                samples.push(pct);
            }
        }
    }
    if samples.len() < 2 {
        let mut fallback: Vec<f32> = Vec::new();
        for line in output.lines() {
            let t = line.trim().to_lowercase();
            if let Some(rest) = t.strip_prefix("percentage:") {
                if let Ok(pct) = rest.trim().trim_end_matches('%').parse::<f32>() {
                    if (0.0..=100.0).contains(&pct) {
                        fallback.push(pct);
                    }
                }
            }
        }
        if fallback.len() >= 7 {
            let tail = &fallback[fallback.len() - 7..];
            let mut arr = [0.0f32; 7];
            arr.copy_from_slice(tail);
            return Some(arr);
        }
        return None;
    }
    let n = samples.len().min(7);
    let tail = &samples[samples.len() - n..];
    let mut arr = [0.0f32; 7];
    if n == 7 {
        arr.copy_from_slice(tail);
    } else {
        let first = tail[0];
        for v in arr.iter_mut().take(7 - n) {
            *v = first;
        }
        arr[7 - n..].copy_from_slice(tail);
    }
    Some(arr)
}

pub fn query_battery_history() -> Option<[f32; 7]> {
    let out = Command::new("upower").arg("--dump").output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_upower_dump_history(&String::from_utf8_lossy(&out.stdout))
}

pub fn query_power_profile() -> String {
    if let Ok(o) = Command::new("powerprofilesctl").arg("get").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() { return s; }
    }
    "balanced".to_string()
}

pub fn list_power_profiles() -> Vec<String> {
    if let Ok(o) = Command::new("powerprofilesctl").arg("list").output() {
        if o.status.success() {
            let out = String::from_utf8_lossy(&o.stdout);
            let mut profiles = Vec::new();
            for line in out.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix('*') {
                    let name = rest.trim().split_whitespace().next().unwrap_or("").trim_end_matches(':').to_string();
                    if !name.is_empty() { profiles.push(name); }
                } else if let Some(rest) = t.strip_prefix('-').or_else(|| t.strip_prefix('+')) {
                    let name = rest.trim().split_whitespace().next().unwrap_or("").trim_end_matches(':').to_string();
                    if !name.is_empty() { profiles.push(name); }
                } else if t.ends_with(':') && !t.contains(' ') {
                    let name = t.trim_end_matches(':').to_string();
                    if !name.is_empty() && name != "Profiles" { profiles.push(name); }
                }
            }
            profiles.retain(|p| !p.is_empty());
            if !profiles.is_empty() { return profiles; }
        }
    }
    // Fallback to the well-known set; availability is gated by supports_power_profiles.
    vec!["performance".to_string(), "balanced".to_string(), "power-saver".to_string()]
}

pub fn supports_power_profiles() -> bool {
    if let Ok(o) = Command::new("powerprofilesctl").arg("list").output() {
        if o.status.success() { return true; }
    }
    if let Ok(o) = Command::new("powerprofilesctl").arg("get").output() {
        if o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty() { return true; }
    }
    false
}

pub fn apply_power_profile(profile: &str) -> Result<(), String> {
    let out = Command::new("powerprofilesctl").args(["set", profile]).output()
        .map_err(|e| format!("failed to run powerprofilesctl: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn charge_limit_path() -> Option<std::path::PathBuf> {
    std::fs::read_dir("/sys/class/power_supply").ok()?.flatten().find_map(|e| {
        let p = e.path().join("charge_control_end_threshold");
        if p.exists() { Some(p) } else { None }
    })
}

pub fn query_charge_limit() -> Option<u8> {
    let p = charge_limit_path()?;
    std::fs::read_to_string(p).ok()?.trim().parse::<u8>().ok()
}

pub fn apply_charge_limit(percent: u8) -> Result<(), String> {
    let p = charge_limit_path().ok_or_else(|| "Charge limit not supported on this hardware".to_string())?;
    let v = percent.clamp(40, 100);
    std::fs::write(p, format!("{v}\n")).map_err(|e| format!("failed to write charge limit (need permissions?): {e}"))?;
    if let Some(f) = finick_data_file("charge_limit") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(f, format!("{v}"));
    }
    Ok(())
}

// --- Issue #19: Quick settings parity (airplane, kbd backlight, background apps) ---

#[derive(Clone, Debug, PartialEq)]
pub struct RfkillDevice {
    pub id: String,
    pub kind: String,
    pub soft_blocked: bool,
    pub hard_blocked: bool,
}

pub fn parse_rfkill_list(output: &str) -> Vec<RfkillDevice> {
    let mut devices = Vec::new();
    let mut cur_id = String::new();
    let mut cur_kind = String::new();
    let mut cur_soft = false;
    let mut cur_hard = false;
    let mut in_block = false;
    for line in output.lines() {
        let t = line.trim();
        if t.contains(':') && !t.starts_with("Soft blocked") && !t.starts_with("Hard blocked") {
            if in_block && !cur_kind.is_empty() {
                devices.push(RfkillDevice {
                    id: cur_id.clone(),
                    kind: cur_kind.clone(),
                    soft_blocked: cur_soft,
                    hard_blocked: cur_hard,
                });
            }
            if let Some((id_part, kind_part)) = t.split_once(':') {
                cur_id = id_part.trim().to_string();
                cur_kind = kind_part.trim().to_lowercase();
                if cur_kind.contains("wireless") || cur_kind.contains("wlan") || cur_kind.contains("wifi") {
                    cur_kind = "wlan".to_string();
                } else if cur_kind.contains("bluetooth") || cur_kind.contains("bluetooth".to_uppercase().as_str()) {
                    cur_kind = "bluetooth".to_string();
                }
                cur_soft = false;
                cur_hard = false;
                in_block = !cur_id.is_empty() && cur_id.chars().next().is_some_and(|c| c.is_ascii_digit());
            } else {
                in_block = false;
            }
            continue;
        }
        if let Some(v) = t.strip_prefix("Soft blocked:") {
            cur_soft = v.trim().eq_ignore_ascii_case("yes");
        } else if let Some(v) = t.strip_prefix("Hard blocked:") {
            cur_hard = v.trim().eq_ignore_ascii_case("yes");
        }
    }
    if in_block && !cur_kind.is_empty() {
        devices.push(RfkillDevice { id: cur_id, kind: cur_kind, soft_blocked: cur_soft, hard_blocked: cur_hard });
    }
    devices.retain(|d| !d.id.is_empty() && (d.kind == "wlan" || d.kind == "bluetooth"));
    devices
}

pub fn airplane_from_states(wifi_enabled: bool, bt_soft_blocked: bool) -> bool {
    !wifi_enabled && bt_soft_blocked
}

pub fn query_airplane_mode() -> bool {
    let wifi_enabled = Command::new("nmcli")
        .args(["radio", "wifi"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().eq_ignore_ascii_case("enabled"))
        .unwrap_or(false);
    let bt_blocked = Command::new("rfkill")
        .args(["list", "bluetooth"])
        .output()
        .ok()
        .map(|o| parse_rfkill_list(&String::from_utf8_lossy(&o.stdout)).iter().any(|d| d.soft_blocked))
        .unwrap_or(false);
    airplane_from_states(wifi_enabled, bt_blocked)
}

pub fn apply_airplane_mode(enabled: bool) -> Result<(), String> {
    if enabled {
        let _ = Command::new("nmcli").args(["radio", "all", "off"]).output();
        let out = Command::new("rfkill").args(["block", "all"]).output()
            .map_err(|e| format!("failed to run rfkill: {e}"))?;
        if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
    } else {
        let _ = Command::new("rfkill").args(["unblock", "all"]).output();
        let _ = Command::new("nmcli").args(["radio", "all", "on"]).output();
        Ok(())
    }
}

pub fn keyboard_backlight_candidates() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/leds") {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower.contains("kbd") || lower.contains("keyboard") {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

pub fn keyboard_backlight_device() -> Option<String> {
    keyboard_backlight_candidates().into_iter().next()
}

pub fn kbd_pct_from_values(current: u64, max: u64) -> Option<u32> {
    if max == 0 { return None; }
    Some(((current * 100) / max).min(100) as u32)
}

pub fn kbd_value_from_pct(percent: u32, max: u64) -> u64 {
    ((percent.clamp(0, 100) as u64) * max) / 100
}

pub fn query_keyboard_backlight() -> Option<u32> {
    let dev = keyboard_backlight_device()?;
    let base = std::path::Path::new("/sys/class/leds").join(&dev);
    let cur: u64 = std::fs::read_to_string(base.join("brightness")).ok()?.trim().parse().ok()?;
    let max: u64 = std::fs::read_to_string(base.join("max_brightness")).ok()?.trim().parse().ok()?;
    kbd_pct_from_values(cur, max)
}

pub fn apply_keyboard_backlight(percent: u32) -> Result<(), String> {
    let dev = keyboard_backlight_device().ok_or_else(|| "No keyboard backlight found".to_string())?;
    let base = std::path::Path::new("/sys/class/leds").join(&dev);
    let max: u64 = std::fs::read_to_string(base.join("max_brightness"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(3);
    let target = kbd_value_from_pct(percent, max);
    if std::fs::write(base.join("brightness"), format!("{target}\n")).is_ok() {
        return Ok(());
    }
    let out = Command::new("brightnessctl").args(["-d", &dev, "set", &format!("{}%", percent.clamp(0, 100))]).output()
        .map_err(|e| format!("failed to run brightnessctl: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn parse_hypr_clients_classes(output: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.get("class").and_then(|k| k.as_str()).map(|s| s.to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub fn query_background_app_count() -> usize {
    Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .ok()
        .map(|o| parse_hypr_clients_classes(&String::from_utf8_lossy(&o.stdout)).len())
        .unwrap_or(0)
}

// --- Issue #10: Displays (modes / scale / primary / night schedule) ---
#[derive(Clone, Debug, PartialEq)]
pub struct NightSchedule {
    pub mode: usize,
    pub from: String,
    pub to: String,
}

impl Default for NightSchedule {
    fn default() -> Self { Self { mode: 0, from: "22:00".to_string(), to: "06:00".to_string() } }
}

pub fn parse_display_mode_refresh(mode: &str) -> Option<(String, f64)> {
    let (res, hz_part) = mode.split_once('@')?;
    let hz_str = hz_part.trim().trim_end_matches("Hz");
    let hz = hz_str.parse::<f64>().ok()?;
    Some((res.trim().to_string(), hz))
}

pub fn apply_display_scale(name: &str, scale: f64) -> Result<(), String> {
    let clamped = match scale {
        s if s < 0.5 => 0.5,
        s if s > 3.0 => 3.0,
        s => (s * 4.0).round() / 4.0,
    };
    HyprlandBackend::hyprctl_set_config(
        &format!("hl.monitor({{ output = \"{name}\", scale = {clamped} }})"),
        "monitor",
        &format!("{name},preferred,auto,{clamped}"),
    )
}

pub fn load_primary_display() -> Option<String> {
    finick_data_file("primary_display").and_then(|f| std::fs::read_to_string(f).ok()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn apply_primary_display(name: &str) -> Result<(), String> {
    if let Some(f) = finick_data_file("primary_display") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        std::fs::write(f, name).map_err(|e| format!("failed to save primary display: {e}"))?;
        Ok(())
    } else {
        Err("no config dir".to_string())
    }
}

pub fn load_night_schedule() -> NightSchedule {
    let mut s = NightSchedule::default();
    if let Some(f) = finick_data_file("night_schedule.json") {
        if let Ok(raw) = std::fs::read_to_string(f) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(m) = v.get("mode").and_then(|x| x.as_u64()) { s.mode = m as usize; }
                if let Some(x) = v.get("from").and_then(|x| x.as_str()) { s.from = x.to_string(); }
                if let Some(x) = v.get("to").and_then(|x| x.as_str()) { s.to = x.to_string(); }
            }
        }
    }
    s
}

pub fn apply_night_schedule(sched: &NightSchedule) -> Result<(), String> {
    if let Some(f) = finick_data_file("night_schedule.json") {
        if let Some(parent) = f.parent() { let _ = std::fs::create_dir_all(parent); }
        let v = serde_json::json!({ "mode": sched.mode, "from": sched.from, "to": sched.to });
        std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default())
            .map_err(|e| format!("failed to save night schedule: {e}"))?;
    }
    Ok(())
}

pub fn night_schedule_active(sched: &NightSchedule, now_hm: &str) -> bool {
    if sched.mode == 0 { return false; }
    if sched.mode == 1 { return true; }
    fn mins(s: &str) -> Option<i32> {
        let (h, m) = s.split_once(':')?;
        Some(h.parse::<i32>().ok()? * 60 + m.parse::<i32>().ok()?)
    }
    match (mins(&sched.from), mins(&sched.to), mins(now_hm)) {
        (Some(f), Some(t), Some(n)) => {
            if f <= t { n >= f && n < t } else { n >= f || n < t }
        }
        _ => false,
    }
}

// --- Issue #1: Network extras ---
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WiredDeviceInfo {
    pub interface: String,
    pub state: String,
    pub ip: String,
    pub dhcp: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct VpnInfo {
    pub name: String,
    pub vpn_type: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProxySettings {
    pub mode: String,
    pub http_host: String,
    pub http_port: u16,
    pub https_host: String,
    pub https_port: u16,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MobileDeviceInfo {
    pub interface: String,
    pub state: String,
    pub operator: String,
}

pub fn list_wired_devices() -> Vec<WiredDeviceInfo> {
    let mut out = Vec::new();
    let devs = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"]).output();
    let Ok(devs) = devs else { return out; };
    for line in String::from_utf8_lossy(&devs.stdout).lines() {
        let Some((device, ty, state)) = parse_nmcli_dev_line(line) else { continue; };
        if ty != "ethernet" && ty != "802-3-ethernet" { continue; }
        let mut ip = String::new();
        if let Ok(o) = Command::new("nmcli").args(["-t", "-f", "IP4.ADDRESS", "dev", "show", &device]).output() {
            for l in String::from_utf8_lossy(&o.stdout).lines() {
                let v = l.trim().strip_prefix("IP4.ADDRESS").map(|s| s.trim().trim_start_matches(':').trim().to_string()).unwrap_or_else(|| l.trim().to_string());
                if !v.is_empty() { ip = v.split('/').next().unwrap_or(&v).to_string(); break; }
            }
        }
        out.push(WiredDeviceInfo { interface: device, state, ip, dhcp: true });
    }
    out
}

pub fn apply_wired_dhcp(iface: &str, auto: bool) -> Result<(), String> {
    let method = if auto { "auto" } else { "manual" };
    let out = Command::new("nmcli").args(["con", "mod", iface, "ipv4.method", method]).output()
        .map_err(|e| format!("failed to run nmcli: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn list_vpn_connections() -> Vec<VpnInfo> {
    let mut out = Vec::new();
    let Ok(o) = Command::new("nmcli").args(["-t", "-f", "NAME,TYPE", "connection", "show"]).output() else { return out; };
    let active = Command::new("nmcli").args(["-t", "-f", "NAME,TYPE", "connection", "show", "--active"]).output()
        .map(|a| String::from_utf8_lossy(&a.stdout).to_string()).unwrap_or_default();
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 { continue; }
        if parts[1].contains("vpn") || parts[1].contains("wireguard") {
            out.push(VpnInfo { name: parts[0].to_string(), vpn_type: parts[1].to_string(), active: active.contains(parts[0]) });
        }
    }
    out
}

pub fn create_vpn_connection(name: &str, vpn_type: &str, gateway: &str) -> Result<(), String> {
    if name.trim().is_empty() { return Err("VPN name is required".to_string()); }
    if gateway.trim().is_empty() { return Err("Gateway is required".to_string()); }
    let vpn_data = format!("gateway={gateway}");
    let out = Command::new("nmcli").args(["connection", "add", "type", vpn_type, "con-name", name, "ifname", "--", "vpn.data", &vpn_data]).output()
        .map_err(|e| format!("failed to run nmcli: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn apply_vpn_state(name: &str, up: bool) -> Result<(), String> {
    let verb = if up { "up" } else { "down" };
    let out = Command::new("nmcli").args(["connection", verb, name]).output()
        .map_err(|e| format!("failed to run nmcli: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn load_proxy_settings() -> ProxySettings {
    let get = |schema: &str, key: &str| -> String {
        Command::new("gsettings").args(["get", schema, key]).output().map(|o| {
            String::from_utf8_lossy(&o.stdout).trim().trim_matches('\'').to_string()
        }).unwrap_or_default()
    };
    let mode = get("org.gnome.system.proxy", "mode");
    ProxySettings {
        mode: if mode.is_empty() { "none".to_string() } else { mode },
        http_host: get("org.gnome.system.proxy.http", "host"),
        http_port: get("org.gnome.system.proxy.http", "port").parse().unwrap_or(0),
        https_host: get("org.gnome.system.proxy.https", "host"),
        https_port: get("org.gnome.system.proxy.https", "port").parse().unwrap_or(0),
    }
}

pub fn apply_proxy_settings(proxy: &ProxySettings) -> Result<(), String> {
    let run = |schema: &str, key: &str, val: &str| -> Result<(), String> {
        let out = Command::new("gsettings").args(["set", schema, key, val]).output()
            .map_err(|e| format!("failed to run gsettings: {e}"))?;
        if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
    };
    run("org.gnome.system.proxy", "mode", &proxy.mode)?;
    if !proxy.http_host.is_empty() {
        run("org.gnome.system.proxy.http", "host", &proxy.http_host)?;
        run("org.gnome.system.proxy.http", "port", &proxy.http_port.to_string())?;
    }
    if !proxy.https_host.is_empty() {
        run("org.gnome.system.proxy.https", "host", &proxy.https_host)?;
        run("org.gnome.system.proxy.https", "port", &proxy.https_port.to_string())?;
    }
    Ok(())
}

pub fn list_mobile_devices() -> Vec<MobileDeviceInfo> {
    let mut out = Vec::new();
    let Ok(o) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"]).output() else { return out; };
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let Some((device, ty, state)) = parse_nmcli_dev_line(line) else { continue; };
        if ty.contains("gsm") || ty.contains("cdma") || ty.contains("wwan") || ty.contains("mobile") {
            out.push(MobileDeviceInfo { interface: device, state, operator: String::new() });
        }
    }
    out
}

pub trait SystemBackend {
    fn get_displays(&self) -> Vec<DisplayInfo>;
    fn get_input_devices(&self) -> InputDevices;
    fn get_audio_info(&self) -> AudioInfo;
    fn get_audio_sinks(&self) -> Vec<AudioDevice>;
    fn get_audio_sources(&self) -> Vec<AudioDevice>;
    fn set_default_audio_sink(&self, sink_id: &str) -> bool;
    fn set_volume(&self, percent: i32);
    fn supports_brightness(&self) -> bool;
    fn set_brightness(&self, percent: u32);
    fn toggle_mute(&self);
    fn get_power_info(&self) -> PowerInfo;
    fn get_general_info(&self) -> GeneralInfo;
    fn get_host_info(&self) -> HostInfo;
    fn get_current_user(&self) -> UserInfo;
    fn get_wifi_status(&self) -> bool;
    fn get_wifi_networks(&self) -> Vec<String>;
    fn get_wifi_details(&self) -> (CurrentWifiInfo, Vec<WifiNetworkDetail>);
    fn get_wired_info(&self) -> Option<WiredInfo>;
    fn connect_wifi(&self, ssid: &str);
    fn connect_wifi_with_password(&self, ssid: &str, password: Option<&str>, hidden: bool) -> Result<(), String> {
        let _ = (ssid, password, hidden);
        Ok(())
    }
    fn disconnect_wifi(&self);
    fn forget_wifi(&self, ssid: &str);
    fn set_wifi_status(&self, enabled: bool);
    fn get_bluetooth_status(&self) -> bool;
    fn set_bluetooth_status(&self, enabled: bool);
    fn get_storage_info(&self) -> StorageInfo;
    fn empty_trash(&self) -> Result<(), String> { Ok(()) }
    fn get_paired_bluetooth_devices(&self) -> Vec<BluetoothDevice>;
    fn connect_bluetooth_device(&self, mac: &str);
    fn disconnect_bluetooth_device(&self, mac: &str);
    fn remove_bluetooth_device(&self, mac: &str);
    fn pair_bluetooth_device(&self, mac: &str, pin: Option<&str>) -> Result<(), String> {
        let _ = (mac, pin);
        Ok(())
    }
    fn set_display_order(&self, ordered_names: &[String]) -> Result<(), String>;
    fn set_display_config(&self, name: &str, mode: Option<&str>, transform: Option<u8>) -> Result<(), String>;
    fn set_display_positions(&self, positions: &[(String, i32, i32)]) -> Result<(), String> {
        let _ = positions;
        Ok(())
    }
    fn set_night_shift(&self, enabled: bool, color_temp: f32) -> Result<(), String> {
        let _ = (enabled, color_temp);
        Ok(())
    }
    fn restore_display_configs(&self) -> Result<(), String> { Ok(()) }
    fn save_display_configs(&self) {}
    fn set_mute(&self, _muted: bool) {}
    fn log_out(&self) -> bool;
    fn reboot(&self) -> bool;
    fn power_off(&self) -> bool;
    fn sleep(&self) -> bool {
        Command::new("systemctl").arg("suspend").output().map(|o| o.status.success()).unwrap_or(false)
    }
    fn set_wallpaper(&self, target: &str) -> Result<(), String>;
    fn get_current_wallpaper(&self) -> Option<String>;
    fn get_keyboard_shortcuts(&self) -> Vec<KeyboardShortcut> { shortcuts::parse_hyprland_shortcuts() }
    fn add_custom_shortcut(&self, shortcut: &KeyboardShortcut) -> Result<(), String> { shortcuts::add_shortcut(shortcut) }
    fn remove_custom_shortcut(&self, id: &str) -> Result<(), String> { shortcuts::remove_shortcut(id) }
    fn get_default_applications(&self) -> Vec<DefaultAppCategoryInfo> { desktop_apps::get_default_apps() }
    fn set_default_application(&self, category_id: &str, desktop_id: &str) -> Result<(), String> {
        desktop_apps::set_default_app(category_id, desktop_id)
    }
    // --- Mouse & Touchpad (#6) ---
    fn get_pointer_settings(&self) -> PointerSettings { load_pointer_settings() }
    fn set_pointer_speed(&self, speed: f64) -> Result<(), String> { apply_pointer_speed(speed) }
    fn set_pointer_accel_profile(&self, profile: &str) -> Result<(), String> { apply_pointer_accel(profile) }
    fn set_natural_scroll(&self, enabled: bool) -> Result<(), String> { apply_input_bool("natural_scroll", enabled) }
    fn set_tap_to_click(&self, enabled: bool) -> Result<(), String> { apply_input_bool("tap_to_click", enabled) }
    fn set_disable_while_typing(&self, enabled: bool) -> Result<(), String> {
        apply_input_bool("disable_while_typing", enabled)
    }
    fn set_left_handed(&self, enabled: bool) -> Result<(), String> { apply_input_bool("left_handed", enabled) }
    // --- Sound per-app / input (#8) ---
    fn get_sink_inputs(&self) -> Vec<AudioStream> { list_sink_inputs() }
    fn set_stream_volume(&self, stream_id: u32, percent: i32) -> Result<(), String> {
        set_sink_input_volume(stream_id, percent)
    }
    fn get_source_volume(&self) -> i32 { query_source_volume() }
    fn set_source_volume(&self, percent: i32) -> Result<(), String> { apply_source_volume(percent) }
    fn get_sink_balance(&self) -> f64 { query_sink_balance() }
    fn set_sink_balance(&self, balance: f64) -> Result<(), String> { apply_sink_balance(balance) }
    fn get_input_peak(&self) -> f64 { query_input_peak() }
    // --- Power (#9) ---
    fn get_idle_settings(&self) -> IdleSettings { load_idle_settings() }
    fn set_idle_delay(&self, kind: &str, seconds: u32) -> Result<(), String> { apply_idle_delay(kind, seconds) }
    fn get_lid_action(&self) -> String { load_lid_action() }
    fn set_lid_action(&self, action: &str) -> Result<(), String> { apply_lid_action(action) }
    fn get_power_profile(&self) -> String { query_power_profile() }
    fn set_power_profile(&self, profile: &str) -> Result<(), String> { apply_power_profile(profile) }
    fn supports_power_profiles(&self) -> bool { supports_power_profiles() }
    fn list_power_profiles(&self) -> Vec<String> { list_power_profiles() }
    fn get_charge_limit(&self) -> Option<u8> { query_charge_limit() }
    fn set_charge_limit(&self, percent: u8) -> Result<(), String> { apply_charge_limit(percent) }
    // --- Displays (#10) ---
    fn get_display_modes(&self, name: &str) -> Vec<String> {
        self.get_displays().into_iter().find(|d| d.name == name).map(|d| d.available_modes).unwrap_or_default()
    }
    fn set_display_mode(&self, name: &str, mode: &str) -> Result<(), String> {
        self.set_display_config(name, Some(mode), None)
    }
    fn set_display_scale(&self, name: &str, scale: f64) -> Result<(), String> { apply_display_scale(name, scale) }
    fn get_primary_display(&self) -> Option<String> { load_primary_display() }
    fn set_primary_display(&self, name: &str) -> Result<(), String> { apply_primary_display(name) }
    fn get_night_schedule(&self) -> NightSchedule { load_night_schedule() }
    fn set_night_schedule(&self, sched: &NightSchedule) -> Result<(), String> { apply_night_schedule(sched) }
    // --- Network (#1) ---
    fn get_wired_devices(&self) -> Vec<WiredDeviceInfo> { list_wired_devices() }
    fn set_wired_dhcp(&self, iface: &str, auto: bool) -> Result<(), String> { apply_wired_dhcp(iface, auto) }
    fn get_vpn_connections(&self) -> Vec<VpnInfo> { list_vpn_connections() }
    fn add_vpn_connection(&self, name: &str, vpn_type: &str, gateway: &str) -> Result<(), String> {
        create_vpn_connection(name, vpn_type, gateway)
    }
    fn set_vpn_state(&self, name: &str, up: bool) -> Result<(), String> { apply_vpn_state(name, up) }
    fn get_proxy_settings(&self) -> ProxySettings { load_proxy_settings() }
    fn set_proxy_settings(&self, proxy: &ProxySettings) -> Result<(), String> { apply_proxy_settings(proxy) }
    fn get_mobile_devices(&self) -> Vec<MobileDeviceInfo> { list_mobile_devices() }
    // --- Quick settings parity (#19) ---
    fn get_airplane_mode(&self) -> bool { query_airplane_mode() }
    fn set_airplane_mode(&self, enabled: bool) -> Result<(), String> { apply_airplane_mode(enabled) }
    fn supports_keyboard_backlight(&self) -> bool { keyboard_backlight_device().is_some() }
    fn get_keyboard_backlight(&self) -> Option<u32> { query_keyboard_backlight() }
    fn set_keyboard_backlight(&self, percent: u32) -> Result<(), String> { apply_keyboard_backlight(percent) }
    fn get_background_app_count(&self) -> usize { query_background_app_count() }
    // --- Privacy & Security (#12) ---
    fn get_firewall(&self) -> FirewallStatus { query_firewall() }
    fn set_firewall_enabled(&self, enabled: bool) -> Result<(), String> { set_firewall_enabled(enabled) }
    fn get_location_status(&self) -> LocationStatus { query_location() }
    fn set_location_enabled(&self, enabled: bool) -> Result<(), String> { set_location_enabled(enabled) }
    fn get_camera_status(&self) -> CameraStatus { query_camera() }
    fn set_camera_enabled(&self, enabled: bool) -> Result<(), String> { set_camera_enabled(enabled) }
    fn get_mic_status(&self) -> MicStatus { query_mic() }
    fn set_mic_enabled(&self, enabled: bool) -> Result<(), String> { set_mic_enabled(enabled) }
    fn get_usb_status(&self) -> UsbStatus { query_usb_protection() }
    fn set_usb_protection(&self, enabled: bool) -> Result<(), String> { set_usb_protection(enabled) }
    fn get_lock_delay(&self) -> u32 { query_lock_delay() }
    fn set_lock_delay(&self, seconds: u32) -> Result<(), String> { set_lock_delay(seconds) }
    fn clear_history(&self) -> Result<HistoryClearReport, String> { clear_file_history() }
    fn get_diagnostics(&self) -> bool { query_diagnostics() }
    fn set_diagnostics(&self, enabled: bool) -> Result<(), String> { set_diagnostics(enabled) }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PersistentDisplayConfig {
    pub name: String,
    pub mode: String,
    pub position: String,
    pub scale: String,
    pub transform: u8,
}

pub struct HyprlandBackend;

impl HyprlandBackend {
    fn hyprctl_try_eval(lua: &str) -> bool {
        if let Ok(o) = Command::new("hyprctl").args(["eval", lua]).output() {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            return o.status.success()
                && (out.starts_with("ok") || (!out.contains("error") && !out.contains("fail") && err.is_empty()));
        }
        false
    }

    fn hyprctl_keyword(keyword: &str, val: &str, ctx: &str) -> Result<(), String> {
        let o = Command::new("hyprctl")
            .args(["keyword", keyword, val])
            .output()
            .map_err(|e| format!("failed to spawn hyprctl: {e}"))?;
        let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
        let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
        if out.contains("can't work") || out.contains("error") || !o.status.success() {
            let detail = if !err.is_empty() { err } else { out };
            return Err(format!("failed to configure {ctx}: {detail}"));
        }
        Ok(())
    }

    pub fn hyprctl_set_config(lua_code: &str, legacy_keyword: &str, legacy_val: &str) -> Result<(), String> {
        if Self::hyprctl_try_eval(lua_code) {
            return Ok(());
        }
        Self::hyprctl_keyword(legacy_keyword, legacy_val, "hyprland")
    }

    pub fn apply_monitor_config(name: &str, mode: &str, position: &str, scale: &str, transform: u8) -> Result<(), String> {
        let scale_val: f64 = scale.parse().unwrap_or(1.0);
        let lua = format!(
            r#"hl.monitor({{ output = "{name}", mode = "{mode}", position = "{position}", scale = {scale_val}, transform = {transform} }})"#
        );
        if Self::hyprctl_try_eval(&lua) {
            return Ok(());
        }
        Self::hyprctl_keyword(
            "monitor",
            &format!("{name},{mode},{position},{scale_val},transform,{transform}"),
            &format!("monitor {name}"),
        )
    }
}

impl SystemBackend for HyprlandBackend {
    fn supports_brightness(&self) -> bool {
        if let Ok(entries) = std::fs::read_dir("/sys/class/backlight") {
            return entries.count() > 0;
        }
        false
    }

    fn set_brightness(&self, percent: u32) {
        let _ = Command::new("brightnessctl").args(["set", &format!("{}%", percent)]).output();
    }

    fn get_displays(&self) -> Vec<DisplayInfo> {
        if let Ok(output) = Command::new("hyprctl").args(["monitors", "-j"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            parse_hyprctl_monitors(&out)
        } else {
            Vec::new()
        }
    }

    fn set_display_order(&self, ordered_names: &[String]) -> Result<(), String> {
        if ordered_names.is_empty() {
            return Ok(());
        }
        let displays = self.get_displays();
        let disp_map: HashMap<String, DisplayInfo> = displays.into_iter().map(|d| (d.name.clone(), d)).collect();

        let mut x: i32 = 0;
        for name in ordered_names {
            if let Some(d) = disp_map.get(name) {
                let (w, h) = parse_display_dimensions(&d.resolution);
                let effective_w = if d.transform == 1 || d.transform == 3 { h } else { w };
                let pos = format!("{x}x0");
                let mode = if !d.resolution.is_empty() && !d.refresh_rate.is_empty() {
                    if let Ok(hz) = d.refresh_rate.parse::<f64>() {
                        format!("{}@{:.2}Hz", d.resolution, hz)
                    } else {
                        format!("{}@{}", d.resolution, d.refresh_rate)
                    }
                } else if !d.resolution.is_empty() {
                    d.resolution.clone()
                } else {
                    "preferred".to_string()
                };
                Self::apply_monitor_config(name, &mode, &pos, &d.scale, d.transform)?;
                x = x.saturating_add(effective_w as i32);
            }
        }
        self.save_display_configs();
        Ok(())
    }

    fn set_display_config(&self, name: &str, mode: Option<&str>, transform: Option<u8>) -> Result<(), String> {
        let displays = self.get_displays();
        let Some(target) = displays.iter().find(|d| d.name == name) else {
            return Err(format!("monitor {name} not found"));
        };
        let final_mode = mode.map(|m| m.to_string()).unwrap_or_else(|| {
            if !target.resolution.is_empty() && !target.refresh_rate.is_empty() {
                if let Ok(hz) = target.refresh_rate.parse::<f64>() {
                    format!("{}@{:.2}Hz", target.resolution, hz)
                } else {
                    format!("{}@{}", target.resolution, target.refresh_rate)
                }
            } else if !target.resolution.is_empty() {
                target.resolution.clone()
            } else {
                "preferred".to_string()
            }
        });
        let final_transform = transform.unwrap_or(target.transform);
        let pos = format!("{}x{}", target.x, target.y);
        Self::apply_monitor_config(name, &final_mode, &pos, &target.scale, final_transform)?;
        self.save_display_configs();
        Ok(())
    }

    fn set_display_positions(&self, positions: &[(String, i32, i32)]) -> Result<(), String> {
        if positions.is_empty() {
            return Ok(());
        }
        let displays = self.get_displays();
        let disp_map: HashMap<String, DisplayInfo> = displays.into_iter().map(|d| (d.name.clone(), d)).collect();
        let mut adjusted: Vec<(String, i32, i32)> = Vec::new();
        for (name, mut x, mut y) in positions.iter().cloned() {
            if adjusted.is_empty() {
                x = 0;
                y = 0;
            } else if let Some(d) = disp_map.get(&name) {
                let (rw, rh) = parse_display_dimensions(&d.resolution);
                let sc: f64 = d.scale.parse().unwrap_or(1.0);
                let sc = if sc <= 0.0 { 1.0 } else { sc };
                let mut sw = (rw as f64 / sc).round() as i32;
                let mut sh = (rh as f64 / sc).round() as i32;
                if d.transform == 1 || d.transform == 3 {
                    std::mem::swap(&mut sw, &mut sh);
                }
                let mut best: Option<(i32, i32, i32)> = None;
                for (on, ox, oy) in &adjusted {
                    if let Some(od) = disp_map.get(on) {
                        let (orw, orh) = parse_display_dimensions(&od.resolution);
                        let osc: f64 = od.scale.parse().unwrap_or(1.0);
                        let osc = if osc <= 0.0 { 1.0 } else { osc };
                        let mut ow = (orw as f64 / osc).round() as i32;
                        let mut oh = (orh as f64 / osc).round() as i32;
                        if od.transform == 1 || od.transform == 3 {
                            std::mem::swap(&mut ow, &mut oh);
                        }
                        for (cx, cy) in [(*ox + ow, *oy), (*ox - sw, *oy), (*ox, *oy + oh), (*ox, *oy - sh)] {
                            let dx = x - cx;
                            let dy = y - cy;
                            let dist = dx * dx + dy * dy;
                            if best.is_none() || dist < best.unwrap().0 {
                                best = Some((dist, cx, cy));
                            }
                        }
                    }
                }
                if let Some((_, bx, by)) = best {
                    x = bx;
                    y = by;
                }
            }
            x = x.clamp(-3000, 3000);
            y = y.clamp(-2000, 2000);
            adjusted.push((name, x, y));
        }
        for (name, x, y) in &adjusted {
            if let Some(d) = disp_map.get(name) {
                let pos = format!("{x}x{y}");
                let mode = if !d.resolution.is_empty() && !d.refresh_rate.is_empty() {
                    if let Ok(hz) = d.refresh_rate.parse::<f64>() {
                        format!("{}@{:.2}Hz", d.resolution, hz)
                    } else {
                        format!("{}@{}", d.resolution, d.refresh_rate)
                    }
                } else if !d.resolution.is_empty() {
                    d.resolution.clone()
                } else {
                    "preferred".to_string()
                };
                Self::apply_monitor_config(name, &mode, &pos, &d.scale, d.transform)?;
            }
        }
        self.save_display_configs();
        Ok(())
    }

    fn set_night_shift(&self, enabled: bool, color_temp: f32) -> Result<(), String> {
        apply_night_shift(enabled, color_temp);
        Ok(())
    }

    fn save_display_configs(&self) {
        if let Some(dir) = get_finick_config_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let displays = self.get_displays();
            if displays.is_empty() {
                return;
            }
            let configs: Vec<PersistentDisplayConfig> = displays
                .into_iter()
                .map(|d| {
                    let pos = format!("{}x{}", d.x, d.y);
                    let mode = if !d.resolution.is_empty() && !d.refresh_rate.is_empty() {
                        if let Ok(hz) = d.refresh_rate.parse::<f64>() {
                            format!("{}@{:.2}Hz", d.resolution, hz)
                        } else {
                            format!("{}@{}", d.resolution, d.refresh_rate)
                        }
                    } else if !d.resolution.is_empty() {
                        d.resolution
                    } else {
                        "preferred".to_string()
                    };
                    PersistentDisplayConfig {
                        name: d.name,
                        mode,
                        position: pos,
                        scale: if d.scale.is_empty() { "1".to_string() } else { d.scale },
                        transform: d.transform,
                    }
                })
                .collect();
            if let Ok(json) = serde_json::to_string_pretty(&configs) {
                let _ = std::fs::write(dir.join("monitors.json"), json);
            }
        }
    }

    fn restore_display_configs(&self) -> Result<(), String> {
        if let Some(dir) = get_finick_config_dir() {
            let file = dir.join("monitors.json");
            if let Ok(content) = std::fs::read_to_string(file) {
                if let Ok(configs) = serde_json::from_str::<Vec<PersistentDisplayConfig>>(&content) {
                    let current = self.get_displays();
                    let current_names: std::collections::HashSet<String> = current.into_iter().map(|d| d.name).collect();
                    for c in configs {
                        if current_names.contains(&c.name) {
                            let _ = Self::apply_monitor_config(&c.name, &c.mode, &c.position, &c.scale, c.transform);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn get_input_devices(&self) -> InputDevices {
        let mut mice = Vec::new();
        let mut keyboards = Vec::new();
        if let Ok(output) = Command::new("hyprctl").args(["devices", "-j"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            let mut in_mice = false;
            let mut in_kb = false;
            let mut current_name = String::new();
            let mut current_layout = String::new();
            for line in out.lines() {
                let l = line.trim();
                if l.starts_with("\"mice\":") {
                    in_mice = true;
                    in_kb = false;
                } else if l.starts_with("\"keyboards\":") {
                    in_mice = false;
                    in_kb = true;
                } else if l.starts_with("\"tablets\":") {
                    in_kb = false;
                }
                if in_mice {
                    if l.starts_with("\"name\":") {
                        current_name = l
                            .split(':')
                            .nth(1)
                            .unwrap_or_default()
                            .trim_matches(|c| c == '\"' || c == ',' || c == ' ')
                            .to_string();
                    }
                    if (l == "}," || l == "}") && !current_name.is_empty() {
                        mice.push(InputDevice { name: current_name.clone(), layout_or_type: "Pointer Device".to_string() });
                        current_name.clear();
                    }
                }
                if in_kb {
                    if l.starts_with("\"name\":") {
                        current_name = l
                            .split(':')
                            .nth(1)
                            .unwrap_or_default()
                            .trim_matches(|c| c == '\"' || c == ',' || c == ' ')
                            .to_string();
                    }
                    if l.starts_with("\"active_keymap\":") {
                        current_layout = l
                            .split(':')
                            .nth(1)
                            .unwrap_or_default()
                            .trim_matches(|c| c == '\"' || c == ',' || c == ' ')
                            .to_string();
                    }
                    if (l == "}," || l == "}") && !current_name.is_empty() {
                        keyboards.push(InputDevice {
                            name: current_name.clone(),
                            layout_or_type: if current_layout.is_empty() {
                                "Keyboard".to_string()
                            } else {
                                current_layout.clone()
                            },
                        });
                        current_name.clear();
                        current_layout.clear();
                    }
                }
            }
        }
        InputDevices { mice, keyboards }
    }

    fn get_audio_info(&self) -> AudioInfo {
        let mut current_vol = 50.0;
        let mut current_mute = false;
        let mut sink_name = "Unknown Device".to_string();
        if let Ok(output) = Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            if out.contains("[MUTED]") {
                current_mute = true;
            }
            if let Some(vol_str) = out.split_whitespace().nth(1)
                && let Ok(v) = vol_str.parse::<f64>()
            {
                current_vol = v * 100.0;
            }
        }
        let mut source_name = "Unknown Device".to_string();
        if let Ok(output) = Command::new("wpctl").args(["status"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            let (sinks, sources) = parse_wpctl_devices(&out);
            if let Some(default) = sinks.iter().find(|d| d.is_default) {
                sink_name = default.name.clone();
            } else if let Some(first) = sinks.first() {
                sink_name = first.name.clone();
            }
            if let Some(default) = sources.iter().find(|d| d.is_default) {
                source_name = default.name.clone();
            } else if let Some(first) = sources.first() {
                source_name = first.name.clone();
            }
        }
        AudioInfo {
            volume: current_vol,
            is_muted: current_mute,
            default_sink_name: sink_name,
            default_source_name: source_name,
        }
    }

    fn get_audio_sinks(&self) -> Vec<AudioDevice> {
        Command::new("wpctl")
            .args(["status"])
            .output()
            .map(|o| parse_wpctl_devices(&String::from_utf8_lossy(&o.stdout)).0)
            .unwrap_or_default()
    }

    fn get_audio_sources(&self) -> Vec<AudioDevice> {
        Command::new("wpctl")
            .args(["status"])
            .output()
            .map(|o| parse_wpctl_devices(&String::from_utf8_lossy(&o.stdout)).1)
            .unwrap_or_default()
    }

    fn set_default_audio_sink(&self, sink_id: &str) -> bool {
        if sink_id.trim().is_empty() {
            return false;
        }
        Command::new("wpctl").args(["set-default", sink_id]).output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn set_volume(&self, percent: i32) {
        // wpctl `VOL%` steps volume relatively (`65%` would add ~65% on top
        // of the current level). Absolute sets use a bare float (1.0 = 100%).
        let clamped = percent.clamp(0, 150);
        let v_str = format!("{:.2}", clamped as f64 / 100.0);
        let _ = Command::new("wpctl").args(["set-volume", "@DEFAULT_AUDIO_SINK@", &v_str]).output();
    }

    fn toggle_mute(&self) { let _ = Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).output(); }

    fn set_mute(&self, muted: bool) {
        let arg = if muted { "1" } else { "0" };
        let _ = Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", arg]).output();
    }

    fn get_power_info(&self) -> PowerInfo {
        let mut capacity = "Unknown".to_string();
        let mut status = "Unknown".to_string();
        let mut health_percent = None;
        let mut cycle_count = None;
        if let Ok(paths) = std::fs::read_dir("/sys/class/power_supply") {
            for path in paths.flatten() {
                let name = path.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    if let Ok(c) = std::fs::read_to_string(path.path().join("capacity")) {
                        capacity = format!("{}%", c.trim());
                    }
                    if let Ok(s) = std::fs::read_to_string(path.path().join("status")) {
                        status = s.trim().to_string();
                    }
                    let dir = path.path();
                    let full = read_sysfs_u64(&dir, &["charge_full", "energy_full"]);
                    let design = read_sysfs_u64(&dir, &["charge_full_design", "energy_full_design"]);
                    if let (Some(f), Some(d)) = (full, design) {
                        health_percent = compute_battery_health(f, d);
                    }
                    cycle_count = read_sysfs_u64(&dir, &["cycle_count"]).and_then(|c| u32::try_from(c).ok());
                    break;
                }
            }
        }
        PowerInfo { capacity, status, health_percent, cycle_count }
    }

    fn get_general_info(&self) -> GeneralInfo {
        let locale = Command::new("locale")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
            .lines()
            .find(|l| l.starts_with("LANG="))
            .unwrap_or("LANG=Unknown")
            .replace("LANG=", "");
        let mut timezone = "Unknown".to_string();
        if let Ok(tdctl) = Command::new("timedatectl").output() {
            let out = String::from_utf8_lossy(&tdctl.stdout).to_string();
            if let Some(line) = out.lines().find(|l| l.contains("Time zone:")) {
                timezone =
                    line.split(':').nth(1).unwrap_or_default().split('(').next().unwrap_or_default().trim().to_string();
            }
        }
        GeneralInfo { locale, timezone }
    }

    fn get_host_info(&self) -> HostInfo {
        let hostname = std::fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        let os_name = std::fs::read_to_string("/etc/os-release")
            .unwrap_or_default()
            .lines()
            .find(|l| l.starts_with("PRETTY_NAME="))
            .map(|l| l.replace("PRETTY_NAME=", "").replace("\"", "").trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        let kernel = Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        let uptime = Command::new("uptime")
            .arg("-p")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().replace("up ", ""))
            .unwrap_or_else(|| "Unknown".to_string());
        let memory = Command::new("free")
            .arg("-h")
            .output()
            .ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.lines().nth(1).unwrap_or("").split_whitespace().nth(2).unwrap_or("Unknown").to_string()
            })
            .unwrap_or_else(|| "Unknown".to_string());
        HostInfo { hostname, os_name, kernel, uptime, memory }
    }

    fn get_current_user(&self) -> UserInfo {
        let username = Command::new("id")
            .arg("-un")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("USER").ok().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| "Unknown".to_string());
        let uid = Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        UserInfo { username, uid }
    }

    fn get_wifi_networks(&self) -> Vec<String> {
        let mut networks = std::collections::HashSet::new();
        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "SSID", "dev", "wifi"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed != "--" {
                    networks.insert(trimmed.to_string());
                }
            }
        }
        let mut sorted: Vec<String> = networks.into_iter().collect();
        sorted.sort();
        sorted
    }

    fn get_wifi_details(&self) -> (CurrentWifiInfo, Vec<WifiNetworkDetail>) {
        let mut saved_ssids = std::collections::HashSet::new();
        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "NAME,TYPE", "connection", "show"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 && (parts[1] == "802-11-wireless" || parts[1] == "wifi") {
                    saved_ssids.insert(parts[0].to_string());
                }
            }
        }

        let mut wifi_interface = None;
        let mut wifi_dev_connected = false;
        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 && parts[1] == "wifi" {
                    wifi_interface = Some(parts[0].to_string());
                    if parts[2].starts_with("connected") {
                        wifi_dev_connected = true;
                    }
                }
            }
        }

        let mut networks_map: std::collections::HashMap<String, WifiNetworkDetail> = std::collections::HashMap::new();
        let mut active_ssid = None;
        let mut active_sec = None;

        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "active,ssid,signal,security", "dev", "wifi"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let Some((active, ssid, signal, sec)) = parse_nmcli_wifi_line(line) else { continue };
                if active {
                    active_ssid = Some(ssid.clone());
                    active_sec = Some(sec.clone());
                }

                let is_saved = saved_ssids.contains(ssid.as_str());
                let entry = networks_map.entry(ssid.clone()).or_insert_with(|| WifiNetworkDetail {
                    ssid: ssid.clone(),
                    signal,
                    security: sec.clone(),
                    is_connected: active,
                    is_saved,
                });
                if active {
                    entry.is_connected = true;
                }
                if signal > entry.signal {
                    entry.signal = signal;
                }
            }
        }

        let mut current_ip = None;
        if wifi_dev_connected
            && let Some(ref iface) = wifi_interface
            && let Ok(output) = Command::new("nmcli").args(["-t", "-f", "IP4.ADDRESS", "dev", "show", iface]).output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Some(ip) = trimmed.strip_prefix("IP4.ADDRESS:") {
                    current_ip = Some(ip.split('/').next().unwrap_or(ip).trim().to_string());
                    break;
                } else if !trimmed.is_empty() {
                    current_ip = Some(trimmed.split('/').next().unwrap_or(trimmed).to_string());
                    break;
                }
            }
        }
        if current_ip.is_none()
            && let Ok(output) = Command::new("ip").args(["route", "get", "1.1.1.1"]).output()
        {
            let out = String::from_utf8_lossy(&output.stdout);
            if let Some((route_iface, src)) = parse_ip_route_get(&out) {
                current_ip = Some(src);
                if wifi_interface.is_none() {
                    wifi_interface = Some(route_iface);
                }
            }
        }

        let current_info = CurrentWifiInfo {
            connected: wifi_dev_connected && active_ssid.is_some(),
            ssid: active_ssid,
            ip_address: current_ip,
            security: active_sec,
            interface: wifi_interface,
        };

        let mut networks: Vec<WifiNetworkDetail> = networks_map.into_values().collect();
        networks.sort_by(|a, b| {
            b.is_connected
                .cmp(&a.is_connected)
                .then_with(|| b.is_saved.cmp(&a.is_saved))
                .then_with(|| b.signal.cmp(&a.signal))
        });

        (current_info, networks)
    }

    fn get_wired_info(&self) -> Option<WiredInfo> {
        let mut iface = None;
        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let Some((device, ty, state)) = parse_nmcli_dev_line(line) else { continue };
                if (ty == "ethernet" || ty == "802-3-ethernet") && state.starts_with("connected") {
                    iface = Some(device);
                    break;
                }
            }
        }
        let iface = iface?;
        let mut profile = String::new();
        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "NAME,TYPE,DEVICE", "c", "show", "--active"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let Some((name, ty, device)) = parse_nmcli_dev_line(line) else { continue };
                if device == iface && (ty == "ethernet" || ty == "802-3-ethernet" || ty == "wired") {
                    profile = name;
                    break;
                }
            }
        }
        let mut ip = String::new();
        if let Ok(output) = Command::new("ip").args(["route", "get", "1.1.1.1"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            if let Some((route_iface, src)) = parse_ip_route_get(&out)
                && route_iface == iface
            {
                ip = src;
            }
        }
        Some(WiredInfo { connected: true, interface: iface, ip_address: ip, profile })
    }

    fn connect_wifi(&self, ssid: &str) { let _ = Command::new("nmcli").args(["dev", "wifi", "connect", ssid]).output(); }

    fn connect_wifi_with_password(&self, ssid: &str, password: Option<&str>, hidden: bool) -> Result<(), String> {
        let mut args = vec!["dev", "wifi", "connect", ssid];
        let pwd_str;
        if let Some(pwd) = password {
            if !pwd.is_empty() {
                pwd_str = pwd.to_string();
                args.push("password");
                args.push(&pwd_str);
            }
        }
        if hidden {
            args.push("hidden");
            args.push("yes");
        }
        let output = Command::new("nmcli").args(&args).output().map_err(|e| format!("failed to run nmcli: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let err = if !stderr.is_empty() { stderr } else { stdout };
            return Err(if err.is_empty() { "Connection failed".to_string() } else { err });
        }
        Ok(())
    }

    fn disconnect_wifi(&self) {
        if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE", "dev"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 && parts[1] == "wifi" {
                    let _ = Command::new("nmcli").args(["dev", "disconnect", parts[0]]).output();
                    break;
                }
            }
        }
    }

    fn forget_wifi(&self, ssid: &str) { let _ = Command::new("nmcli").args(["connection", "delete", "id", ssid]).output(); }

    fn get_wifi_status(&self) -> bool {
        Command::new("nmcli")
            .args(["radio", "wifi"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
            .unwrap_or(false)
    }

    fn set_wifi_status(&self, enabled: bool) {
        let _ = Command::new("nmcli").args(["radio", "wifi", if enabled { "on" } else { "off" }]).output();
    }

    fn get_bluetooth_status(&self) -> bool {
        Command::new("rfkill")
            .args(["list", "bluetooth"])
            .output()
            .ok()
            .map(|o| !String::from_utf8_lossy(&o.stdout).contains("Soft blocked: yes"))
            .unwrap_or(false)
    }

    fn set_bluetooth_status(&self, enabled: bool) {
        let _ = Command::new("rfkill").args([if enabled { "unblock" } else { "block" }, "bluetooth"]).output();
    }

    fn get_storage_info(&self) -> StorageInfo {
        let mut disks = Vec::new();
        let mut root_disk = DiskInfo {
            filesystem: "Unknown".to_string(),
            mount_point: "/".to_string(),
            total: "Unknown".to_string(),
            used: "Unknown".to_string(),
            available: "Unknown".to_string(),
            use_percent: 0.0,
        };
        if let Ok(output) = std::process::Command::new("df").args(["-hP"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            for line in out.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let (fs, size, used, avail, pct_str, mount) =
                        (parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
                    let pct = pct_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
                    if fs.starts_with("tmpfs")
                        || fs.starts_with("devtmpfs")
                        || fs.starts_with("efivarfs")
                        || fs.starts_with("none")
                        || fs.starts_with("overlay")
                        || mount.starts_with("/run")
                        || mount.starts_with("/sys")
                        || mount.starts_with("/dev")
                    {
                        continue;
                    }
                    let disk = DiskInfo {
                        filesystem: fs.to_string(),
                        mount_point: mount.to_string(),
                        total: size.to_string(),
                        used: used.to_string(),
                        available: avail.to_string(),
                        use_percent: pct,
                    };
                    if mount == "/" {
                        root_disk = disk.clone();
                    }
                    disks.push(disk);
                }
            }
        }
        StorageInfo {
            total: root_disk.total.clone(),
            used: root_disk.used.clone(),
            available: root_disk.available.clone(),
            use_percent: root_disk.use_percent,
            disks,
        }
    }

    fn empty_trash(&self) -> Result<(), String> {
        let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        let trash_files = home.join(".local/share/Trash/files");
        let trash_info = home.join(".local/share/Trash/info");
        for dir in [trash_files, trash_info] {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(&dir);
                let _ = std::fs::create_dir_all(&dir);
            }
        }
        Ok(())
    }

    fn get_paired_bluetooth_devices(&self) -> Vec<BluetoothDevice> {
        let mut devices = Vec::new();
        let mut connected_macs = HashSet::new();
        if let Ok(output) = Command::new("bluetoothctl").args(["devices", "Connected"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "Device" {
                    connected_macs.insert(parts[1].to_uppercase());
                }
            }
        }
        let out_str = Command::new("bluetoothctl")
            .args(["devices"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        for line in out_str.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Device ") {
                let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let mac = parts[1].trim().to_string();
                    let raw_name = if parts.len() >= 3 && !parts[2].trim().is_empty() {
                        parts[2].trim().to_string()
                    } else {
                        String::new()
                    };
                    let clean = raw_name.replace([':', '-'], "");
                    let name = if raw_name.is_empty() || (clean.len() == 12 && clean.chars().all(|c| c.is_ascii_hexdigit()))
                    {
                        // Query bluetoothctl info for Alias or Name
                        Command::new("bluetoothctl")
                            .args(["info", &mac])
                            .output()
                            .ok()
                            .and_then(|o| {
                                let s = String::from_utf8_lossy(&o.stdout);
                                let mut alias = None;
                                let mut name = None;
                                for l in s.lines() {
                                    let t = l.trim();
                                    if let Some(rest) = t.strip_prefix("Alias:") {
                                        alias = Some(rest.trim().to_string());
                                    }
                                    if let Some(rest) = t.strip_prefix("Name:") {
                                        name = Some(rest.trim().to_string());
                                    }
                                }
                                alias.or(name)
                            })
                            .unwrap_or_else(|| {
                                if !raw_name.is_empty() {
                                    raw_name
                                } else {
                                    format!("Device ({})", &mac[..mac.len().min(8)])
                                }
                            })
                    } else {
                        raw_name
                    };
                    let is_connected = connected_macs.contains(&mac.to_uppercase());
                    if !devices.iter().any(|d: &BluetoothDevice| d.mac.eq_ignore_ascii_case(&mac)) {
                        devices.push(BluetoothDevice { mac, name, connected: is_connected });
                    }
                }
            }
        }
        devices
    }

    fn connect_bluetooth_device(&self, mac: &str) { let _ = Command::new("bluetoothctl").args(["connect", mac]).output(); }

    fn disconnect_bluetooth_device(&self, mac: &str) {
        let _ = Command::new("bluetoothctl").args(["disconnect", mac]).output();
    }

    fn remove_bluetooth_device(&self, mac: &str) { let _ = Command::new("bluetoothctl").args(["remove", mac]).output(); }

    fn pair_bluetooth_device(&self, mac: &str, pin: Option<&str>) -> Result<(), String> {
        use std::io::Write;
        let mut child = Command::new("bluetoothctl")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to run bluetoothctl: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            let pin_cmd =
                if let Some(p) = pin { if !p.is_empty() { format!("{p}\n") } else { String::new() } } else { String::new() };
            let script = format!("agent on\ndefault-agent\npair {mac}\n{pin_cmd}yes\ntrust {mac}\nconnect {mac}\nquit\n");
            let _ = stdin.write_all(script.as_bytes());
        }

        let output = child.wait_with_output().map_err(|e| format!("bluetoothctl wait failed: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Failed to pair")
            || stdout.contains("AuthenticationFailed")
            || stdout.contains("AuthenticationTimeout")
        {
            let detail =
                stdout.lines().find(|l| l.contains("Failed") || l.contains("Authentication")).unwrap_or("Pairing failed");
            return Err(detail.trim().to_string());
        }
        Ok(())
    }

    fn log_out(&self) -> bool {
        Command::new("loginctl")
            .args(["terminate-user", &std::env::var("USER").unwrap_or_default()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn reboot(&self) -> bool {
        Command::new("systemctl").arg("reboot").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn power_off(&self) -> bool {
        Command::new("systemctl").arg("poweroff").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn sleep(&self) -> bool {
        Command::new("systemctl").arg("suspend").output().map(|o| o.status.success()).unwrap_or_else(|_| {
            Command::new("loginctl").arg("suspend").output().map(|o| o.status.success()).unwrap_or(false)
        })
    }

    fn set_wallpaper(&self, target: &str) -> Result<(), String> {
        let trimmed = target.trim();
        let normalized = if trimmed.starts_with("preset:") {
            let idx_str = trimmed.trim_start_matches("preset:");
            let idx: usize = idx_str.parse().unwrap_or(0);
            WALLPAPER_COLOR_PRESETS.get(idx).map(|(_, c)| *c).unwrap_or("#1e1e2e").to_string()
        } else if let Ok(idx) = trimmed.parse::<usize>() {
            WALLPAPER_COLOR_PRESETS.get(idx).map(|(_, c)| *c).unwrap_or("#1e1e2e").to_string()
        } else if trimmed.is_empty() {
            "#1e1e2e".to_string()
        } else {
            trimmed.to_string()
        };

        let clean_path = normalized.strip_prefix("file://").unwrap_or(&normalized);
        let path = std::path::Path::new(clean_path);
        let is_image = path.is_file();

        let swaybg_bin = resolve_swaybg();

        if is_image {
            if let Some(bin) = swaybg_bin {
                let mut cmd = Command::new(bin);
                cmd.args(["-i", clean_path, "-m", "fill"]);
                if let Ok(child) = cmd.spawn() {
                    let new_pid = child.id();
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    if let Ok(mut lock) = ACTIVE_WALLPAPER_PID.lock() {
                        if let Some(old_pid) = *lock {
                            let _ = Command::new("kill").arg(old_pid.to_string()).output();
                        }
                        *lock = Some(new_pid);
                    }
                } else {
                    return Err(format!("Failed to spawn swaybg for image {clean_path}"));
                }
            } else {
                let _ = Command::new("hyprctl").args(["hyprpaper", "preload", clean_path]).output();
                let _ = Command::new("hyprctl").args(["hyprpaper", "wallpaper", &format!(",{clean_path}")]).output();
            }
        } else {
            let hex = normalized.trim_start_matches('#');
            if let Some(bin) = swaybg_bin {
                let mut cmd = Command::new(bin);
                cmd.args(["-c", hex]);
                if let Ok(child) = cmd.spawn() {
                    let new_pid = child.id();
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    if let Ok(mut lock) = ACTIVE_WALLPAPER_PID.lock() {
                        if let Some(old_pid) = *lock {
                            let _ = Command::new("kill").arg(old_pid.to_string()).output();
                        }
                        *lock = Some(new_pid);
                    }
                } else {
                    return Err(format!("Failed to spawn swaybg for color {hex}"));
                }
            }
            let _ = Self::hyprctl_set_config(
                &format!("hl.config({{ misc = {{ background_color = \"0xff{hex}\" }} }})"),
                "misc:background_color",
                &format!("0xff{hex}"),
            );
        }

        if let Some(finick_dir) = get_finick_config_dir() {
            let _ = std::fs::create_dir_all(&finick_dir);
            let _ = std::fs::write(finick_dir.join("current_wallpaper"), &normalized);
        }

        Ok(())
    }

    fn get_current_wallpaper(&self) -> Option<String> {
        if let Some(finick_dir) = get_finick_config_dir() {
            let file = finick_dir.join("current_wallpaper");
            if let Ok(content) = std::fs::read_to_string(file) {
                let s = content.trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
        None
    }
}

static ACTIVE_WALLPAPER_PID: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

pub const WALLPAPER_COLOR_PRESETS: &[(&str, &str)] = &[
    ("Deep Slate", "#1e1e2e"),
    ("Midnight Navy", "#1f2a44"),
    ("Twilight Plum", "#3a2e47"),
    ("Forest Spruce", "#263836"),
    ("Warm Burgundy", "#402e2e"),
    ("Earth Umber", "#3d382c"),
    ("Steel Blue", "#282b35"),
    ("Carbon Black", "#11111b"),
];

fn resolve_swaybg() -> Option<std::path::PathBuf> {
    if let Ok(output) = Command::new("which").arg("swaybg").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let p = std::path::PathBuf::from(path_str);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    // Fallback: check /nix/store for swaybg
    if let Ok(entries) = std::fs::read_dir("/nix/store") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains("-swaybg-") {
                let bin = entry.path().join("bin/swaybg");
                if bin.exists() {
                    return Some(bin);
                }
            }
        }
    }
    None
}

pub fn kelvin_to_rgb_factors(temp_k: f32) -> (f32, f32, f32) {
    let t = (temp_k / 100.0).clamp(10.0, 65.0) as f64;
    let r = 1.0f32;
    let raw_g = (99.4708025861 * t.ln() - 161.1195681661) / 255.0;
    // Normalize relative to 6500K (~0.9965) so 6500K is neutral 1.0
    let g = (raw_g.clamp(0.0, 1.0) as f32 / 0.9965).clamp(0.0, 1.0);
    let raw_b = if t <= 19.0 { 0.0 } else { (138.5177312231 * (t - 10.0).ln() - 305.0447927307) / 255.0 };
    // Normalize relative to 6500K (~0.9805) so 6500K is neutral 1.0
    let b = (raw_b.clamp(0.0, 1.0) as f32 / 0.9805).clamp(0.0, 1.0);
    (r, g, b)
}

pub fn night_temp_from_slider(color_temp: f32) -> i32 { (6500.0 - color_temp * 35.0).clamp(1000.0, 6500.0) as i32 }

#[derive(Copy, Clone, PartialEq, Eq)]
enum NightShiftTool {
    Hyprsunset,
    Wlsunset,
    Gammastep,
    ScreenShader,
}

static NIGHT_SHIFT_TOOL: std::sync::OnceLock<NightShiftTool> = std::sync::OnceLock::new();

fn get_night_shift_tool() -> NightShiftTool {
    *NIGHT_SHIFT_TOOL.get_or_init(|| {
        if Command::new("which").arg("hyprsunset").output().map(|o| o.status.success()).unwrap_or(false) {
            NightShiftTool::Hyprsunset
        } else if Command::new("which").arg("wlsunset").output().map(|o| o.status.success()).unwrap_or(false) {
            NightShiftTool::Wlsunset
        } else if Command::new("which").arg("gammastep").output().map(|o| o.status.success()).unwrap_or(false) {
            NightShiftTool::Gammastep
        } else {
            NightShiftTool::ScreenShader
        }
    })
}

pub fn apply_night_shift(enabled: bool, color_temp: f32) {
    let tool = get_night_shift_tool();
    if enabled {
        let temp = night_temp_from_slider(color_temp);
        let t = temp.to_string();
        match tool {
            NightShiftTool::Hyprsunset => {
                let _ = Command::new("hyprctl").args(["hyprsunset", "temperature", &t]).output();
                let _ = Command::new("pkill").args(["-9", "hyprsunset"]).output();
                let _ = Command::new("hyprsunset").args(["-t", &t]).spawn();
            }
            NightShiftTool::Wlsunset => {
                let _ = Command::new("pkill").args(["-9", "wlsunset"]).output();
                let _ = Command::new("wlsunset").args(["-t", &t, "-T", "6500"]).spawn();
            }
            NightShiftTool::Gammastep => {
                let _ = Command::new("pkill").args(["-9", "gammastep"]).output();
                let _ = Command::new("gammastep").args(["-O", &t]).spawn();
            }
            NightShiftTool::ScreenShader => {
                let (r_factor, g_factor, b_factor) = kelvin_to_rgb_factors(temp as f32);
                let shader = format!(
                    "#version 300 es\nprecision mediump float;\nin vec2 v_texcoord;\nlayout(location = 0) out vec4 \
                     fragColor;\nuniform sampler2D tex;\nvoid main() {{\nvec4 c = texture(tex, v_texcoord);\nc.r = \
                     min(1.0, c.r * {r_factor:.3});\nc.g = min(1.0, c.g * {g_factor:.3});\nc.b = min(1.0, c.b * \
                     {b_factor:.3});\nfragColor = c;\n}}\n"
                );
                let shader_path = "/tmp/finick-nightshift.frag";
                let _ = std::fs::write(shader_path, &shader);
                let lua_set = format!("hl.config({{ decoration = {{ screen_shader = '{shader_path}' }} }})");
                let _ = HyprlandBackend::hyprctl_set_config(&lua_set, "decoration:screen_shader", shader_path);
            }
        }
    } else {
        match tool {
            NightShiftTool::Hyprsunset => {
                let _ = Command::new("hyprctl").args(["hyprsunset", "identity"]).output();
                let _ = Command::new("pkill").args(["hyprsunset"]).output();
            }
            NightShiftTool::Wlsunset => {
                let _ = Command::new("pkill").args(["wlsunset"]).output();
            }
            NightShiftTool::Gammastep => {
                let _ = Command::new("gammastep").args(["-x"]).output();
                let _ = Command::new("pkill").args(["gammastep"]).output();
            }
            NightShiftTool::ScreenShader => {
                let lua_unset = "hl.config({ decoration = { screen_shader = '' } })";
                let _ = HyprlandBackend::hyprctl_set_config(lua_unset, "decoration:screen_shader", "");
            }
        }
    }
}

fn get_finick_config_dir() -> Option<std::path::PathBuf> {
    if let Ok(custom) = std::env::var("FINICK_CONFIG_DIR") {
        return Some(std::path::PathBuf::from(custom));
    }
    if let Ok(home) = std::env::var("HOME") {
        let dot_finick = std::path::Path::new(&home).join(".config").join(".finick");
        if dot_finick.exists() {
            return Some(dot_finick);
        }
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|c| {
            let dot = c.join(".finick");
            if dot.exists() { dot } else { c.join("finick") }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wpctl_devices() {
        let sample = "Audio\n ├─ Devices:\n │  \n ├─ Sinks:\n │  *   47. Built-in Audio Analog Stereo          [vol: 0.65]\n │       48. HDMI Output                               [vol: 1.00]\n │  \n ├─ Sources:\n │  *   49. Built-in Audio Microphone\n ├─ Filters:\n";
        let (sinks, sources) = parse_wpctl_devices(sample);
        assert_eq!(sinks.len(), 2);
        assert_eq!(sinks[0].id, "47");
        assert_eq!(sinks[0].name, "Built-in Audio Analog Stereo");
        assert!(sinks[0].is_default);
        assert!(!sinks[1].is_default);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "Built-in Audio Microphone");
        assert!(sources[0].is_default);
        let (sinks, sources) = parse_wpctl_devices("Audio\n");
        assert!(sinks.is_empty());
        assert!(sources.is_empty());
    }

    #[test]
    fn test_parse_nmcli_wifi_line() {
        let parsed = parse_nmcli_wifi_line("yes:Homebase 5G:78:WPA2").unwrap();
        assert_eq!(parsed, (true, "Homebase 5G".to_string(), 78, "WPA2".to_string()));
        let parsed = parse_nmcli_wifi_line("no:Cafe\\:Free:54:").unwrap();
        assert_eq!(parsed.1, "Cafe:Free");
        assert_eq!(parsed.3, "Open");
        assert!(parse_nmcli_wifi_line("no:--:0:WPA2").is_none());
        assert!(parse_nmcli_wifi_line("no:OnlyTwo:fields").is_none());
    }

    #[test]
    fn test_parse_ip_route_get() {
        let out = "1.1.1.1 via 192.168.1.1 dev wlp166s0 src 192.168.1.105 uid 1000\n";
        assert_eq!(parse_ip_route_get(out), Some(("wlp166s0".to_string(), "192.168.1.105".to_string())));
        assert_eq!(parse_ip_route_get("unreachable\n"), None);
    }

    #[test]
    fn test_compute_battery_health() {
        assert_eq!(compute_battery_health(4600, 5000), Some(92));
        assert_eq!(compute_battery_health(5000, 5000), Some(100));
        assert_eq!(compute_battery_health(6000, 5000), Some(100));
        assert_eq!(compute_battery_health(0, 5000), None);
        assert_eq!(compute_battery_health(4600, 0), None);
    }

    #[test]
    fn test_parse_upower_dump_history() {
        let sample = "Device: /org/freedesktop/UPower/devices/battery_BAT0\n  History (charge):\n    1760000000  82.000  discharging\n    1760000600  81.000  discharging\n    1760001200  80.000  discharging\n    1760001800  79.000  discharging\n    1760002400  78.000  discharging\n    1760003000  77.000  discharging\n    1760003600  76.000  discharging\n  History (rate):\n    1760000000  12.000  discharging\n";
        let parsed = parse_upower_dump_history(sample).unwrap();
        assert_eq!(parsed, [82.0, 81.0, 80.0, 79.0, 78.0, 77.0, 76.0]);
        assert!(parse_upower_dump_history("Device: x\n  percentage: 55%\n").is_none());
        assert!(parse_upower_dump_history("").is_none());
    }

    #[test]
    fn test_parse_nmcli_dev_line() {
        assert_eq!(
            parse_nmcli_dev_line("enp5s0:ethernet:connected"),
            Some(("enp5s0".to_string(), "ethernet".to_string(), "connected".to_string()))
        );
        assert_eq!(
            parse_nmcli_dev_line("wlp166s0:wifi:disconnected"),
            Some(("wlp166s0".to_string(), "wifi".to_string(), "disconnected".to_string()))
        );
        assert!(parse_nmcli_dev_line(":ethernet:connected").is_none());
        assert!(parse_nmcli_dev_line("only:two").is_none());
    }

    #[test]
    fn test_parse_display_width() {
        assert_eq!(parse_display_width("1920x1080"), 1920);
        assert_eq!(parse_display_width("2560x1440"), 2560);
        assert_eq!(parse_display_width(""), 1920);
        assert_eq!(parse_display_width("bad"), 1920);
        assert_eq!(parse_display_width(" 1920x1080 "), 1920);
        assert_eq!(parse_display_width("0x1080"), 1920);
        assert_eq!(parse_display_width("3440X1440"), 3440);
        assert_eq!(parse_display_width("1280"), 1280);
        assert_eq!(parse_display_width("  "), 1920);
        assert_eq!(parse_display_width("x1080"), 1920);
    }

    #[test]
    fn test_build_display_positions_two() {
        let ordered = vec![("DP-1".to_string(), "1920x1080".to_string()), ("HDMI-1".to_string(), "2560x1440".to_string())];
        let pos = build_display_positions(&ordered);
        assert_eq!(pos.len(), 2);
        assert_eq!(pos[0], ("DP-1".to_string(), "0x0".to_string()));
        assert_eq!(pos[1], ("HDMI-1".to_string(), "1920x0".to_string()));
        let alias = build_monitor_position_args(&ordered);
        assert_eq!(alias, pos);
    }

    #[test]
    fn test_build_display_positions_cumulative_three() {
        let ordered = vec![
            ("A".to_string(), "1920x1080".to_string()),
            ("B".to_string(), "1280x720".to_string()),
            ("C".to_string(), "2560x1440".to_string()),
        ];
        let pos = build_display_positions(&ordered);
        assert_eq!(pos[0].1, "0x0");
        assert_eq!(pos[1].1, "1920x0");
        assert_eq!(pos[2].1, "3200x0");
    }

    #[test]
    fn test_build_display_positions_empty_and_single() {
        let empty: Vec<(String, String)> = vec![];
        assert!(build_display_positions(&empty).is_empty());
        assert!(build_monitor_position_args(&empty).is_empty());
        let single = vec![("DP-1".to_string(), "1920x1080".to_string())];
        let pos = build_display_positions(&single);
        assert_eq!(pos.len(), 1);
        assert_eq!(pos[0], ("DP-1".to_string(), "0x0".to_string()));
    }

    #[test]
    fn test_build_display_positions_fallback() {
        let ordered = vec![
            ("DP-1".to_string(), "bad".to_string()),
            ("HDMI-1".to_string(), "".to_string()),
            ("DP-2".to_string(), "2560x1440".to_string()),
        ];
        let pos = build_display_positions(&ordered);
        assert_eq!(pos[0].1, "0x0");
        assert_eq!(pos[1].1, "1920x0");
        assert_eq!(pos[2].1, "3840x0");
    }

    #[test]
    fn test_build_positions_from_names() {
        let mut lookup = HashMap::new();
        lookup.insert("DP-1".to_string(), "1920x1080".to_string());
        lookup.insert("HDMI-1".to_string(), "2560x1440".to_string());
        let names = vec!["DP-1".to_string(), "HDMI-1".to_string()];
        let pos = build_positions_from_names(&names, &lookup);
        assert_eq!(pos[0], ("DP-1".to_string(), "0x0".to_string()));
        assert_eq!(pos[1], ("HDMI-1".to_string(), "1920x0".to_string()));
        let names_missing = vec!["UNKNOWN".to_string(), "DP-1".to_string()];
        let pos2 = build_positions_from_names(&names_missing, &lookup);
        assert_eq!(pos2[0].1, "0x0");
        assert_eq!(pos2[1].1, "1920x0");
        let empty: Vec<String> = vec![];
        assert!(build_positions_from_names(&empty, &lookup).is_empty());
    }

    #[test]
    fn test_set_display_order_empty_no_hyprctl() {
        struct StubBackend;
        impl SystemBackend for StubBackend {
            fn get_displays(&self) -> Vec<DisplayInfo> {
                panic!("get_displays should not be called for empty order");
            }

            fn get_input_devices(&self) -> InputDevices { InputDevices { mice: vec![], keyboards: vec![] } }

            fn get_audio_info(&self) -> AudioInfo {
                AudioInfo {
                    volume: 0.0,
                    is_muted: false,
                    default_sink_name: String::new(),
                    default_source_name: String::new(),
                }
            }

            fn get_audio_sinks(&self) -> Vec<AudioDevice> { vec![] }

            fn get_audio_sources(&self) -> Vec<AudioDevice> { vec![] }

            fn set_default_audio_sink(&self, _sink_id: &str) -> bool { false }

            fn set_volume(&self, _percent: i32) {}

            fn supports_brightness(&self) -> bool { false }

            fn set_brightness(&self, _percent: u32) {}

            fn toggle_mute(&self) {}

            fn get_power_info(&self) -> PowerInfo {
                PowerInfo { capacity: String::new(), status: String::new(), health_percent: None, cycle_count: None }
            }

            fn get_general_info(&self) -> GeneralInfo { GeneralInfo { locale: String::new(), timezone: String::new() } }

            fn get_host_info(&self) -> HostInfo {
                HostInfo {
                    hostname: String::new(),
                    os_name: String::new(),
                    kernel: String::new(),
                    uptime: String::new(),
                    memory: String::new(),
                }
            }

            fn get_current_user(&self) -> UserInfo { UserInfo { username: String::new(), uid: String::new() } }

            fn get_wifi_status(&self) -> bool { false }

            fn get_wifi_networks(&self) -> Vec<String> { vec![] }

            fn get_wifi_details(&self) -> (CurrentWifiInfo, Vec<WifiNetworkDetail>) { (CurrentWifiInfo::default(), vec![]) }

            fn get_wired_info(&self) -> Option<WiredInfo> { None }

            fn connect_wifi(&self, _ssid: &str) {}

            fn disconnect_wifi(&self) {}

            fn forget_wifi(&self, _ssid: &str) {}

            fn set_wifi_status(&self, _enabled: bool) {}

            fn get_bluetooth_status(&self) -> bool { false }

            fn set_bluetooth_status(&self, _enabled: bool) {}

            fn get_storage_info(&self) -> StorageInfo {
                StorageInfo {
                    total: String::new(),
                    used: String::new(),
                    available: String::new(),
                    use_percent: 0.0,
                    disks: vec![],
                }
            }

            fn get_paired_bluetooth_devices(&self) -> Vec<BluetoothDevice> { vec![] }

            fn connect_bluetooth_device(&self, _mac: &str) {}

            fn disconnect_bluetooth_device(&self, _mac: &str) {}

            fn remove_bluetooth_device(&self, _mac: &str) {}

            fn set_display_order(&self, ordered_names: &[String]) -> Result<(), String> {
                if ordered_names.is_empty() {
                    return Ok(());
                }
                Err("stub".to_string())
            }

            fn set_display_config(&self, _name: &str, _mode: Option<&str>, _transform: Option<u8>) -> Result<(), String> {
                Ok(())
            }

            fn log_out(&self) -> bool { false }

            fn reboot(&self) -> bool { false }

            fn power_off(&self) -> bool { false }

            fn set_wallpaper(&self, _target: &str) -> Result<(), String> { Ok(()) }

            fn get_current_wallpaper(&self) -> Option<String> { None }
        }
        let backend = StubBackend;
        assert!(backend.set_display_order(&[]).is_ok());
    }

    #[test]
    fn test_parse_hyprctl_monitors() {
        let json = r#"[{
    "id": 0,
    "name": "DP-2",
    "description": "Dell Inc. DELL P2422HE",
    "width": 1920,
    "height": 1080,
    "refreshRate": 60.00000,
    "x": 0,
    "y": 0,
    "activeWorkspace": {
        "id": 1,
        "name": "1"
    },
    "scale": 1.00,
    "transform": 1,
    "availableModes": ["1920x1080@60.00Hz", "1600x900@60.00Hz"]
},{
    "id": 1,
    "name": "DP-4",
    "description": "Dell Inc. DELL P2422HE",
    "width": 2560,
    "height": 1440,
    "refreshRate": 144.00000,
    "x": 1920,
    "y": 0,
    "activeWorkspace": {
        "id": 2,
        "name": "2"
    },
    "scale": 1.25,
    "transform": 0
}]"#;
        let displays = parse_hyprctl_monitors(json);
        assert_eq!(displays.len(), 2);
        assert_eq!(displays[0].name, "DP-2");
        assert_eq!(displays[0].resolution, "1920x1080");
        assert_eq!(displays[0].refresh_rate, "60.00000");
        assert_eq!(displays[0].scale, "1.00");
        assert_eq!(displays[0].transform, 1);
        assert_eq!(displays[0].available_modes.len(), 2);

        assert_eq!(displays[1].name, "DP-4");
        assert_eq!(displays[1].resolution, "2560x1440");
        assert_eq!(displays[1].refresh_rate, "144.00000");
        assert_eq!(displays[1].scale, "1.25");
        assert_eq!(displays[1].transform, 0);
    }

    #[test]
    fn test_wallpaper_presets() {
        assert_eq!(WALLPAPER_COLOR_PRESETS.len(), 8);
        assert_eq!(WALLPAPER_COLOR_PRESETS[0].1, "#1e1e2e");
        assert_eq!(WALLPAPER_COLOR_PRESETS[1].1, "#1f2a44");
    }

    #[test]
    fn test_persistent_display_config_serde() {
        let configs = vec![
            PersistentDisplayConfig {
                name: "eDP-1".to_string(),
                mode: "1920x1080@60.0".to_string(),
                position: "0x0".to_string(),
                scale: "1.0".to_string(),
                transform: 0,
            },
            PersistentDisplayConfig {
                name: "HDMI-A-1".to_string(),
                mode: "2560x1440@144.0".to_string(),
                position: "1920x0".to_string(),
                scale: "1.25".to_string(),
                transform: 1,
            },
        ];

        let serialized = serde_json::to_string(&configs).expect("Serialization failed");
        let deserialized: Vec<PersistentDisplayConfig> = serde_json::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(configs, deserialized);
    }

    #[test]
    fn test_apply_night_shift_shader_format() {
        apply_night_shift(true, 50.0);
        let content = std::fs::read_to_string("/tmp/finick-nightshift.frag").expect("shader file exists");
        assert!(content.starts_with("#version 300 es"), "shader must start with #version 300 es");
        assert!(content.contains("in vec2 v_texcoord;"));
        assert!(content.contains("layout(location = 0) out vec4 fragColor;"));
        assert!(content.contains("texture(tex, v_texcoord)"));
        apply_night_shift(false, 50.0);
    }

    #[test]
    fn test_parse_pactl_sink_inputs() {
        let sample = "Sink Input #42\n\tDriver: protocol-native.c\n\tMute: no\n\tVolume: front-left: 32768 /  50% / -18.06 dB, front-right: 32768 /  50%\n\tapplication.name = \"Firefox\"\nSink Input #43\n\tMute: yes\n\tVolume: mono: 65536 / 100%\n\tapplication.name = \"mpv\"\n";
        let streams = parse_pactl_sink_inputs(sample);
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].id, 42);
        assert_eq!(streams[0].app_name, "Firefox");
        assert_eq!(streams[0].volume, 50);
        assert!(!streams[0].muted);
        assert!(streams[1].muted);
        assert!(parse_pactl_sink_inputs("").is_empty());
    }

    #[test]
    fn test_parse_display_mode_refresh() {
        assert_eq!(
            parse_display_mode_refresh("1920x1080@60.00Hz"),
            Some(("1920x1080".to_string(), 60.0))
        );
        assert_eq!(parse_display_mode_refresh("preferred"), None);
        assert_eq!(parse_display_mode_refresh(""), None);
    }

    #[test]
    fn test_night_schedule_active() {
        let sched = NightSchedule { mode: 2, from: "22:00".to_string(), to: "06:00".to_string() };
        assert!(night_schedule_active(&sched, "23:00"));
        assert!(night_schedule_active(&sched, "05:00"));
        assert!(!night_schedule_active(&sched, "12:00"));
        let off = NightSchedule { mode: 0, from: "22:00".to_string(), to: "06:00".to_string() };
        assert!(!night_schedule_active(&off, "23:00"));
        let sunset = NightSchedule { mode: 1, from: "22:00".to_string(), to: "06:00".to_string() };
        assert!(night_schedule_active(&sunset, "12:00"));
    }

    #[test]
    fn test_parse_gsettings_u32() {
        assert_eq!(parse_gsettings_u32("300"), Some(300));
        assert_eq!(parse_gsettings_u32("uint32 300"), None);
        assert_eq!(parse_gsettings_u32(""), None);
    }

    #[test]
    fn test_parse_rfkill_list() {
        let sample = "0: phy0: Wireless LAN\n\tSoft blocked: no\n\tHard blocked: no\n1: hci0: Bluetooth\n\tSoft blocked: yes\n\tHard blocked: no\n";
        let devs = parse_rfkill_list(sample);
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0].kind, "wlan");
        assert!(!devs[0].soft_blocked);
        assert_eq!(devs[1].kind, "bluetooth");
        assert!(devs[1].soft_blocked);
        assert!(parse_rfkill_list("").is_empty());
    }

    #[test]
    fn test_airplane_from_states() {
        assert!(airplane_from_states(false, true));
        assert!(!airplane_from_states(true, true));
        assert!(!airplane_from_states(false, false));
        assert!(!airplane_from_states(true, false));
    }

    #[test]
    fn test_kbd_pct_roundtrip() {
        assert_eq!(kbd_pct_from_values(2, 3), Some(66));
        assert_eq!(kbd_pct_from_values(0, 0), None);
        assert_eq!(kbd_value_from_pct(50, 3), 1);
        assert_eq!(kbd_value_from_pct(100, 3), 3);
        assert_eq!(kbd_value_from_pct(200, 3), 3);
    }

    #[test]
    fn test_parse_hypr_clients_classes() {
        let sample = r#"[{"class":"kitty"},{"class":"firefox"},{"class":""}]"#;
        let classes = parse_hypr_clients_classes(sample);
        assert_eq!(classes, vec!["kitty".to_string(), "firefox".to_string()]);
        assert!(parse_hypr_clients_classes("not json").is_empty());
        assert!(parse_hypr_clients_classes("[]").is_empty());
    }
}
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub windows: i32,
}

pub fn get_active_workspaces() -> std::collections::HashMap<String, i32> {
    if let Ok(output) = std::process::Command::new("hyprctl").args(["monitors", "-j"]).output() {
        let out = String::from_utf8_lossy(&output.stdout);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
            if let Some(arr) = v.as_array() {
                let mut map = std::collections::HashMap::new();
                for mon in arr {
                    if let (Some(name), Some(ws)) = (mon["name"].as_str(), mon["activeWorkspace"]["id"].as_i64()) {
                        map.insert(name.to_string(), ws as i32);
                    }
                }
                return map;
            }
        }
    }
    std::collections::HashMap::new()
}

pub fn get_workspaces() -> Vec<WorkspaceInfo> {
    if let Ok(output) = std::process::Command::new("hyprctl").args(["workspaces", "-j"]).output() {
        let out = String::from_utf8_lossy(&output.stdout);
        if let Ok(mut workspaces) = serde_json::from_str::<Vec<WorkspaceInfo>>(&out) {
            workspaces.sort_by_key(|w| w.id);
            return workspaces;
        }
    }
    Vec::new()
}
