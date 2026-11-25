use {
    freya::prelude::*,
    std::{collections::HashSet, process::Command},
    ui::*,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

#[derive(PartialEq)]
pub struct Bluetooth;

impl Component for Bluetooth {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let devices = use_state(Vec::<BluetoothDevice>::new);
        let is_enabled = use_state(|| false);
        let is_loading = use_state(|| true);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut dev_state = devices;
            let mut bt_state = is_enabled;
            let mut loading_state = is_loading;

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let _ = Command::new("bluetoothctl").args(["scan", "on"]).output();
                let mut is_powered = false;
                if let Ok(output) = Command::new("bluetoothctl").arg("show").output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        let line = line.trim();
                        if line.starts_with("Powered:") {
                            if line.contains("yes") {
                                is_powered = true;
                            }
                            break;
                        }
                    }
                }

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

                let mut devs = Vec::new();
                if let Ok(output) = Command::new("bluetoothctl").arg("devices").output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
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
                                let connected = connected_macs.contains(&mac.to_uppercase());
                                if !devs.iter().any(|d: &BluetoothDevice| d.mac.eq_ignore_ascii_case(&mac)) {
                                    devs.push(BluetoothDevice { mac, name, connected });
                                }
                            }
                        }
                    }
                }

                let _ = tx.send((is_powered, devs));
            });

            freya::prelude::spawn(async move {
                if let Some((status, devs)) = rx.recv().await {
                    bt_state.set(status);
                    dev_state.set(devs);
                    loading_state.set(false);
                }
            });
        }

        let paired_devices = devices.read().clone();
        let bt_active = *is_enabled.read();
        let loading = *is_loading.read();

        FadeSlideIn::new().child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(page_head(BLUETOOTH, "Bluetooth", "Manage Bluetooth adapter, paired devices, and connections."))
                .child(
                    tile()
                        .child(tile_head(
                            Some(BLUETOOTH),
                            "Bluetooth Adapter",
                            Some(pill_switch(bt_active, {
                                let mut enabled_state = is_enabled;
                                let mut l = loaded;
                                let mut load_state = is_loading;
                                move |v| {
                                    enabled_state.set(v);
                                    load_state.set(true);
                                    std::thread::spawn(move || {
                                        let _ = Command::new("bluetoothctl")
                                            .args(if v { &["power", "on"] } else { &["power", "off"] })
                                            .output();
                                        std::thread::sleep(std::time::Duration::from_millis(400));
                                    });
                                    l.set(false);
                                }
                            })),
                        ))
                        .child(tile_sub(if bt_active {
                            "Bluetooth is powered on and ready to connect."
                        } else {
                            "Bluetooth is turned off. Turn it on to connect to devices."
                        }))
                        .child(rect().horizontal().spacing(8.).child(secondary_button("Refresh", {
                            let mut l = loaded;
                            let mut load_state = is_loading;
                            move || {
                                load_state.set(true);
                                l.set(false);
                            }
                        }))),
                )
                .child(
                    tile()
                        .child(tile_head(None, format!("Paired devices ({})", paired_devices.len()), None::<String>))
                        .child({
                            if loading {
                                rect()
                                    .width(Size::fill())
                                    .padding(16.)
                                    .center()
                                    .child(
                                        label()
                                            .font_size(14.)
                                            .color(t.text_dim)
                                            .text("Loading paired Bluetooth devices..."),
                                    )
                                    .into_element()
                            } else if paired_devices.is_empty() {
                                rect()
                                    .width(Size::fill())
                                    .padding(16.)
                                    .center()
                                    .child(
                                        label()
                                            .font_size(14.)
                                            .color(t.text_dim)
                                            .text("No paired Bluetooth devices found."),
                                    )
                                    .into_element()
                            } else {
                                rect()
                                    .children(paired_devices.into_iter().map(|dev| {
                                        let is_conn = dev.connected;
                                        let mac = dev.mac.clone();
                                        let name = dev.name.clone();
                                        let l = loaded;
                                        let load_state = is_loading;

                                        rect()
                                            .key(dev.mac.clone())
                                            .width(Size::fill())
                                            .horizontal()
                                            .main_align(Alignment::SpaceBetween)
                                            .cross_align(Alignment::Center)
                                            .padding((10., 12.))
                                            .margin((0., 0., 6., 0.))
                                            .corner_radius(8.)
                                            .background(if is_conn { t.bg_active } else { t.panel_raised })
                                            .border(Border::new().width(1.).fill(if is_conn {
                                                t.primary_accent
                                            } else {
                                                t.border
                                            }))
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::Center)
                                                    .child(
                                                        rect()
                                                            .width(Size::px(10.))
                                                            .height(Size::px(10.))
                                                            .corner_radius(5.)
                                                            .background(if is_conn {
                                                                t.accent_green
                                                            } else {
                                                                t.text_disabled
                                                            })
                                                            .margin((0., 12., 0., 0.)),
                                                    )
                                                    .child(
                                                        rect()
                                                            .child(
                                                                label()
                                                                    .font_size(14.)
                                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                                    .color(t.text)
                                                                    .text(name),
                                                            )
                                                            .child(
                                                                label()
                                                                    .font_size(12.)
                                                                    .color(if is_conn {
                                                                        t.accent_green
                                                                    } else {
                                                                        t.text_dim
                                                                    })
                                                                    .text(if is_conn {
                                                                        format!("Connected • {}", mac)
                                                                    } else {
                                                                        format!("Paired • {}", mac)
                                                                    }),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::Center)
                                                    .spacing(6.)
                                                    .content(Content::Flex)
                                                    .child(ghost_button(if is_conn { "Disconnect" } else { "Connect" }, {
                                                        let mac_c = mac.clone();
                                                        let mut l_c = l;
                                                        let mut ls_c = load_state;
                                                        move || {
                                                            let m = mac_c.clone();
                                                            ls_c.set(true);
                                                            std::thread::spawn(move || {
                                                                if is_conn {
                                                                    let _ = Command::new("bluetoothctl")
                                                                        .args(["disconnect", &m])
                                                                        .output();
                                                                } else {
                                                                    let _ = Command::new("bluetoothctl")
                                                                        .args(["connect", &m])
                                                                        .output();
                                                                }
                                                                std::thread::sleep(std::time::Duration::from_millis(800));
                                                            });
                                                            l_c.set(false);
                                                        }
                                                    }))
                                                    .child(ghost_button("Forget", {
                                                        let mac_c = mac.clone();
                                                        let mut l_c = l;
                                                        let mut ls_c = load_state;
                                                        move || {
                                                            let m = mac_c.clone();
                                                            ls_c.set(true);
                                                            std::thread::spawn(move || {
                                                                let _ = Command::new("bluetoothctl")
                                                                    .args(["remove", &m])
                                                                    .output();
                                                                std::thread::sleep(std::time::Duration::from_millis(500));
                                                            });
                                                            l_c.set(false);
                                                        }
                                                    })),
                                            )
                                            .into_element()
                                    }))
                                    .into_element()
                            }
                        }),
                ),
        )
    }
}
