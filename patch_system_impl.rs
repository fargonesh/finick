use std::fs;

fn main() {
    let mut content = fs::read_to_string("libs/system/src/lib.rs").unwrap();
    
    let trait_impl = r#"
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
                    let fs = parts[0];
                    let size = parts[1];
                    let used = parts[2];
                    let avail = parts[3];
                    let pct_str = parts[4].trim_end_matches('%');
                    let pct = pct_str.parse::<f64>().unwrap_or(0.0);
                    let mount = parts[5];

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
        let mut connected_macs = std::collections::HashSet::new();

        if let Ok(output) = std::process::Command::new("bluetoothctl").args(&["devices", "Connected"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "Device" {
                    connected_macs.insert(parts[1].to_uppercase());
                }
            }
        }

        let paired_output = std::process::Command::new("bluetoothctl").args(&["devices", "Paired"]).output().ok();
        let out_str = match paired_output {
            Some(ref output) if !output.stdout.is_empty() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => std::process::Command::new("bluetoothctl").args(&["devices"]).output().ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default(),
        };

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
        let _ = std::process::Command::new("bluetoothctl").args(&["connect", mac]).output();
    }

    fn disconnect_bluetooth_device(&self, mac: &str) {
        let _ = std::process::Command::new("bluetoothctl").args(&["disconnect", mac]).output();
    }

    fn remove_bluetooth_device(&self, mac: &str) {
        let _ = std::process::Command::new("bluetoothctl").args(&["remove", mac]).output();
    }
"#;
    content = content.replace("}\n", &format!("}}\n{}", trait_impl));
    // Since replace("}\n", ...) will replace ALL instances of }\n, we should just append it before the final '}'
    
    // Instead of replace, let's inject it into `impl SystemBackend for HyprlandBackend {`
    let start_idx = content.rfind("}").unwrap();
    content.insert_str(start_idx, trait_impl);

    fs::write("libs/system/src/lib.rs", content).unwrap();
}
