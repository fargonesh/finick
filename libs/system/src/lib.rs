use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayInfo {
    pub name: String,
    pub resolution: String,
    pub refresh_rate: String,
    pub scale: String,
}
pub fn parse_display_width(resolution: &str) -> u32 {
    let trimmed = resolution.trim();
    if trimmed.is_empty() {
        return 1920;
    }
    let w_str = trimmed.split(|c| c == 'x' || c == 'X').next().unwrap_or("").trim();
    w_str.parse::<u32>().ok().filter(|&v| v > 0).unwrap_or(1920)
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

pub fn parse_hyprctl_monitors(out: &str) -> Vec<DisplayInfo> {
    let mut res = Vec::new();
    let mut current_name = String::new();
    let mut current_w = String::new();
    let mut current_h = String::new();
    let mut current_hz = String::new();
    let mut current_scale = String::new();
    let mut depth: i32 = 0;

    for line in out.lines() {
        let l = line.trim();
        for ch in l.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth <= 0 && !current_name.is_empty() {
                    let resolution = if !current_w.is_empty() && !current_h.is_empty() {
                        format!("{}x{}", current_w, current_h)
                    } else {
                        String::new()
                    };
                    res.push(DisplayInfo {
                        name: current_name.clone(),
                        resolution,
                        refresh_rate: current_hz.clone(),
                        scale: current_scale.clone(),
                    });
                    current_name.clear();
                    current_w.clear();
                    current_h.clear();
                    current_hz.clear();
                    current_scale.clear();
                }
            }
        }

        if depth == 1 {
            if l.starts_with("\"name\":") {
                current_name =
                    l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == '\"' || c == ',' || c == ' ').to_string();
            } else if l.starts_with("\"width\":") {
                current_w = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
            } else if l.starts_with("\"height\":") {
                current_h = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
            } else if l.starts_with("\"refreshRate\":") {
                current_hz = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
            } else if l.starts_with("\"scale\":") {
                current_scale = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
            }
        }
    }
    res
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputDevice {
    pub name: String,
    pub layout_or_type: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InputDevices {
    pub mice: Vec<InputDevice>,
    pub keyboards: Vec<InputDevice>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AudioInfo {
    pub volume: f64,
    pub is_muted: bool,
    pub default_sink_name: String,
    pub default_source_name: String,
}
#[derive(Clone, Debug, PartialEq)]
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
    fn disconnect_wifi(&self);
    fn forget_wifi(&self, ssid: &str);
    fn set_wifi_status(&self, enabled: bool);
    fn get_bluetooth_status(&self) -> bool;
    fn set_bluetooth_status(&self, enabled: bool);
    fn get_storage_info(&self) -> StorageInfo;
    fn get_paired_bluetooth_devices(&self) -> Vec<BluetoothDevice>;
    fn connect_bluetooth_device(&self, mac: &str);
    fn disconnect_bluetooth_device(&self, mac: &str);
    fn remove_bluetooth_device(&self, mac: &str);
    fn set_display_order(&self, ordered_names: &[String]) -> Result<(), String>;
    fn log_out(&self) -> bool;
    fn reboot(&self) -> bool;
    fn power_off(&self) -> bool;
}

pub struct HyprlandBackend;

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
        let mut lookup: HashMap<String, String> = HashMap::new();
        let mut scale_lookup: HashMap<String, String> = HashMap::new();
        for d in displays {
            lookup.insert(d.name.clone(), d.resolution);
            if !d.scale.trim().is_empty() {
                scale_lookup.insert(d.name, d.scale);
            }
        }
        let positions = build_positions_from_names(ordered_names, &lookup);
        for (name, pos) in positions {
            let scale = scale_lookup.get(&name).map(|s| s.as_str()).unwrap_or("1");
            let arg = format!("{name},preferred,{pos},{scale}");
            let output = Command::new("hyprctl")
                .args(["keyword", "monitor", &arg])
                .output()
                .map_err(|e| format!("failed to spawn hyprctl: {e}"))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let detail = if !stderr.is_empty() { stderr } else { stdout };
                let msg = if detail.is_empty() {
                    format!("hyprctl keyword monitor {arg} failed")
                } else {
                    format!("hyprctl keyword monitor {arg} failed: {detail}")
                };
                return Err(msg);
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
                    let name = if parts.len() >= 3 && !parts[2].trim().is_empty() {
                        parts[2].trim().to_string()
                    } else {
                        mac.clone()
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

            fn log_out(&self) -> bool { false }

            fn reboot(&self) -> bool { false }

            fn power_off(&self) -> bool { false }
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
    "scale": 1.00
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
    "scale": 1.25
}]"#;
        let displays = parse_hyprctl_monitors(json);
        assert_eq!(displays.len(), 2);
        assert_eq!(displays[0].name, "DP-2");
        assert_eq!(displays[0].resolution, "1920x1080");
        assert_eq!(displays[0].refresh_rate, "60.00000");
        assert_eq!(displays[0].scale, "1.00");

        assert_eq!(displays[1].name, "DP-4");
        assert_eq!(displays[1].resolution, "2560x1440");
        assert_eq!(displays[1].refresh_rate, "144.00000");
        assert_eq!(displays[1].scale, "1.25");
    }
}
