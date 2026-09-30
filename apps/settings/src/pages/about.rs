use {freya::prelude::*, std::process::Command, ui::*};

#[derive(Clone, Debug, PartialEq)]
pub struct AboutInfo {
    pub os_version: String,
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub architecture: String,
    pub memory: String,
}

impl Default for AboutInfo {
    fn default() -> Self {
        Self {
            os_version: "Finick OS (Linux)".to_string(),
            hostname: "finick-station".to_string(),
            kernel: "Linux 6.12.0".to_string(),
            uptime: "Unknown".to_string(),
            architecture: "x86_64".to_string(),
            memory: "Unknown".to_string(),
        }
    }
}

pub fn fetch_about_info() -> AboutInfo {
    // Hostname
    let hostname = Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !s.is_empty() { Some(s) } else { None }
            } else {
                None
            }
        })
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()))
        .unwrap_or_else(|| "finick-station".to_string());

    // OS Version
    let os_version = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            let pretty = content
                .lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').trim().to_string());
            let version = content
                .lines()
                .find(|l| l.starts_with("VERSION="))
                .map(|l| l.trim_start_matches("VERSION=").trim_matches('"').trim().to_string());

            match (pretty, version) {
                (Some(p), Some(v)) if !p.contains(&v) => Some(format!("{} ({})", p, v)),
                (Some(p), _) => Some(p),
                (None, Some(v)) => Some(format!("Finick OS {}", v)),
                _ => None,
            }
        })
        .or_else(|| {
            Command::new("uname").arg("-o").output().ok().and_then(|o| {
                if o.status.success() {
                    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if !s.is_empty() { Some(format!("Finick OS ({})", s)) } else { None }
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "Finick OS 1.0 (Rolling)".to_string());

    // Kernel
    let kernel = Command::new("uname")
        .args(["-s", "-r"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !s.is_empty() { Some(s) } else { None }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Linux".to_string());

    // Uptime
    let uptime = Command::new("uptime")
        .arg("-p")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let clean = s.strip_prefix("up ").unwrap_or(&s).trim().to_string();
                if !clean.is_empty() { Some(clean) } else { None }
            } else {
                None
            }
        })
        .or_else(|| {
            std::fs::read_to_string("/proc/uptime").ok().and_then(|up| {
                let secs_str = up.split_whitespace().next()?;
                let total_secs = secs_str.parse::<f64>().ok()? as u64;
                let days = total_secs / 86400;
                let hours = (total_secs % 86400) / 3600;
                let mins = (total_secs % 3600) / 60;
                if days > 0 {
                    Some(format!("{} days, {} hours, {} mins", days, hours, mins))
                } else if hours > 0 {
                    Some(format!("{} hours, {} mins", hours, mins))
                } else {
                    Some(format!("{} mins", mins))
                }
            })
        })
        .unwrap_or_else(|| "Unknown".to_string());

    // Architecture
    let architecture = Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !s.is_empty() { Some(s) } else { None }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "x86_64".to_string());

    // Memory
    let memory = Command::new("free")
        .arg("-h")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                let line = s.lines().nth(1)?;
                let total = line.split_whitespace().nth(1)?;
                Some(format!("{} RAM", total))
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    AboutInfo { os_version, hostname, kernel, uptime, architecture, memory }
}

#[derive(PartialEq)]
pub struct About;

impl Component for About {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let info = use_state(fetch_about_info);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut info_state = info;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let data = fetch_about_info();
                let _ = tx.send(data);
            });

            freya::prelude::spawn(async move {
                if let Some(data) = rx.recv().await {
                    info_state.set(data);
                }
            });
        }

        let current = info.read().clone();

        FadeSlideIn::new().child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(page_head(ABOUT, "About", "System and device specifications."))
                .child(tile().child(hero_identity("⚡", current.os_version.clone(), "Finick desktop environment")))
                .child(
                    tile()
                        .child(tile_head(Some(ABOUT), "System specifications", None::<String>))
                        .child(setting_row(
                            "Device name",
                            None::<String>,
                            false,
                            label().font_size(14.).color(t.text_dim).text(current.hostname.clone()),
                        ))
                        .child(setting_row(
                            "Operating system",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.os_version.clone()),
                        ))
                        .child(setting_row(
                            "Kernel version",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.kernel.clone()),
                        ))
                        .child(setting_row(
                            "System uptime",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.uptime.clone()),
                        ))
                        .child(setting_row(
                            "Architecture",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.architecture.clone()),
                        ))
                        .child(setting_row(
                            "Memory",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.memory.clone()),
                        ))
                        .child(rect().horizontal().margin((8., 0., 0., 0.)).spacing(12.).child(secondary_button(
                            "Refresh info",
                            {
                                let mut l = loaded;
                                move || l.set(false)
                            },
                        ))),
                ),
        )
    }
}
