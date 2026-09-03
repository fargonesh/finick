use std::fs;

fn main() {
    let mut content = fs::read_to_string("libs/system/src/lib.rs").unwrap();
    let prefix = r#"use std::process::Command;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)] pub struct DisplayInfo { pub name: String, pub resolution: String, pub refresh_rate: String, pub scale: String }
#[derive(Clone, Debug, PartialEq)] pub struct InputDevice { pub name: String, pub layout_or_type: String }
#[derive(Clone, Debug, PartialEq)] pub struct InputDevices { pub mice: Vec<InputDevice>, pub keyboards: Vec<InputDevice> }
#[derive(Clone, Debug, PartialEq)] pub struct AudioInfo { pub volume: f64, pub is_muted: bool, pub default_sink_name: String }
#[derive(Clone, Debug, PartialEq)] pub struct PowerInfo { pub capacity: String, pub status: String }
#[derive(Clone, Debug, PartialEq)] pub struct GeneralInfo { pub locale: String, pub timezone: String }
#[derive(Clone, Debug, PartialEq)] pub struct HostInfo { pub hostname: String, pub os_name: String, pub kernel: String, pub uptime: String, pub memory: String }
#[derive(Clone, Debug, PartialEq)] pub struct DiskInfo { pub filesystem: String, pub mount_point: String, pub total: String, pub used: String, pub available: String, pub use_percent: f64 }
#[derive(Clone, Debug, PartialEq)] pub struct StorageInfo { pub total: String, pub used: String, pub available: String, pub use_percent: f64, pub disks: Vec<DiskInfo> }
#[derive(Clone, Debug, PartialEq)] pub struct BluetoothDevice { pub mac: String, pub name: String, pub connected: bool }

pub trait SystemBackend {"#;

    let idx = content.find("pub trait SystemBackend {").unwrap();
    let rest = &content[idx + "pub trait SystemBackend {".len()..];
    
    fs::write("libs/system/src/lib.rs", format!("{}\n{}", prefix, rest)).unwrap();
}
