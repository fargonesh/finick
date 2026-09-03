
#[derive(PartialEq)]
struct Bluetooth;

impl Component for Bluetooth {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let mut devices = use_state(|| Vec::<BluetoothDevice>::new());
        let mut is_enabled = use_state(|| false);
        let mut is_loading = use_state(|| true);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut dev_state = devices.clone();
            let mut bt_state = is_enabled.clone();
            let mut loading_state = is_loading.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let backend = HyprlandBackend;
                let status = backend.get_bluetooth_status();
                let devs = backend.get_paired_bluetooth_devices();
                let _ = tx.send((status, devs));
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

        rect()
            .child(page_header(
                "Bluetooth",
                "Manage Bluetooth adapter, paired devices, and connections.",
            ))
            .child(
                rect()
                    .margin((0., 0., 24., 0.))
                    .padding(24.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .margin((0., 0., 8., 0.))
                            .child(
                                label()
                                    .font_size(16.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_primary)
                                    .width(Size::fill())
                                    .text("Bluetooth Adapter"),
                            )
                            .child(
                                Switch::new()
                                    .toggled(bt_active)
                                    .on_toggle({
                                        let mut enabled_state = is_enabled.clone();
                                        let mut l = loaded.clone();
                                        let mut load_state = is_loading.clone();
                                        move |_| {
                                            let next = !*enabled_state.read();
                                            enabled_state.set(next);
                                            load_state.set(true);
                                            std::thread::spawn(move || {
                                                let backend = HyprlandBackend;
                                                backend.set_bluetooth_status(next);
                                                std::thread::sleep(std::time::Duration::from_millis(400));
                                            });
                                            l.set(false);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        label()
                            .color(t.text_secondary)
                            .font_size(14.)
                            .margin((0., 0., 16., 0.))
                            .text(if bt_active {
                                "Bluetooth is powered on and ready to connect."
                            } else {
                                "Bluetooth is turned off. Turn it on to connect to devices."
                            }),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .child(secondary_button("Refresh", {
                                let mut l = loaded.clone();
                                let mut load_state = is_loading.clone();
                                move || {
                                    load_state.set(true);
                                    l.set(false);
                                }
                            })),
                    ),
            )
            .child(
                rect()
                    .margin((0., 0., 24., 0.))
                    .padding(24.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .margin((0., 0., 16., 0.))
                            .child(
                                label()
                                    .font_size(16.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_primary)
                                    .width(Size::fill())
                                    .text(format!("Paired Devices ({})", paired_devices.len())),
                            ),
                    )
                    .child({
                        if loading {
                            rect()
                                .padding(24.)
                                .center()
                                .child(
                                    label()
                                        .font_size(14.)
                                        .color(t.text_secondary)
                                        .text("Loading paired Bluetooth devices..."),
                                )
                                .into_element()
                        } else if paired_devices.is_empty() {
                            rect()
                                .padding(24.)
                                .center()
                                .child(
                                    label()
                                        .font_size(14.)
                                        .color(t.text_muted)
                                        .text("No paired Bluetooth devices found."),
                                )
                                .into_element()
                        } else {
                            rect()
                                .children(paired_devices.into_iter().map(|dev| {
                                    let is_conn = dev.connected;
                                    let mac = dev.mac.clone();
                                    let name = dev.name.clone();
                                    let mut l = loaded.clone();
                                    let mut load_state = is_loading.clone();

                                    rect()
                                        .key(dev.mac.clone())
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .padding(12.)
                                        .margin((0., 0., 8., 0.))
                                        .corner_radius(8.)
                                        .background(if is_conn { t.bg_active } else { t.bg_base })
                                        .border(Border::new().width(1.).fill(if is_conn { t.primary_accent } else { t.border_subtle }))
                                        .child(
                                            rect()
                                                .width(Size::px(10.))
                                                .height(Size::px(10.))
                                                .corner_radius(5.)
                                                .background(if is_conn { t.accent_green } else { t.text_disabled })
                                                .margin((0., 12., 0., 0.)),
                                        )
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .child(
                                                    label()
                                                        .font_size(14.)
                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                        .color(t.text_primary)
                                                        .text(name),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(12.)
                                                        .color(if is_conn { t.accent_green } else { t.text_secondary })
                                                        .text(if is_conn {
                                                            format!("Connected • {}", mac)
                                                        } else {
                                                            format!("Paired • {}", mac)
                                                        }),
                                                ),
                                        )
                                        .child(
                                            rect()
                                                .horizontal()
                                                .spacing(8.)
                                                .child(secondary_button(
                                                    if is_conn { "Disconnect" } else { "Connect" },
                                                    {
                                                        let mac_c = mac.clone();
                                                        let mut l_c = l.clone();
                                                        let mut ls_c = load_state.clone();
                                                        move || {
                                                            let m = mac_c.clone();
                                                            ls_c.set(true);
                                                            std::thread::spawn(move || {
                                                                let backend = HyprlandBackend;
                                                                if is_conn {
                                                                    backend.disconnect_bluetooth_device(&m);
                                                                } else {
                                                                    backend.connect_bluetooth_device(&m);
                                                                }
                                                                std::thread::sleep(std::time::Duration::from_millis(800));
                                                            });
                                                            l_c.set(false);
                                                        }
                                                    },
                                                ))
                                                .child(secondary_button("Forget", {
                                                    let mac_c = mac.clone();
                                                    let mut l_c = l.clone();
                                                    let mut ls_c = load_state.clone();
                                                    move || {
                                                        let m = mac_c.clone();
                                                        ls_c.set(true);
                                                        std::thread::spawn(move || {
                                                            let backend = HyprlandBackend;
                                                            backend.remove_bluetooth_device(&m);
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
            )
    }
}
