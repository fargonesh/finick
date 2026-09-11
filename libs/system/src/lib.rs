use {
    serde::{Deserialize, Serialize},
    std::{
        collections::{HashMap, HashSet},
        process::Command,
    },
};

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

pub fn parse_nmcli_wifi_line(line: &str) -> Option<(bool, String, u8, String)> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                current.push(next);
            }
            continue;
        }
        if c == ':' {
            fields.push(std::mem::take(&mut current));
            continue;
        }
        current.push(c);
    }
    fields.push(current);
    if fields.len() < 4 {
        return None;
    }
    let active = fields[0].trim() == "yes";
    let ssid = fields[1].trim().to_string();
    if ssid.is_empty() || ssid == "--" {
        return None;
    }
    let signal = fields[2].trim().parse::<u8>().unwrap_or(0);
    let security = fields[3].trim().to_string();
    let security = if security.is_empty() { "Open".to_string() } else { security };
    Some((active, ssid, signal, security))
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
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                current.push(next);
            }
            continue;
        }
        if c == ':' {
            fields.push(std::mem::take(&mut current));
            continue;
        }
        current.push(c);
    }
    fields.push(current);
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
    fn set_wallpaper(&self, target: &str) -> Result<(), String>;
    fn get_current_wallpaper(&self) -> Option<String>;
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
    pub fn hyprctl_set_config(lua_code: &str, legacy_keyword: &str, legacy_val: &str) -> Result<(), String> {
        if let Ok(output) = Command::new("hyprctl").args(["eval", lua_code]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if output.status.success() && stdout.starts_with("ok") {
                return Ok(());
            }
        }
        let output = Command::new("hyprctl")
            .args(["keyword", legacy_keyword, legacy_val])
            .output()
            .map_err(|e| format!("failed to spawn hyprctl: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stdout.contains("can't work") || stdout.contains("error") || !output.status.success() {
            let detail = if !stderr.is_empty() { stderr } else { stdout };
            return Err(format!("failed to configure hyprland: {detail}"));
        }
        Ok(())
    }

    pub fn apply_monitor_config(name: &str, mode: &str, position: &str, scale: &str, transform: u8) -> Result<(), String> {
        let scale_val: f64 = scale.parse().unwrap_or(1.0);
        let eval_lua = format!(
            r#"hl.monitor({{ output = "{name}", mode = "{mode}", position = "{position}", scale = {scale_val}, transform = {transform} }})"#
        );
        if let Ok(output) = Command::new("hyprctl").args(["eval", &eval_lua]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if output.status.success()
                && (stdout.starts_with("ok") || (!stdout.contains("error") && !stdout.contains("fail") && stderr.is_empty()))
            {
                return Ok(());
            }
        }

        let arg = format!("{name},{mode},{position},{scale_val},transform,{transform}");
        let output = Command::new("hyprctl")
            .args(["keyword", "monitor", &arg])
            .output()
            .map_err(|e| format!("failed to spawn hyprctl: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stdout.contains("can't work") || stdout.contains("error") || !output.status.success() {
            let detail = if !stderr.is_empty() { stderr } else { stdout };
            return Err(format!("failed to configure monitor {name}: {detail}"));
        }
        Ok(())
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
        let v_str = format!("{}%", percent);
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
}
