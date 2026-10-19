use {
    crate::state::{
        apply, load_extended_batch, load_initial_batch, open_in_settings, parse_capacity_pct, power_action,
        subscribe_extended_live, subscribe_live,
    },
    freya::prelude::*,
    ipsea::settings::{SettingKey, SettingValue, set_and_apply},
    system::{BluetoothDevice, CurrentWifiInfo, HyprlandBackend, SystemBackend, WifiNetworkDetail, WiredInfo},
    ui::*,
};
fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    }
}

pub fn control_panel_app() -> Element {
    let theme_state = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let wifi = use_state(|| true);
    let bt = use_state(|| true);
    let volume = use_state(|| 65.0);
    let muted = use_state(|| false);
    let brightness = use_state(|| 72.0);
    let dnd = use_state(|| false);
    let connected = use_state(|| false);
    let wired = use_state(|| Option::<WiredInfo>::None);
    let battery_pct = use_state(|| 0u8);
    let battery_status = use_state(|| "Unknown".to_string());
    let wifi_details = use_state(|| (CurrentWifiInfo::default(), Vec::<WifiNetworkDetail>::new()));
    let bt_devices = use_state(Vec::<BluetoothDevice>::new);
    let topbar_settings = use_state(crate::state::TopbarSettings::default);
    let pending_power = use_state(|| Option::<&'static str>::None);
    let airplane = use_state(|| false);
    let night = use_state(|| false);
    let power_profile = use_state(|| "balanced".to_string());
    let kbd_level = use_state(|| Option::<u32>::None);
    let kbd_supported = use_state(|| false);
    let bg_apps = use_state(|| 0usize);
    let power_supported = use_state(|| true);
    let last_error = use_state(|| Option::<String>::None);
    let dnd_sched = use_state(crate::state::DndSchedule::default);
    let mut dismiss = use_state(|| false);
    use_hook(move || {
        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
        subscribe_live(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
        load_extended_batch(airplane, night, power_profile);
        subscribe_extended_live(airplane, night, power_profile);
        {
            let mut ds = dnd_sched;
            spawn(async move {
                let sched = tokio::task::spawn_blocking(crate::state::load_dnd_schedule).await.unwrap_or_default();
                ds.set_if_modified(sched);
            });
        }
        let mut wired_state = wired;
        let mut pct_state = battery_pct;
        let mut status_state = battery_status;
        let mut wifi_state = wifi_details;
        let mut bt_state = bt_devices;
        let mut airplane_state = airplane;
        let mut night_state = night;
        let mut profile_state = power_profile;
        let mut kbd_level_state = kbd_level;
        let mut kbd_supported_state = kbd_supported;
        let mut bg_state = bg_apps;
        let mut power_supported_state = power_supported;

        // Fetch initial data asynchronously without blocking window creation

        spawn(async move {
            let initial_wifi = tokio::task::spawn_blocking(|| HyprlandBackend.get_wifi_details()).await.unwrap_or_default();
            wifi_state.set_if_modified(initial_wifi);
        });

        spawn(async move {
            let initial_bt =
                tokio::task::spawn_blocking(|| HyprlandBackend.get_paired_bluetooth_devices()).await.unwrap_or_default();
            bt_state.set_if_modified(initial_bt);
        });

        spawn(async move {
            let wired_now = tokio::task::spawn_blocking(|| HyprlandBackend.get_wired_info()).await.unwrap_or(None);
            wired_state.set_if_modified(wired_now);
            let power =
                tokio::task::spawn_blocking(|| HyprlandBackend.get_power_info()).await.unwrap_or(system::PowerInfo {
                    capacity: "0%".to_string(),
                    status: "Unknown".to_string(),
                    health_percent: None,
                    cycle_count: None,
                });
            let p = parse_capacity_pct(&power.capacity);
            pct_state.set_if_modified(p);
            status_state.set_if_modified(power.status);
        });

        spawn(async move {
            let ap = tokio::task::spawn_blocking(system::query_airplane_mode).await.unwrap_or(false);
            airplane_state.set_if_modified(ap);
            let prof = tokio::task::spawn_blocking(|| HyprlandBackend.get_power_profile()).await.unwrap_or_default();
            if !prof.is_empty() {
                profile_state.set_if_modified(prof);
            }
            let sup = tokio::task::spawn_blocking(system::supports_power_profiles).await.unwrap_or(false);
            power_supported_state.set_if_modified(sup);
            let kbd = tokio::task::spawn_blocking(system::query_keyboard_backlight).await.unwrap_or(None);
            kbd_supported_state.set_if_modified(kbd.is_some());
            kbd_level_state.set_if_modified(kbd);
            let bg = tokio::task::spawn_blocking(system::query_background_app_count).await.unwrap_or(0);
            bg_state.set_if_modified(bg);
            let nz = tokio::task::spawn_blocking(system::load_night_schedule).await.unwrap_or_default();
            night_state.set_if_modified(nz.mode == 1);
        });

        spawn(async move {
            let _ = std::process::Command::new("bluetoothctl").args(["scan", "on"]).output();
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                let wired_now = tokio::task::spawn_blocking(|| HyprlandBackend.get_wired_info()).await.unwrap_or(None);
                wired_state.set_if_modified(wired_now);
                let power =
                    tokio::task::spawn_blocking(|| HyprlandBackend.get_power_info()).await.unwrap_or(system::PowerInfo {
                        capacity: "0%".to_string(),
                        status: "Unknown".to_string(),
                        health_percent: None,
                        cycle_count: None,
                    });
                let p = parse_capacity_pct(&power.capacity);
                pct_state.set_if_modified(p);
                status_state.set_if_modified(power.status.clone());
                let w = tokio::task::spawn_blocking(|| HyprlandBackend.get_wifi_details()).await.unwrap_or_default();
                wifi_state.set_if_modified(w);
                let b =
                    tokio::task::spawn_blocking(|| HyprlandBackend.get_paired_bluetooth_devices()).await.unwrap_or_default();
                bt_state.set_if_modified(b);
                let ap = tokio::task::spawn_blocking(system::query_airplane_mode).await.unwrap_or(false);
                airplane_state.set_if_modified(ap);
                let prof = tokio::task::spawn_blocking(|| HyprlandBackend.get_power_profile()).await.unwrap_or_default();
                if !prof.is_empty() {
                    profile_state.set_if_modified(prof);
                }
                let kbd = tokio::task::spawn_blocking(system::query_keyboard_backlight).await.unwrap_or(None);
                kbd_level_state.set_if_modified(kbd);
                let bg = tokio::task::spawn_blocking(system::query_background_app_count).await.unwrap_or(0);
                bg_state.set_if_modified(bg);
            }
        });
    });
    let wid = Platform::window_id();
    use_hook(move || {
        let my_wid = wid;
        crate::state::spawn_panel_focus_watcher_guarded(
            &["overlay-panel", "topbar-panel", "overlay", "topbar"],
            "control-panel",
            dismiss,
            move || {
                crate::state::is_panel_window_current(my_wid)
            },
        );
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::clear_close_flag(&crate::state::CLOSE_PANEL);
        crate::state::spawn_close_flag_watcher_guarded(&crate::state::CLOSE_PANEL, dismiss, move || {
            crate::state::is_panel_window_current(my_wid)
        });
    });
    let _wifi_on = *wifi.read();
    let _bt_on = *bt.read();
    let wired_info = wired.read().clone();
    let bat_pct = *battery_pct.read();
    let bat_status = battery_status.read().clone();
    let has_battery = !bat_status.is_empty() && bat_status != "Unknown" && bat_pct > 0;
    let (current_wifi, available_networks) = wifi_details.read().clone();
    let bt_list = bt_devices.read().clone();
    let wifi_row_count = if !*wifi.read() {
        0
    } else {
        let n = available_networks
            .iter()
            .filter(|net| !(current_wifi.ssid.as_ref() == Some(&net.ssid) && net.is_connected))
            .take(8)
            .count();
        if n == 0 && current_wifi.ssid.is_none() { 1 } else { n }
    };
    let bt_row_count = if !*bt.read() {
        0
    } else {
        let connected_n = bt_list.iter().filter(|d| d.connected).count();
        let other_n = bt_list.iter().filter(|d| !d.connected).take(8).count();
        let mut n = (if connected_n == 0 { 1 } else { connected_n }) + other_n;
        if bt_list.is_empty() {
            n += 1;
        }
        n
    };
    let row1_list_h = ((wifi_row_count.min(5) as f32 * 30.0 + 8.0).clamp(40.0, 120.0))
        .max((bt_row_count.min(5) as f32 * 32.0 + 8.0).clamp(40.0, 120.0));
    let row2_h = if wired.read().clone().is_some_and(|i| !i.ip_address.is_empty()) { 108.0 } else { 92.0 };

    let wifi_card = {
        let wifi_on = *wifi.read();
        let mut card = tile().child(tile_head(
            Some(WIFI),
            "Wi-Fi",
            Some({
                let mut w = wifi;
                pill_switch(*w.read(), move |v| {
                    w.set_if_modified(v);
                    apply(SettingKey::WifiEnabled, v.into());
                    if v {
                        std::thread::spawn(|| {
                            let _ = std::process::Command::new("nmcli").args(["radio", "wifi", "on"]).output();
                        });
                    } else {
                        std::thread::spawn(|| {
                            let _ = std::process::Command::new("nmcli").args(["radio", "wifi", "off"]).output();
                        });
                    }
                })
            }),
        ));
        if !wifi_on {
            card = card.child(tile_sub("Wi-Fi off"));
        } else {
            if let Some(ssid) = current_wifi.ssid.clone() {
                if current_wifi.connected {
                    let iface = current_wifi.interface.clone();
                    let display = trunc(&ssid, 18);
                    card = card.child(
                        rect()
                            .width(Size::fill())
                            .cursor(CursorIcon::Pointer)
                            .content(Content::Flex)
                            .padding((6., 0.))
                            .on_press(move |_| {
                                let iface = iface.clone();
                                std::thread::spawn(move || {
                                    if let Some(iface) = iface {
                                        let _ =
                                            std::process::Command::new("nmcli").args(["dev", "disconnect", &iface]).output();
                                    } else {
                                        HyprlandBackend.disconnect_wifi();
                                    }
                                });
                            })
                            .child(dot_status_row(
                                display,
                                None::<String>,
                                true,
                                label().font_size(11.).color(t.text_dim).text("Connected"),
                            )),
                    );
                }
            }
            let mut rows: Vec<Element> = Vec::new();
            let iface_clone = current_wifi.interface.clone();
            let mut shown = 0;
            for net in available_networks.iter() {
                if shown >= 8 {
                    break;
                }
                if current_wifi.ssid.as_ref() == Some(&net.ssid) && net.is_connected {
                    continue;
                }
                let ssid = net.ssid.clone();
                let ssid2 = ssid.clone();
                let display = trunc(&ssid, 20);
                let is_connected = current_wifi.ssid.as_ref() == Some(&ssid);
                let iface_for_row = iface_clone.clone();
                let is_secured = !net.is_saved
                    && !net.security.is_empty()
                    && net.security != "--"
                    && !net.security.eq_ignore_ascii_case("Open");
                let net_for_click = net.clone();
                rows.push(
                    rect()
                        .width(Size::fill())
                        .corner_radius(8.)
                        .cursor(CursorIcon::Pointer)
                        .background(Color::TRANSPARENT)
                        .content(Content::Flex)
                        .on_press({
                            move |_| {
                                if is_connected {
                                    let iface = iface_for_row.clone();
                                    std::thread::spawn(move || {
                                        if let Some(iface) = iface {
                                            let _ = std::process::Command::new("nmcli")
                                                .args(["dev", "disconnect", &iface])
                                                .output();
                                        } else {
                                            HyprlandBackend.disconnect_wifi();
                                        }
                                    });
                                } else if is_secured {
                                    let net = net_for_click.clone();
                                    std::thread::spawn(move || {
                                        let _ =
                                            ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::WifiPassword {
                                                ssid: net.ssid,
                                                security: net.security,
                                            });
                                    });
                                } else {
                                    let s = ssid2.clone();
                                    std::thread::spawn(move || {
                                        HyprlandBackend.connect_wifi(&s);
                                    });
                                }
                            }
                        })
                        .child(setting_row(
                            display,
                            None::<String>,
                            false,
                            if is_connected {
                                label().font_size(11.).color(t.accent).text("Connected").into_element()
                            } else {
                                rect().into_element()
                            },
                        ))
                        .into_element(),
                );
                shown += 1;
            }
            if rows.is_empty() && current_wifi.ssid.is_none() {
                rows.push(tile_sub("No networks found").into_element());
            }
            let list_h = row1_list_h;
            card = card.child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::px(list_h))
                    .show_scrollbar(rows.len() > 4)
                    .child(rect().width(Size::fill()).vertical().spacing(2.).content(Content::Flex).children(rows)),
            );
        }
        card
    };

    let bt_card = {
        let bt_on = *bt.read();
        let mut card = tile().child(tile_head(
            Some(BLUETOOTH),
            "Bluetooth",
            Some({
                let mut b = bt;
                pill_switch(*b.read(), move |v| {
                    b.set_if_modified(v);
                    apply(SettingKey::BluetoothEnabled, v.into());
                    let en = v;
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("rfkill")
                            .args([if en { "unblock" } else { "block" }, "bluetooth"])
                            .output();
                        if en {
                            let _ = std::process::Command::new("bluetoothctl").args(["scan", "on"]).output();
                        }
                    });
                })
            }),
        ));
        if !bt_on {
            card = card.child(tile_sub("Bluetooth off"));
        } else {
            let connected_devs: Vec<_> = bt_list.iter().filter(|d| d.connected).cloned().collect();
            let other_devs: Vec<_> = bt_list.iter().filter(|d| !d.connected).cloned().collect();
            let mut rows: Vec<Element> = Vec::new();
            if !connected_devs.is_empty() {
                for dev in connected_devs.iter() {
                    let mac = dev.mac.clone();
                    let name = dev.name.clone();
                    let display = trunc(&name, 20);
                    rows.push(
                        rect()
                            .width(Size::fill())
                            .corner_radius(8.)
                            .cursor(CursorIcon::Pointer)
                            .background(Color::TRANSPARENT)
                            .content(Content::Flex)
                            .on_press(move |_| {
                                let m = mac.clone();
                                std::thread::spawn(move || {
                                    HyprlandBackend.disconnect_bluetooth_device(&m);
                                });
                            })
                            .child(dot_status_row(
                                display.clone(),
                                Some("Connected"),
                                true,
                                icon(CHEVRON_RIGHT, 12., t.text_dim),
                            ))
                            .into_element(),
                    );
                }
            } else {
                rows.push(tile_sub("No device connected").into_element());
            }
            for dev in other_devs.iter().take(8) {
                let name = dev.name.clone();
                let display = trunc(&name, 20);
                let dev_for_click = dev.clone();
                rows.push(
                    rect()
                        .width(Size::fill())
                        .corner_radius(8.)
                        .cursor(CursorIcon::Pointer)
                        .background(Color::TRANSPARENT)
                        .content(Content::Flex)
                        .on_press({
                            move |_| {
                                let dev = dev_for_click.clone();
                                std::thread::spawn(move || {
                                    let _ = ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::BluetoothPair {
                                        name: dev.name,
                                        mac: dev.mac,
                                    });
                                });
                            }
                        })
                        .child(setting_row(display.clone(), None::<String>, false, rect().into_element()))
                        .into_element(),
                );
            }
            if bt_list.is_empty() {
                rows.push(tile_sub("No devices found — scanning...").into_element());
            }
            let list_h = row1_list_h;
            card = card.child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::px(list_h))
                    .show_scrollbar(rows.len() > 4)
                    .child(rect().width(Size::fill()).vertical().spacing(2.).content(Content::Flex).children(rows)),
            );
        }
        card
    };
    let wired_card = tile()
        .height(Size::px(row2_h))
        .child(tile_head(Some(WIRED), "Wired", None::<Element>)).child(if let Some(ref info) = wired_info {
            if info.ip_address.is_empty() {
                dot_status_row(
                    info.interface.clone(),
                    None::<String>,
                    true,
                    label().font_size(12.).color(t.accent).text("●"),
                )
                .into_element()
            } else {
                dot_status_row(
                    info.interface.clone(),
                    Some(info.ip_address.clone()),
                    true,
                    label().font_size(12.).color(t.accent).text("●"),
                )
                .into_element()
            }
        } else {
            dot_status_row(
                "No connection".to_string(),
                None::<String>,
                false,
                label().font_size(12.).color(t.text_dim).text("○"),
            )
            .into_element()
        });
    let focus_card = {
        let sched = dnd_sched.read().clone();
        let sched_meta = format!("{}–{}", sched.start, sched.end);
        let sched_on = sched.enabled;
        tile().height(Size::px(row2_h)).child(tile_head(
            Some(FOCUS),
            "Focus",
            Some({
                let mut f = dnd;
                pill_switch(*f.read(), move |v| {
                    f.set_if_modified(v);
                    apply(SettingKey::DoNotDisturb, if v { "on".to_string().into() } else { "off".to_string().into() });
                })
            }),
        )).child(setting_row(
            format!("Schedule · {sched_meta}"),
            Some(if crate::state::dnd_schedule_active(&sched, &crate::state::now_hm_local()) {
                "In quiet hours now"
            } else {
                "Silences banners on schedule"
            }),
            false,
            {
                let mut ds = dnd_sched;
                pill_switch(sched_on, move |v| {
                    let mut s = ds.read().clone();
                    s.enabled = v;
                    crate::state::save_dnd_schedule(&s);
                    ds.set(s);
                })
            },
        ))
    };
    let sound_card = tile().child(tile_head(Some(SOUND), "Sound", None::<Element>)).child({
        let mut v = volume;
        slider_row(Some(SOUND), *v.read(), move |nv| {
            v.set_if_modified(nv);
            apply(SettingKey::AudioVolume, nv.into());
            let pct = nv as i32;
            std::thread::spawn(move || {
                HyprlandBackend.set_volume(pct);
            });
        })
    });
    let brightness_card = tile().child(tile_head(Some(DISPLAY), "Brightness", None::<Element>)).child({
        let mut b = brightness;
        slider_row(Some(SUN), *b.read(), move |nv| {
            b.set_if_modified(nv);
            apply(SettingKey::DisplayBrightness, nv.into());
            let pct = nv as u32;
            std::thread::spawn(move || {
                HyprlandBackend.set_brightness(pct);
            });
        })
    });
    let battery_card = tile().child(tile_head(Some(BATTERY), "Power", None::<Element>)).child(
        rect()
            .width(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(14.)
            .content(Content::Flex)
            .child(battery_ring(bat_pct, false))
            .child(
                rect()
                    .vertical()
                    .spacing(2.)
                    .child(
                        label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(format!("{bat_pct}%")),
                    )
                    .child(label().font_size(12.).color(t.text_dim).text(if bat_status.is_empty() {
                        "Unknown".to_string()
                    } else {
                        bat_status
                    })),
            ),
    );
    let airplane_on = *airplane.read();
    let airplane_card = {
        let mut card = tile().child(tile_head(
            Some(AIRPLANE),
            "Airplane",
            Some({
                let mut ap = airplane;
                let mut err = last_error;
                pill_switch(airplane_on, move |v| {
                    let prev = *ap.read();
                    ap.set_if_modified(v);
                    err.set(None);
                    let mut ap_rb = ap;
                    let mut err_rb = err;
                    spawn(async move {
                        let res = tokio::task::spawn_blocking(move || {
                            set_and_apply(crate::state::socket(), SettingKey::AirplaneMode, SettingValue::Bool(v))
                        })
                        .await;
                        match res {
                            Ok(Ok(Ok(()))) => {}
                            Ok(Ok(Err(denial))) => {
                                ap_rb.set(prev);
                                err_rb.set(Some(format!("Airplane mode denied: {denial}")));
                            }
                            Ok(Err(e)) => {
                                ap_rb.set(prev);
                                err_rb.set(Some(format!("Couldn't reach settings daemon: {e}")));
                            }
                            Err(e) => {
                                ap_rb.set(prev);
                                err_rb.set(Some(format!("Couldn't apply airplane mode: {e}")));
                            }
                        }
                    });
                })
            }),
        ));
        card = card.child(setting_row(
            if airplane_on { "Radios off".to_string() } else { "Radios on".to_string() },
            Some(if airplane_on { "Wi-Fi and Bluetooth paused" } else { "Tap to pause all radios" }),
            false,
            status_chip(if airplane_on { "On" } else { "Off" }, airplane_on, None::<EventHandler<()>>),
        ));
        card
    };
    let night_on = *night.read();
    let night_card = {
        let mut card = tile().child(tile_head(
            Some(MOON),
            "Night light",
            Some({
                let mut n = night;
                let mut err = last_error;
                pill_switch(night_on, move |v| {
                    let prev = *n.read();
                    n.set_if_modified(v);
                    err.set(None);
                    let mut n_rb = n;
                    let mut err_rb = err;
                    spawn(async move {
                        let res = tokio::task::spawn_blocking(move || {
                            set_and_apply(crate::state::socket(), SettingKey::DisplayNightShift, SettingValue::Bool(v))
                        })
                        .await;
                        match res {
                            Ok(Ok(Ok(()))) => {}
                            Ok(Ok(Err(denial))) => {
                                n_rb.set(prev);
                                err_rb.set(Some(format!("Night light denied: {denial}")));
                            }
                            Ok(Err(e)) => {
                                n_rb.set(prev);
                                err_rb.set(Some(format!("Couldn't reach settings daemon: {e}")));
                            }
                            Err(e) => {
                                n_rb.set(prev);
                                err_rb.set(Some(format!("Couldn't apply night light: {e}")));
                            }
                        }
                    });
                })
            }),
        ));
        if night_on {
            card = card.child(tile_sub("Warmer display in the evening"));
        } else {
            card = card.child(tile_sub("Off"));
        }
        card
    };
    let current_profile = power_profile.read().clone();
    let is_power_supported = *power_supported.read();
    let power_card = {
        let mut card = tile().child(tile_head(Some(POWER), "Power mode", None::<Element>));
        if !is_power_supported {
            card = card.child(tile_sub("Unavailable: powerprofilesctl not found"));
        } else {
            let selected = current_profile.clone();
            card = card.child(segmented_control(
                vec![
                    ("Performance", "performance".to_string()),
                    ("Balanced", "balanced".to_string()),
                    ("Saver", "power-saver".to_string()),
                ],
                selected.clone(),
                EventHandler::new({
                    let mut pp = power_profile;
                    let mut err = last_error;
                    let prev = selected.clone();
                    move |v: String| {
                        pp.set_if_modified(v.clone());
                        err.set(None);
                        let mut pp_rb = pp;
                        let mut err_rb = err;
                        let prev_c = prev.clone();
                        spawn(async move {
                            let res = tokio::task::spawn_blocking(move || {
                                set_and_apply(
                                    crate::state::socket(),
                                    SettingKey::PowerProfile,
                                    SettingValue::String(v),
                                )
                            })
                            .await;
                            match res {
                                Ok(Ok(Ok(()))) => {}
                                Ok(Ok(Err(denial))) => {
                                    pp_rb.set(prev_c);
                                    err_rb.set(Some(format!("Power profile denied: {denial}")));
                                }
                                Ok(Err(e)) => {
                                    pp_rb.set(prev_c);
                                    err_rb.set(Some(format!("Couldn't reach settings daemon: {e}")));
                                }
                                Err(e) => {
                                    pp_rb.set(prev_c);
                                    err_rb.set(Some(format!("Couldn't apply power profile: {e}")));
                                }
                            }
                        });
                    }
                }),
            ));
        }
        card
    };
    let kbd_opt = *kbd_level.read();
    let is_kbd_supported = *kbd_supported.read();
    let kbd_card = {
        let mut card = tile().child(tile_head(Some(KEYBOARD), "Keyboard light", None::<Element>));
        if !is_kbd_supported || kbd_opt.is_none() {
            card = card.child(tile_sub("Unavailable: no keyboard backlight found"));
        } else {
            let lvl = kbd_opt.unwrap_or(0) as f64;
            card = card.child({
                let mut kl = kbd_level;
                let mut err = last_error;
                slider_row(Some(KEYBOARD), lvl, move |nv| {
                    let v = nv.round().clamp(0.0, 100.0) as u32;
                    kl.set_if_modified(Some(v));
                    err.set(None);
                    let mut err_rb = err;
                    spawn(async move {
                        let r = tokio::task::spawn_blocking(move || system::apply_keyboard_backlight(v))
                            .await
                            .unwrap_or(Err("task failed".to_string()));
                        if let Err(e) = r {
                            err_rb.set(Some(format!("Keyboard light failed: {e}")));
                        }
                    });
                })
            });
            card = card.child(setting_row(
                format!("{}%", kbd_opt.unwrap_or(0)),
                None::<String>,
                false,
                status_chip("On", kbd_opt.unwrap_or(0) > 0, None::<EventHandler<()>>),
            ));
        }
        card
    };
    let bg_count = *bg_apps.read();
    let bg_card = tile()
        .child(tile_head(Some(APPS), "Background apps", None::<Element>))
        .child(setting_row(
            format!("{bg_count} running"),
            None::<String>,
            false,
            status_chip(if bg_count > 0 { "Active" } else { "Idle" }, bg_count > 0, None::<EventHandler<()>>),
        ));
    // Two-step arm/confirm for destructive power actions. First press arms,
    // second press executes. Armed state auto-reverts after a timeout.
    let arm_power = move |key: &'static str, action: &'static str| {
        let mut pp = pending_power;
        if *pp.read() == Some(key) {
            pp.set(None);
            power_action(action);
        } else {
            pp.set(Some(key));
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                if *pp.read() == Some(key) {
                    pp.set(None);
                }
            });
        }
    };
    let armed: Option<&'static str> = *pending_power.read();
    let lock_btn = action_tile_flex_column(LOCK, "Lock", ActionTileTone::Default, move || system::trigger_lock());
    let logout_armed = armed == Some("logout");
    let logout_btn = action_tile_flex_column(
        ARROW_RIGHT,
        if logout_armed { "Sure?" } else { "Log out" },
        if logout_armed { ActionTileTone::Danger } else { ActionTileTone::Default },
        move || arm_power("logout", "logout"),
    );
    let reboot_armed = armed == Some("reboot");
    let reboot_btn = action_tile_flex_column(
        REFRESH_CW,
        if reboot_armed { "Sure?" } else { "Restart" },
        if reboot_armed { ActionTileTone::Danger } else { ActionTileTone::Default },
        move || arm_power("reboot", "reboot"),
    );
    let shutdown_btn =
        action_tile_flex_column(POWER, "Shut down", ActionTileTone::Danger, move || arm_power("shutdown", "shutdown"));
    let actions_row = rect()
        .width(Size::fill())
        .horizontal()
        .spacing(GAP)
        .content(Content::Flex)
        .child(lock_btn)
        .child(logout_btn)
        .child(reboot_btn)
        .child(shutdown_btn);
    let confirm_title: Option<(&'static str, &'static str)> = match armed {
        Some("logout") => Some(("Log out?", "End the current session.")),
        Some("reboot") => Some(("Restart?", "Unsaved work may be lost.")),
        Some("shutdown") => Some(("Shut down?", "Unsaved work may be lost.")),
        _ => None,
    };
    let confirm_card: Option<Element> = confirm_title.map(|(title, subtitle)| {
        let mut pp_cancel = pending_power;
        let key: &'static str = armed.unwrap_or("logout");
        let action: &'static str = match key {
            "reboot" => "reboot",
            "shutdown" => "shutdown",
            _ => "logout",
        };
        tile()
            .spacing(8.)
            .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(title))
            .child(label().font_size(11.).color(t.text_dim).text(format!("{subtitle} Press confirm again or use the button below. Auto-cancels in a few seconds.")))
            .child(
                rect().width(Size::fill()).horizontal().spacing(GAP).content(Content::Flex).child(
                    rect().width(Size::flex(1.)).child(secondary_button_full("Cancel", move || pp_cancel.set(None))),
                ).child(
                    rect().width(Size::flex(1.)).child(danger_button_full("Confirm", move || {
                        let mut pp = pending_power;
                        pp.set(None);
                        power_action(action);
                    })),
                ),
            )
            .into_element()
    });
    let header = rect()
        .width(Size::fill())
        .horizontal()
        .main_align(Alignment::SpaceBetween)
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .content(Content::Flex)
                .child(tool_button(GENERAL, t.text_dim, open_in_settings))
                .child(label().font_size(12.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text("Overlay")),
        )
        .child(small_close_button(move || dismiss.set(true)));
    let stagger_total = 7;
    let stagger_last = stagger_total - 1;
    let stagger_dr = dismiss.into_readable();
    let error_notice: Option<Element> = last_error.read().clone().map(|e| {
        notice_pill(e, NoticeKind::Error).into_element()
    });
    let panel_inner_content = rect()
        .width(Size::fill())
        .vertical()
        .spacing(GAP)
        .content(Content::Flex)
        .child(header)
        .child(
            StaggerRow::new(0, stagger_total, stagger_dr.clone()).child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).vertical().spacing(GAP).child(wifi_card).child(wired_card))
                .child(rect().width(Size::flex(1.)).vertical().spacing(GAP).child(bt_card).child(focus_card)),
        ))
        .child(
            StaggerRow::new(1, stagger_total, stagger_dr.clone()).child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).child(sound_card))
                .maybe_child(
                    system::HyprlandBackend
                        .supports_brightness()
                        .then(|| rect().width(Size::flex(1.)).child(brightness_card).into_element()),
                ),
        ))
        .child(
            StaggerRow::new(2, stagger_total, stagger_dr.clone()).child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).child(airplane_card))
                .child(rect().width(Size::flex(1.)).child(night_card)),
        ))
        .child(StaggerRow::new(3, stagger_total, stagger_dr.clone()).child(power_card))
        .child(
            StaggerRow::new(4, stagger_total, stagger_dr.clone()).child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).child(kbd_card))
                .child(rect().width(Size::flex(1.)).child(bg_card)),
        ))
        .maybe(has_battery, |el| el.child(StaggerRow::new(5, stagger_total, stagger_dr.clone()).child(battery_card)))
        .maybe_child(error_notice)
        .maybe_child(confirm_card)
        .child(StaggerRow::new(stagger_last, stagger_total, stagger_dr.clone()).child(actions_row));
    let panel_inner = rect()
        .width(Size::fill())
        .vertical()
        .padding(10.)
        .background(t.panel)
        .content(Content::Flex)
        .corner_radius(20.)
        .border(Border::new().width(1.).fill(t.border))
        .child(ScrollView::new().width(Size::fill()).height(Size::px(640.)).show_scrollbar(true).child(panel_inner_content));
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(6.)
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(
            PanelShell::new(dismiss.into_readable())
                .on_dismiss(EventHandler::new(move |()| Platform::get().close_window(wid)))
                .child(panel_inner),
        )
        .into_element()
}
