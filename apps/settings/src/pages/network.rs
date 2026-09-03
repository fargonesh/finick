use freya::prelude::*;
use std::collections::HashSet;
use std::process::Command;
use ui::*;

/// Wi-Fi Network representation.
#[derive(Clone, Debug, PartialEq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub security: String,
    pub connected: bool,
    pub is_known: bool,
}

/// VPN connection representation.
#[derive(Clone, Debug, PartialEq)]
pub struct VpnConnection {
    pub name: String,
    pub interface: String,
    pub vpn_type: String,
    pub connected: bool,
    pub ip_address: Option<String>,
}

/// Consolidated network status and configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkInfo {
    pub wifi_enabled: bool,
    pub active_ssid: Option<String>,
    pub available_networks: Vec<WifiNetwork>,
    pub known_networks: Vec<WifiNetwork>,
    pub vpns: Vec<VpnConnection>,
    pub primary_ip: String,
    pub primary_interface: String,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            wifi_enabled: true,
            active_ssid: Some("Dark Depths".to_string()),
            available_networks: vec![
                WifiNetwork {
                    ssid: "Dark Depths".to_string(),
                    signal: 78,
                    security: "WPA2/WPA3".to_string(),
                    connected: true,
                    is_known: true,
                },
                WifiNetwork {
                    ssid: "OPTUS_FF7C2CL".to_string(),
                    signal: 54,
                    security: "WPA2".to_string(),
                    connected: false,
                    is_known: false,
                },
                WifiNetwork {
                    ssid: "dd_iot".to_string(),
                    signal: 90,
                    security: "WPA2".to_string(),
                    connected: false,
                    is_known: true,
                },
                WifiNetwork {
                    ssid: "VX420-34F8".to_string(),
                    signal: 32,
                    security: "WPA2".to_string(),
                    connected: false,
                    is_known: false,
                },
            ],
            known_networks: vec![
                WifiNetwork {
                    ssid: "Dark Depths".to_string(),
                    signal: 78,
                    security: "WPA2/WPA3".to_string(),
                    connected: true,
                    is_known: true,
                },
                WifiNetwork {
                    ssid: "Flora's iPhone".to_string(),
                    signal: 0,
                    security: "WPA2".to_string(),
                    connected: false,
                    is_known: true,
                },
                WifiNetwork {
                    ssid: "eduroam".to_string(),
                    signal: 0,
                    security: "WPA2-Enterprise".to_string(),
                    connected: false,
                    is_known: true,
                },
                WifiNetwork {
                    ssid: "Home_5G_Backup".to_string(),
                    signal: 0,
                    security: "WPA3".to_string(),
                    connected: false,
                    is_known: true,
                },
            ],
            vpns: vec![
                VpnConnection {
                    name: "Tailscale Mesh".to_string(),
                    interface: "tailscale0".to_string(),
                    vpn_type: "Mesh VPN".to_string(),
                    connected: true,
                    ip_address: Some("100.64.0.5".to_string()),
                },
                VpnConnection {
                    name: "Work WireGuard".to_string(),
                    interface: "wg0".to_string(),
                    vpn_type: "WireGuard".to_string(),
                    connected: false,
                    ip_address: None,
                },
            ],
            primary_ip: "192.168.1.105".to_string(),
            primary_interface: "wlp166s0".to_string(),
        }
    }
}

/// Helper function to retrieve IPv4 address for a network interface using `ip`.
fn get_interface_ip(iface: &str) -> Option<String> {
    if let Ok(output) = Command::new("ip").args(["-br", "addr", "show", "dev", iface]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                return Some(parts[2].split('/').next().unwrap_or(parts[2]).to_string());
            }
        }
    }
    None
}

/// Query system networking details via `nmcli` and `ip`, falling back to realistic mock data.
pub fn fetch_network_info() -> NetworkInfo {
    let mut info = NetworkInfo::default();
    let mut real_data_detected = false;

    // 1. Check Wi-Fi radio status
    if let Ok(output) = Command::new("nmcli").args(["radio", "wifi"]).output() {
        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !status.is_empty() {
            info.wifi_enabled = status == "enabled";
            real_data_detected = true;
        }
    }

    // 2. Fetch known / saved connections
    let mut known_ssids = HashSet::new();
    if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "NAME,TYPE", "connection", "show"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && parts[1].contains("802-11-wireless") {
                let name = parts[0].trim().to_string();
                if !name.is_empty() {
                    known_ssids.insert(name);
                }
            }
        }
        if !known_ssids.is_empty() {
            real_data_detected = true;
        }
    }

    // 3. Fetch scanned Wi-Fi networks
    let mut scanned_networks = Vec::new();
    let mut active_ssid = None;
    if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut seen_ssids = HashSet::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                let ssid = parts[0].trim().to_string();
                if ssid.is_empty() || seen_ssids.contains(&ssid) {
                    continue;
                }
                seen_ssids.insert(ssid.clone());

                let signal = parts[1].trim().parse::<u8>().unwrap_or(50);
                let security = if parts[2].trim().is_empty() {
                    "Open".to_string()
                } else {
                    parts[2].trim().to_string()
                };
                let is_connected = parts[3].trim() == "*";
                if is_connected {
                    active_ssid = Some(ssid.clone());
                }
                let is_known = known_ssids.contains(&ssid);

                scanned_networks.push(WifiNetwork {
                    ssid,
                    signal,
                    security,
                    connected: is_connected,
                    is_known,
                });
            }
        }
    }

    if !scanned_networks.is_empty() {
        real_data_detected = true;
        info.available_networks = scanned_networks;
        info.active_ssid = active_ssid;
    }

    // 4. Populate known networks
    if !known_ssids.is_empty() {
        let mut known_list = Vec::new();
        for ssid in &known_ssids {
            let is_connected = info.active_ssid.as_ref() == Some(ssid);
            let signal = info.available_networks.iter()
                .find(|n| &n.ssid == ssid)
                .map(|n| n.signal)
                .unwrap_or(0);
            let security = info.available_networks.iter()
                .find(|n| &n.ssid == ssid)
                .map(|n| n.security.clone())
                .unwrap_or_else(|| "Saved".to_string());

            known_list.push(WifiNetwork {
                ssid: ssid.clone(),
                signal,
                security,
                connected: is_connected,
                is_known: true,
            });
        }
        known_list.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.ssid.cmp(&b.ssid)));
        info.known_networks = known_list;
    }

    // 5. Detect VPN & Tunnel interfaces from `ip link`
    let mut vpns = Vec::new();
    if let Ok(output) = Command::new("ip").args(["-br", "link", "show"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let iface = parts[0].trim();
                let state = parts[1].trim();
                let is_up = state.eq_ignore_ascii_case("UP") || state.eq_ignore_ascii_case("UNKNOWN");

                if iface.starts_with("tailscale") {
                    let ip = get_interface_ip(iface);
                    vpns.push(VpnConnection {
                        name: "Tailscale Mesh".to_string(),
                        interface: iface.to_string(),
                        vpn_type: "Tailscale".to_string(),
                        connected: is_up,
                        ip_address: ip,
                    });
                } else if iface.starts_with("tun") || iface.starts_with("tap") {
                    let ip = get_interface_ip(iface);
                    vpns.push(VpnConnection {
                        name: format!("Tunnel ({})", iface),
                        interface: iface.to_string(),
                        vpn_type: "VPN Tunnel".to_string(),
                        connected: is_up,
                        ip_address: ip,
                    });
                } else if iface.starts_with("wg") {
                    let ip = get_interface_ip(iface);
                    vpns.push(VpnConnection {
                        name: format!("WireGuard ({})", iface),
                        interface: iface.to_string(),
                        vpn_type: "WireGuard".to_string(),
                        connected: is_up,
                        ip_address: ip,
                    });
                }
            }
        }
    }

    if !vpns.is_empty() {
        real_data_detected = true;
        info.vpns = vpns;
    }

    // 6. Query Primary IP and Interface from `ip route`
    if let Ok(output) = Command::new("ip").args(["route", "get", "1.1.1.1"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.split_whitespace().collect();
        for i in 0..parts.len() {
            if parts[i] == "dev" && i + 1 < parts.len() {
                info.primary_interface = parts[i + 1].to_string();
            }
            if parts[i] == "src" && i + 1 < parts.len() {
                info.primary_ip = parts[i + 1].to_string();
            }
        }
    }

    if !real_data_detected {
        NetworkInfo::default()
    } else {
        info
    }
}

/// Set Wi-Fi radio status via `nmcli`.
pub fn set_wifi_enabled(enabled: bool) {
    let _ = Command::new("nmcli")
        .args(["radio", "wifi", if enabled { "on" } else { "off" }])
        .output();
}

/// Connect to a Wi-Fi network SSID via `nmcli`.
pub fn connect_wifi(ssid: &str) {
    let _ = Command::new("nmcli")
        .args(["dev", "wifi", "connect", ssid])
        .output();
}

/// Disconnect from a Wi-Fi connection via `nmcli`.
pub fn disconnect_wifi(ssid: &str) {
    let _ = Command::new("nmcli")
        .args(["connection", "down", ssid])
        .output();
}

/// Delete a saved Wi-Fi connection via `nmcli`.
pub fn forget_wifi(ssid: &str) {
    let _ = Command::new("nmcli")
        .args(["connection", "delete", ssid])
        .output();
}

/// Toggle or reconnect VPN interface.
pub fn toggle_vpn(vpn: &VpnConnection) {
    if vpn.interface.starts_with("tailscale") {
        let arg = if vpn.connected { "down" } else { "up" };
        let _ = Command::new("tailscale").arg(arg).output();
    } else {
        let arg = if vpn.connected { "down" } else { "up" };
        let _ = Command::new("nmcli").args(["connection", arg, &vpn.name]).output();
    }
}

#[derive(PartialEq)]
pub struct Network;

impl Component for Network {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let network_state = use_state(NetworkInfo::default);
        let is_loading = use_state(|| true);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut state_clone = network_state.clone();
            let mut loading_clone = is_loading.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<NetworkInfo>();

            std::thread::spawn(move || {
                let info = fetch_network_info();
                let _ = tx.send(info);
            });

            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    state_clone.set(info);
                    loading_clone.set(false);
                }
            });
        }

        let info = network_state.read().clone();
        let loading = *is_loading.read();
        let wifi_on = info.wifi_enabled;

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(page_header("Network", "Manage network connections, Wi-Fi networks, and VPN status."))
                    
                    // --- Status & Overview Card ---
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
                                    .margin((0., 0., 12., 0.))
                                    .content(Content::Flex)
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(
                                                label()
                                                    .font_size(16.)
                                                    .font_weight(FontWeight::BOLD)
                                                    .color(t.text_primary)
                                                    .text("Wi-Fi Adapter")
                                            )
                                            .child(
                                                label()
                                                    .font_size(13.)
                                                    .color(t.text_secondary)
                                                    .margin((4., 0., 0., 0.))
                                                    .text(if wifi_on {
                                                        if let Some(ref ssid) = info.active_ssid {
                                                            format!("Connected to {}", ssid)
                                                        } else {
                                                            "Wi-Fi is on, not connected".to_string()
                                                        }
                                                    } else {
                                                        "Wi-Fi is disabled".to_string()
                                                    })
                                            )
                                    )
                                    .child(
                                        Switch::new()
                                            .toggled(wifi_on)
                                            .on_toggle({
                                                let mut ns = network_state.clone();
                                                let mut l = loaded.clone();
                                                let mut ld = is_loading.clone();
                                                move |_| {
                                                    let next_val = !wifi_on;
                                                    let mut curr = ns.read().clone();
                                                    curr.wifi_enabled = next_val;
                                                    ns.set(curr);
                                                    ld.set(true);
                                                    std::thread::spawn(move || {
                                                        set_wifi_enabled(next_val);
                                                        std::thread::sleep(std::time::Duration::from_millis(500));
                                                    });
                                                    l.set(false);
                                                }
                                            })
                                    )
                            )
                            // Network details strip
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .spacing(16.)
                                    .cross_align(Alignment::Center)
                                    .content(Content::Flex)
                                    .padding((10., 14.))
                                    .margin((8., 0., 14., 0.))
                                    .corner_radius(8.)
                                    .background(t.bg_base)
                                    .border(Border::new().width(1.).fill(t.border_subtle))
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("IP ADDRESS"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(info.primary_ip.clone()))
                                    )
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("INTERFACE"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(info.primary_interface.clone()))
                                    )
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("STATUS"))
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::Center)
                                                    .spacing(6.)
                                                    .child(
                                                        rect()
                                                            .width(Size::px(8.))
                                                            .height(Size::px(8.))
                                                            .corner_radius(4.)
                                                            .background(if wifi_on && info.active_ssid.is_some() { t.accent_green } else { t.text_disabled })
                                                    )
                                                    .child(
                                                        label()
                                                            .font_size(14.)
                                                            .font_weight(FontWeight::SEMI_BOLD)
                                                            .color(if wifi_on && info.active_ssid.is_some() { t.accent_green } else { t.text_secondary })
                                                            .text(if wifi_on && info.active_ssid.is_some() { "Online" } else { "Offline" })
                                                    )
                                            )
                                    )
                            )
                            // Action buttons
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .child(
                                        secondary_button(if loading { "Scanning..." } else { "Scan & Refresh" }, {
                                            let mut l = loaded.clone();
                                            let mut ld = is_loading.clone();
                                            move || {
                                                ld.set(true);
                                                l.set(false);
                                            }
                                        })
                                    )
                            )
                    )

                    // --- Available Wi-Fi Networks Section ---
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
                                    .cross_align(Alignment::Center)
                                    .margin((0., 0., 14., 0.))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_secondary)
                                            .text(format!("AVAILABLE NETWORKS ({})", info.available_networks.len()))
                                    )
                            )
                            .child({
                                if !wifi_on {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("Wi-Fi is turned off. Turn on Wi-Fi to scan for nearby networks."))
                                        .into_element()
                                } else if loading && info.available_networks.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_secondary).text("Scanning for Wi-Fi networks..."))
                                        .into_element()
                                } else if info.available_networks.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("No wireless networks found in range."))
                                        .into_element()
                                } else {
                                    rect()
                                        .children(info.available_networks.into_iter().map(|net| {
                                            let ssid = net.ssid.clone();
                                            let is_conn = net.connected;
                                            let sig = net.signal;
                                            let sec = net.security.clone();
                                            let l = loaded.clone();
                                            let ld = is_loading.clone();

                                            rect()
                                                .key(net.ssid.clone())
                                                .width(Size::fill())
                                                .horizontal()
                                                .main_align(Alignment::SpaceBetween)
                                                .cross_align(Alignment::Center)
                                                .padding((10., 12.))
                                                .margin((0., 0., 6., 0.))
                                                .corner_radius(8.)
                                                .background(if is_conn { t.bg_active } else { t.bg_base })
                                                .border(Border::new().width(1.).fill(if is_conn { t.primary_accent } else { t.border_subtle }))
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            rect()
                                                                .width(Size::px(10.))
                                                                .height(Size::px(10.))
                                                                .corner_radius(5.)
                                                                .background(if is_conn { t.accent_green } else { t.primary_accent })
                                                                .margin((0., 12., 0., 0.))
                                                        )
                                                        .child(
                                                            rect()
                                                                .child(
                                                                    label()
                                                                        .font_size(14.)
                                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                                        .color(t.text_primary)
                                                                        .text(ssid.clone())
                                                                )
                                                                .child(
                                                                    rect()
                                                                        .horizontal()
                                                                        .spacing(8.)
                                                                        .margin((2., 0., 0., 0.))
                                                                        .child(
                                                                            label()
                                                                                .font_size(12.)
                                                                                .color(if is_conn { t.accent_green } else { t.text_secondary })
                                                                                .text(if is_conn { "Connected".to_string() } else { format!("Signal: {}%", sig) })
                                                                        )
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(sec))
                                                                )
                                                        )
                                                )
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .spacing(8.)
                                                        .child(
                                                            secondary_button(if is_conn { "Disconnect" } else { "Connect" }, {
                                                                let ssid_c = ssid.clone();
                                                                let mut l_c = l.clone();
                                                                let mut ld_c = ld.clone();
                                                                move || {
                                                                    let s = ssid_c.clone();
                                                                    ld_c.set(true);
                                                                    std::thread::spawn(move || {
                                                                        if is_conn {
                                                                            disconnect_wifi(&s);
                                                                        } else {
                                                                            connect_wifi(&s);
                                                                        }
                                                                        std::thread::sleep(std::time::Duration::from_millis(800));
                                                                    });
                                                                    l_c.set(false);
                                                                }
                                                            })
                                                        )
                                                )
                                                .into_element()
                                        }))
                                        .into_element()
                                }
                            })
                    )

                    // --- Known Networks Section ---
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
                                    .cross_align(Alignment::Center)
                                    .margin((0., 0., 14., 0.))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_secondary)
                                            .text(format!("KNOWN NETWORKS ({})", info.known_networks.len()))
                                    )
                            )
                            .child({
                                if info.known_networks.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("No known networks saved on this system."))
                                        .into_element()
                                } else {
                                    rect()
                                        .children(info.known_networks.into_iter().map(|net| {
                                            let ssid = net.ssid.clone();
                                            let is_conn = net.connected;
                                            let sec = net.security.clone();
                                            let l = loaded.clone();
                                            let ld = is_loading.clone();

                                            rect()
                                                .key(format!("known-{}", net.ssid))
                                                .width(Size::fill())
                                                .horizontal()
                                                .main_align(Alignment::SpaceBetween)
                                                .cross_align(Alignment::Center)
                                                .padding((10., 12.))
                                                .margin((0., 0., 6., 0.))
                                                .corner_radius(8.)
                                                .background(if is_conn { t.bg_active } else { t.bg_base })
                                                .border(Border::new().width(1.).fill(if is_conn { t.primary_accent } else { t.border_subtle }))
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            rect()
                                                                .width(Size::px(10.))
                                                                .height(Size::px(10.))
                                                                .corner_radius(5.)
                                                                .background(if is_conn { t.accent_green } else { t.text_disabled })
                                                                .margin((0., 12., 0., 0.))
                                                        )
                                                        .child(
                                                            rect()
                                                                .child(
                                                                    label()
                                                                        .font_size(14.)
                                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                                        .color(t.text_primary)
                                                                        .text(ssid.clone())
                                                                )
                                                                .child(
                                                                    label()
                                                                        .font_size(12.)
                                                                        .color(if is_conn { t.accent_green } else { t.text_secondary })
                                                                        .margin((2., 0., 0., 0.))
                                                                        .text(if is_conn { "Connected • Saved Profile".to_string() } else { format!("Saved Profile • {}", sec) })
                                                                )
                                                        )
                                                )
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .spacing(8.)
                                                        .child(
                                                            secondary_button(if is_conn { "Disconnect" } else { "Connect" }, {
                                                                let ssid_c = ssid.clone();
                                                                let mut l_c = l.clone();
                                                                let mut ld_c = ld.clone();
                                                                move || {
                                                                    let s = ssid_c.clone();
                                                                    ld_c.set(true);
                                                                    std::thread::spawn(move || {
                                                                        if is_conn {
                                                                            disconnect_wifi(&s);
                                                                        } else {
                                                                            connect_wifi(&s);
                                                                        }
                                                                        std::thread::sleep(std::time::Duration::from_millis(800));
                                                                    });
                                                                    l_c.set(false);
                                                                }
                                                            })
                                                        )
                                                        .child(
                                                            secondary_button("Forget", {
                                                                let ssid_c = ssid.clone();
                                                                let mut l_c = l.clone();
                                                                let mut ld_c = ld.clone();
                                                                move || {
                                                                    let s = ssid_c.clone();
                                                                    ld_c.set(true);
                                                                    std::thread::spawn(move || {
                                                                        forget_wifi(&s);
                                                                        std::thread::sleep(std::time::Duration::from_millis(500));
                                                                    });
                                                                    l_c.set(false);
                                                                }
                                                            })
                                                        )
                                                )
                                                .into_element()
                                        }))
                                        .into_element()
                                }
                            })
                    )

                    // --- VPN & Secure Tunnels Section ---
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
                                    .cross_align(Alignment::Center)
                                    .margin((0., 0., 14., 0.))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_secondary)
                                            .text(format!("VPN & SECURE TUNNELS ({})", info.vpns.len()))
                                    )
                            )
                            .child({
                                if info.vpns.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("No active or configured VPN tunnels found."))
                                        .into_element()
                                } else {
                                    rect()
                                        .children(info.vpns.into_iter().map(|vpn| {
                                            let vpn_clone = vpn.clone();
                                            let is_conn = vpn.connected;
                                            let ip_str = vpn.ip_address.clone().unwrap_or_else(|| "No IP assigned".to_string());
                                            let l = loaded.clone();
                                            let ld = is_loading.clone();

                                            rect()
                                                .key(format!("vpn-{}", vpn.interface))
                                                .width(Size::fill())
                                                .horizontal()
                                                .main_align(Alignment::SpaceBetween)
                                                .cross_align(Alignment::Center)
                                                .padding((10., 12.))
                                                .margin((0., 0., 6., 0.))
                                                .corner_radius(8.)
                                                .background(if is_conn { t.bg_active } else { t.bg_base })
                                                .border(Border::new().width(1.).fill(if is_conn { t.primary_accent } else { t.border_subtle }))
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            rect()
                                                                .width(Size::px(10.))
                                                                .height(Size::px(10.))
                                                                .corner_radius(5.)
                                                                .background(if is_conn { t.accent_green } else { t.text_disabled })
                                                                .margin((0., 12., 0., 0.))
                                                        )
                                                        .child(
                                                            rect()
                                                                .child(
                                                                    label()
                                                                        .font_size(14.)
                                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                                        .color(t.text_primary)
                                                                        .text(vpn.name.clone())
                                                                )
                                                                .child(
                                                                    rect()
                                                                        .horizontal()
                                                                        .spacing(8.)
                                                                        .margin((2., 0., 0., 0.))
                                                                        .child(
                                                                            label()
                                                                                .font_size(12.)
                                                                                .color(if is_conn { t.accent_green } else { t.text_secondary })
                                                                                .text(if is_conn { "Active" } else { "Disconnected" })
                                                                        )
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(format!("Interface: {}", vpn.interface)))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(ip_str))
                                                                )
                                                        )
                                                )
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .spacing(8.)
                                                        .child(
                                                            secondary_button(if is_conn { "Disconnect" } else { "Connect" }, {
                                                                let v = vpn_clone.clone();
                                                                let mut l_c = l.clone();
                                                                let mut ld_c = ld.clone();
                                                                move || {
                                                                    let v_act = v.clone();
                                                                    ld_c.set(true);
                                                                    std::thread::spawn(move || {
                                                                        toggle_vpn(&v_act);
                                                                        std::thread::sleep(std::time::Duration::from_millis(800));
                                                                    });
                                                                    l_c.set(false);
                                                                }
                                                            })
                                                        )
                                                )
                                                .into_element()
                                        }))
                                        .into_element()
                                }
                            })
                    )
            )
    }
}
