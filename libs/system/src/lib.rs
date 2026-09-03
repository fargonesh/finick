use std::collections::HashSet;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayInfo {
    pub name: String,
    pub resolution: String,
    pub refresh_rate: String,
    pub scale: String,
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
}
#[derive(Clone, Debug, PartialEq)]
pub struct PowerInfo {
    pub capacity: String,
    pub status: String,
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
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

pub trait SystemBackend {
    fn get_displays(&self) -> Vec<DisplayInfo>;
    fn get_input_devices(&self) -> InputDevices;
    fn get_audio_info(&self) -> AudioInfo;
    fn set_volume(&self, percent: i32);
    fn toggle_mute(&self);
    fn get_power_info(&self) -> PowerInfo;
    fn get_general_info(&self) -> GeneralInfo;
    fn get_host_info(&self) -> HostInfo;
    fn get_wifi_status(&self) -> bool;
    fn set_wifi_status(&self, enabled: bool);
    fn get_bluetooth_status(&self) -> bool;
    fn set_bluetooth_status(&self, enabled: bool);
    fn get_storage_info(&self) -> StorageInfo;
    fn get_paired_bluetooth_devices(&self) -> Vec<BluetoothDevice>;
    fn connect_bluetooth_device(&self, mac: &str);
    fn disconnect_bluetooth_device(&self, mac: &str);
    fn remove_bluetooth_device(&self, mac: &str);
}

pub struct HyprlandBackend;

impl SystemBackend for HyprlandBackend {
    fn get_displays(&self) -> Vec<DisplayInfo> {
        let mut res = Vec::new();
        if let Ok(output) = Command::new("hyprctl").args(&["monitors", "-j"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);

            let mut current_name = String::new();
            let mut current_w = String::new();
            let mut current_h = String::new();
            let mut current_hz = String::new();
            let mut current_scale = String::new();
            for line in out.lines() {
                let l = line.trim();
                if l.starts_with("\"name\":") {
                    current_name = l
                        .split(':')
                        .nth(1)
                        .unwrap_or_default()
                        .trim_matches(|c| c == '\"' || c == ',' || c == ' ')
                        .to_string();
                } else if l.starts_with("\"width\":") {
                    current_w = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
                } else if l.starts_with("\"height\":") {
                    current_h = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
                } else if l.starts_with("\"refreshRate\":") {
                    current_hz = l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
                } else if l.starts_with("\"scale\":") {
                    current_scale =
                        l.split(':').nth(1).unwrap_or_default().trim_matches(|c| c == ',' || c == ' ').to_string();
                }
                if l == "}," || l == "}" {
                    if !current_name.is_empty() {
                        res.push(DisplayInfo {
                            name: current_name.clone(),
                            resolution: format!("{}x{}", current_w, current_h),
                            refresh_rate: current_hz.clone(),
                            scale: current_scale.clone(),
                        });
                        current_name.clear();
                    }
                }
            }
        }
        res
    }
    fn get_input_devices(&self) -> InputDevices {
        let mut mice = Vec::new();
        let mut keyboards = Vec::new();
        if let Ok(output) = Command::new("hyprctl").args(&["devices", "-j"]).output() {
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
                    if l == "}," || l == "}" {
                        if !current_name.is_empty() {
                            mice.push(InputDevice {
                                name: current_name.clone(),
                                layout_or_type: "Pointer Device".to_string(),
                            });
                            current_name.clear();
                        }
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
                    if l == "}," || l == "}" {
                        if !current_name.is_empty() {
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
        }
        InputDevices { mice, keyboards }
    }
    fn get_audio_info(&self) -> AudioInfo {
        let mut current_vol = 50.0;
        let mut current_mute = false;
        let mut sink_name = "Unknown Device".to_string();
        if let Ok(output) = Command::new("wpctl").args(&["get-volume", "@DEFAULT_AUDIO_SINK@"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            if out.contains("[MUTED]") {
                current_mute = true;
            }
            if let Some(vol_str) = out.split_whitespace().nth(1) {
                if let Ok(v) = vol_str.parse::<f64>() {
                    current_vol = v * 100.0;
                }
            }
        }
        if let Ok(output) = Command::new("wpctl").args(&["status"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            let mut in_sinks = false;
            for line in out.lines() {
                if line.contains("Sinks:") {
                    in_sinks = true;
                    continue;
                }
                if in_sinks && line.contains("Sources:") {
                    break;
                }
                if in_sinks && line.trim_start().starts_with('*') {
                    if let Some(name_part) = line.split('.').nth(1) {
                        sink_name = name_part.split('[').next().unwrap_or_default().trim().to_string();
                        break;
                    }
                }
            }
        }
        AudioInfo { volume: current_vol, is_muted: current_mute, default_sink_name: sink_name }
    }
    fn set_volume(&self, percent: i32) {
        let v_str = format!("{}%", percent);
        let _ = Command::new("wpctl").args(&["set-volume", "@DEFAULT_AUDIO_SINK@", &v_str]).output();
    }
    fn toggle_mute(&self) {
        let _ = Command::new("wpctl").args(&["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).output();
    }
    fn get_power_info(&self) -> PowerInfo {
        let mut capacity = "Unknown".to_string();
        let mut status = "Unknown".to_string();
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
                    break;
                }
            }
        }
        PowerInfo { capacity, status }
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
        let hostname =
            std::fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "finick-os".to_string()).trim().to_string();
        let os_name = std::fs::read_to_string("/etc/os-release")
            .unwrap_or_default()
            .lines()
            .find(|l| l.starts_with("PRETTY_NAME="))
            .unwrap_or("PRETTY_NAME=\"Finick OS\"")
            .replace("PRETTY_NAME=", "")
            .replace("\"", "");
        let kernel = Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "Linux".to_string());
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
    fn get_wifi_status(&self) -> bool {
        Command::new("nmcli")
            .args(&["radio", "wifi"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
            .unwrap_or(false)
    }
    fn set_wifi_status(&self, enabled: bool) {
        let _ = Command::new("nmcli").args(&["radio", "wifi", if enabled { "on" } else { "off" }]).output();
    }
    fn get_bluetooth_status(&self) -> bool {
        Command::new("rfkill")
            .args(&["list", "bluetooth"])
            .output()
            .ok()
            .map(|o| !String::from_utf8_lossy(&o.stdout).contains("Soft blocked: yes"))
            .unwrap_or(false)
    }
    fn set_bluetooth_status(&self, enabled: bool) {
        let _ = Command::new("rfkill").args(&[if enabled { "unblock" } else { "block" }, "bluetooth"]).output();
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
        if let Ok(output) = std::process::Command::new("df").args(&["-hP"]).output() {
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
        if let Ok(output) = Command::new("bluetoothctl").args(&["devices", "Connected"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "Device" {
                    connected_macs.insert(parts[1].to_uppercase());
                }
            }
        }
        let out_str = Command::new("bluetoothctl")
            .args(&["devices"])
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
    fn connect_bluetooth_device(&self, mac: &str) {
        let _ = Command::new("bluetoothctl").args(&["connect", mac]).output();
    }
    fn disconnect_bluetooth_device(&self, mac: &str) {
        let _ = Command::new("bluetoothctl").args(&["disconnect", mac]).output();
    }
    fn remove_bluetooth_device(&self, mac: &str) {
        let _ = Command::new("bluetoothctl").args(&["remove", mac]).output();
    }
}
