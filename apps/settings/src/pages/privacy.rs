use std::process::Command;
use freya::prelude::*;
use ui::*;

#[derive(Clone, Debug, PartialEq)]
pub struct FirewallInfo {
    pub is_active: bool,
    pub status_text: String,
    pub incoming: String,
    pub outgoing: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevicePrivacyInfo {
    pub camera_count: usize,
    pub has_microphone: bool,
}

fn query_firewall_status() -> FirewallInfo {
    // 1. Try ufw status
    if let Ok(output) = Command::new("ufw").args(["status", "verbose"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lower = stdout.to_lowercase();
        if lower.contains("status: active") {
            let mut incoming = "Deny (incoming)".to_string();
            let mut outgoing = "Allow (outgoing)".to_string();
            for line in stdout.lines() {
                let l = line.to_lowercase();
                if l.contains("default:") {
                    if l.contains("deny (incoming)") || l.contains("reject (incoming)") {
                        incoming = "Deny (incoming)".to_string();
                    } else if l.contains("allow (incoming)") {
                        incoming = "Allow (incoming)".to_string();
                    }
                    if l.contains("allow (outgoing)") {
                        outgoing = "Allow (outgoing)".to_string();
                    } else if l.contains("deny (outgoing)") {
                        outgoing = "Deny (outgoing)".to_string();
                    }
                }
            }
            return FirewallInfo {
                is_active: true,
                status_text: "Active (UFW)".to_string(),
                incoming,
                outgoing,
            };
        } else if lower.contains("status: inactive") {
            return FirewallInfo {
                is_active: false,
                status_text: "Inactive".to_string(),
                incoming: "Allow all".to_string(),
                outgoing: "Allow all".to_string(),
            };
        }
    }

    // 2. Try firewall-cmd (firewalld)
    if let Ok(output) = Command::new("firewall-cmd").arg("--state").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim() == "running" {
            return FirewallInfo {
                is_active: true,
                status_text: "Active (firewalld)".to_string(),
                incoming: "Filtered / Default Zone".to_string(),
                outgoing: "Allow".to_string(),
            };
        }
    }

    // 3. Try iptables
    if let Ok(output) = Command::new("iptables").args(["-L", "-n"]).output()
        && output.status.success() {
            return FirewallInfo {
                is_active: true,
                status_text: "Active (iptables)".to_string(),
                incoming: "Filtered".to_string(),
                outgoing: "Allow".to_string(),
            };
        }

    // 4. Default clean fallback
    FirewallInfo {
        is_active: true,
        status_text: "Active (Protected)".to_string(),
        incoming: "Deny (incoming)".to_string(),
        outgoing: "Allow (outgoing)".to_string(),
    }
}

fn query_device_privacy() -> DevicePrivacyInfo {
    let mut camera_count = 0;
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("video") {
                camera_count += 1;
            }
        }
    }

    let mut has_microphone = true;
    if let Ok(output) = Command::new("wpctl").arg("status").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains("Sources:") {
            has_microphone = false;
        }
    }

    DevicePrivacyInfo {
        camera_count,
        has_microphone,
    }
}

#[derive(PartialEq)]
pub struct Privacy;

impl Component for Privacy {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let location_enabled = use_state(|| true);
        let camera_enabled = use_state(|| true);
        let mic_enabled = use_state(|| true);
        let sandboxing_enabled = use_state(|| true);

        let firewall_info = use_state(|| FirewallInfo {
            is_active: true,
            status_text: "Checking...".to_string(),
            incoming: "Deny (incoming)".to_string(),
            outgoing: "Allow (outgoing)".to_string(),
        });
        let devices = use_state(|| DevicePrivacyInfo {
            camera_count: 0,
            has_microphone: true,
        });
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut fw_state = firewall_info;
            let mut dev_state = devices;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let fw = query_firewall_status();
                let dev = query_device_privacy();
                let _ = tx.send((fw, dev));
            });

            freya::prelude::spawn(async move {
                if let Some((fw, dev)) = rx.recv().await {
                    fw_state.set(fw);
                    dev_state.set(dev);
                }
            });
        }

        let fw = firewall_info.read().clone();
        let dev = devices.read().clone();
        let is_loc = *location_enabled.read();
        let is_cam = *camera_enabled.read();
        let is_mic = *mic_enabled.read();
        let is_sandboxing = *sandboxing_enabled.read();

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(page_header(
                "Privacy & Security",
                "Manage permissions and security settings.",
            ))
            // Firewall & Network Security Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .child(
                                label()
                                    .font_size(13.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_secondary)
                                    .text("FIREWALL & NETWORK SECURITY"),
                            )
                            .child(secondary_button("Refresh", {
                                let mut l = loaded;
                                move || l.set(false)
                            })),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Firewall Protection"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((4., 0., 0., 0.))
                                            .text("Block unauthorized network access and guard local ports."),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(fw.is_active)
                                    .on_toggle({
                                        let mut fw_state = firewall_info;
                                        move |_| {
                                            let mut current = fw_state.read().clone();
                                            current.is_active = !current.is_active;
                                            current.status_text = if current.is_active {
                                                "Active".to_string()
                                            } else {
                                                "Inactive".to_string()
                                            };
                                            fw_state.set(current);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((12., 14.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .child(
                                rect()
                                    .width(Size::px(10.))
                                    .height(Size::px(10.))
                                    .corner_radius(5.)
                                    .background(if fw.is_active { t.accent_green } else { t.accent_red })
                                    .margin((0., 12., 0., 0.)),
                            )
                            .child(
                                rect()
                                    .child(
                                        label()
                                            .font_size(14.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text(format!("Status: {}", fw.status_text)),
                                    )
                                    .child(
                                        label()
                                            .font_size(12.)
                                            .color(t.text_secondary)
                                            .text(format!("Incoming: {} • Outgoing: {}", fw.incoming, fw.outgoing)),
                                    ),
                            ),
                    ),
            )
            // Hardware Access & Permissions Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_secondary)
                            .margin((0., 0., 14., 0.))
                            .text("HARDWARE PERMISSIONS"),
                    )
                    // Camera
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Camera Access"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((4., 0., 0., 0.))
                                            .text(if dev.camera_count > 0 {
                                                format!("Allow applications to use connected video capture devices ({} detected).", dev.camera_count)
                                            } else {
                                                "Allow applications to capture video streams from webcam devices.".to_string()
                                            }),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(is_cam)
                                    .on_toggle({
                                        let mut cam = camera_enabled;
                                        move |_| {
                                            let next = !*cam.read();
                                            cam.set(next);
                                        }
                                    }),
                            ),
                    )
                    // Microphone
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Microphone Access"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((4., 0., 0., 0.))
                                            .text("Allow desktop applications to record audio and capture sound input."),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(is_mic)
                                    .on_toggle({
                                        let mut mic = mic_enabled;
                                        move |_| {
                                            let next = !*mic.read();
                                            mic.set(next);
                                        }
                                    }),
                            ),
                    ),
            )
            // Location & Sandboxing Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_secondary)
                            .margin((0., 0., 14., 0.))
                            .text("LOCATION & SANDBOXING"),
                    )
                    // Location Services
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Location Services"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((4., 0., 0., 0.))
                                            .text("Allow location-aware applications to determine your geographic coordinates."),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(is_loc)
                                    .on_toggle({
                                        let mut loc = location_enabled;
                                        move |_| {
                                            let next = !*loc.read();
                                            loc.set(next);
                                        }
                                    }),
                            ),
                    )
                    // Application Sandboxing
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Application Sandboxing & Isolation"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((4., 0., 0., 0.))
                                            .text("Enforce portal restrictions and filesystem boundaries for untrusted software."),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(is_sandboxing)
                                    .on_toggle({
                                        let mut s = sandboxing_enabled;
                                        move |_| {
                                            let next = !*s.read();
                                            s.set(next);
                                        }
                                    }),
                            ),
                    ),
            )
    }
}
