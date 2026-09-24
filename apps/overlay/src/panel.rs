use crate::state::{
        apply, load_initial_batch, parse_capacity_pct, power_action, subscribe_live,
    };
use {
    freya::prelude::*,
    ipsea::settings::SettingKey,
    system::{BluetoothDevice, CurrentWifiInfo, HyprlandBackend, SystemBackend, WifiNetworkDetail, WiredInfo},
    ui::*,
};
fn trunc(s: &str, n: usize) -> String { if s.chars().count() <= n { s.to_string() } else { let mut t: String = s.chars().take(n-1).collect(); t.push('…'); t } }

pub fn compute_panel_height(
    wifi_on: bool,
    has_connected_wifi: bool,
    available_wifi_count: usize,
    has_wired: bool,
    bt_on: bool,
    connected_bt_count: usize,
    other_bt_count: usize,
    has_battery: bool,
) -> i32 {
    let base = 12 + 20 + 28; // window padding + panel inner padding + header

    // Left column: Wi-Fi (+ Wired)
    let wifi_card_h = if !wifi_on {
        76
    } else {
        let conn_h = if has_connected_wifi { 28 } else { 0 };
        let shown_networks = available_wifi_count.min(5);
        let net_h = if shown_networks == 0 && !has_connected_wifi {
            22
        } else {
            shown_networks * 28
        };
        66 + conn_h + net_h
    };
    let wired_card_h = if has_wired { 10 + 64 } else { 0 };
    let left_col_h = wifi_card_h + wired_card_h;

    // Right column: Bluetooth + Focus
    let bt_card_h = if !bt_on {
        76
    } else {
        let conn_h = if connected_bt_count > 0 {
            connected_bt_count.min(3) * 36
        } else {
            22
        };
        let other_h = other_bt_count.min(4) * 28;
        let empty_h = if connected_bt_count == 0 && other_bt_count == 0 { 22 } else { 0 };
        66 + conn_h + other_h + empty_h
    };
    let focus_card_h = 10 + 64;
    let right_col_h = bt_card_h + focus_card_h;

    let cards_row_h = left_col_h.max(right_col_h);

    // Sliders
    let sliders_h = 76;

    // Battery
    let battery_h = if has_battery { 10 + 60 } else { 0 };

    // Actions row
    let actions_h = 68;

    let _ = (cards_row_h, sliders_h, battery_h);
    700
}

fn resize_panel_window(target_h: i32) {
    std::thread::spawn(move || {
        let lua = format!(
            r#"
            for _, w in ipairs(hl.get_windows()) do
                if w.title == 'control-panel' or w.class == 'overlay-panel' or w.class == 'topbar-panel' then
                    local addr = 'address:' .. tostring(w.address)
                    hl.dispatch(hl.dsp.window.resize({{ window = addr, x = 460, y = {target_h}, relative = false }}))
                    break
                end
            end
            "#
        );
        let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
    });
}
fn should_resize(last: i32, target: i32) -> bool {
    (last - target).abs() >= 12
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
    let wifi_modal_target: State<Option<WifiNetworkDetail>> = use_state(|| None);
    let wifi_password: State<String> = use_state(String::new);
    let wifi_connecting: State<bool> = use_state(|| false);
    let wifi_error: State<Option<String>> = use_state(|| None);

    let bt_modal_target: State<Option<BluetoothDevice>> = use_state(|| None);
    let bt_pin: State<String> = use_state(String::new);
    let bt_pairing: State<bool> = use_state(|| false);
    let bt_error: State<Option<String>> = use_state(|| None);
    use_hook(move || {
        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
        subscribe_live(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
        let mut wired_state = wired;
        let mut pct_state = battery_pct;
        let mut status_state = battery_status;
        let mut wifi_state = wifi_details;
        let mut bt_state = bt_devices;

        // Fetch initial data asynchronously without blocking window creation

        spawn(async move {
            let initial_wifi = tokio::task::spawn_blocking(|| HyprlandBackend.get_wifi_details()).await.unwrap_or_default();
            wifi_state.set_if_modified(initial_wifi);
        });

        spawn(async move {
            let initial_bt = tokio::task::spawn_blocking(|| HyprlandBackend.get_paired_bluetooth_devices()).await.unwrap_or_default();
            bt_state.set_if_modified(initial_bt);
        });

        spawn(async move {
            let wired_now = tokio::task::spawn_blocking(|| HyprlandBackend.get_wired_info()).await.unwrap_or(None);
            wired_state.set_if_modified(wired_now);
            let power = tokio::task::spawn_blocking(|| HyprlandBackend.get_power_info()).await.unwrap_or(system::PowerInfo {
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
            let _ = std::process::Command::new("bluetoothctl").args(["scan", "on"]).output();
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                let wired_now =
                    tokio::task::spawn_blocking(|| HyprlandBackend.get_wired_info()).await.unwrap_or(None);
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
                let b = tokio::task::spawn_blocking(|| HyprlandBackend.get_paired_bluetooth_devices()).await.unwrap_or_default();
                bt_state.set_if_modified(b);
            }
        });
    });
    use_hook(|| {
        let panel_id = Platform::window_id();
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let should_close = tokio::task::spawn_blocking(|| {
                    let out = std::process::Command::new("hyprctl").args(["activewindow", "-j"]).output();
                    if let Ok(o) = out {
                        let s = String::from_utf8_lossy(&o.stdout).to_string();
                        if s.trim().is_empty() || s.contains("\"class\": \"\"") {
                            return false;
                        }
                        if s.contains("\"class\": \"overlay-panel\"") || s.contains("\"class\": \"overlay\"") || s.contains("\"class\": \"topbar-panel\"") || s.contains("\"class\": \"topbar\"") {
                            return false;
                        }
                        return true;
                    }
                    false
                })
                .await
                .unwrap_or(false);
                if should_close {
                    Platform::get().close_window(panel_id);
                    break;
                }
            }
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
                        std::thread::spawn(|| { let _ = std::process::Command::new("nmcli").args(["radio","wifi","on"]).output(); });
                    } else {
                        std::thread::spawn(|| { let _ = std::process::Command::new("nmcli").args(["radio","wifi","off"]).output(); });
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
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .main_align(Alignment::SpaceBetween)
                            .content(Content::Flex)
                            .padding((6., 0.))
                            .cursor(CursorIcon::Pointer)
                            .on_press(move |_| {
                                let iface = iface.clone();
                                std::thread::spawn(move || {
                                    if let Some(iface) = iface { let _ = std::process::Command::new("nmcli").args(["dev","disconnect", &iface]).output(); } else { HyprlandBackend.disconnect_wifi(); }
                                });
                            })
                            .child(
                                rect().horizontal().spacing(8.).cross_align(Alignment::Center).content(Content::Flex)
                                    .child(label().font_size(12.).color(t.accent).text("●"))
                                    .child(label().font_size(12.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(display))
                            )
                            .child(label().font_size(11.).color(t.text_dim).text("Connected"))
                    );
                }
            }
            let mut rows: Vec<Element> = Vec::new();
            let iface_clone = current_wifi.interface.clone();
            let mut shown = 0;
            for net in available_networks.iter() {
                if shown >= 8 { break; }
                if current_wifi.ssid.as_ref() == Some(&net.ssid) && net.is_connected { continue; }
                let ssid = net.ssid.clone();
                let ssid2 = ssid.clone();
                let display = trunc(&ssid, 20);
                let is_connected = current_wifi.ssid.as_ref() == Some(&ssid);
                let iface_for_row = iface_clone.clone();
                let is_secured = !net.is_saved && !net.security.is_empty() && net.security != "--" && !net.security.eq_ignore_ascii_case("Open");
                let net_for_click = net.clone();
                rows.push(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::SpaceBetween)
                        .content(Content::Flex)
                        .padding((6., 2.))
                        .corner_radius(8.)
                        .cursor(CursorIcon::Pointer)
                        .background(Color::TRANSPARENT)
                        .on_press({
                            move |_| {
                                if is_connected {
                                    let iface = iface_for_row.clone();
                                    std::thread::spawn(move || {
                                        if let Some(iface) = iface {
                                            let _ = std::process::Command::new("nmcli").args(["dev", "disconnect", &iface]).output();
                                        } else {
                                            HyprlandBackend.disconnect_wifi();
                                        }
                                    });
                                } else if is_secured {
                                    let net = net_for_click.clone();
                                    std::thread::spawn(move || {
                                        let _ = ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::WifiPassword {
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
                        .child(label().font_size(12.).color(t.text).text(display))
                        .maybe(is_connected, |el| el.child(label().font_size(11.).color(t.accent).text("Connected")))
                        .into_element()
                );
                shown += 1;
            }
            if rows.is_empty() && current_wifi.ssid.is_none() {
                rows.push(tile_sub("No networks found").into_element());
            }
            let list_h = (rows.len().min(5) as f32 * 30.0 + 8.0).clamp(40.0, 120.0);
            card = card.child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::px(list_h))
                    .show_scrollbar(rows.len() > 4)
                    .child(
                        rect().width(Size::fill()).vertical().spacing(2.).content(Content::Flex).children(rows)
                    )
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
                        let _ = std::process::Command::new("rfkill").args([if en {"unblock"} else {"block"}, "bluetooth"]).output();
                        if en { let _ = std::process::Command::new("bluetoothctl").args(["scan","on"]).output(); }
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
                        rect().width(Size::fill()).horizontal().cross_align(Alignment::Center).main_align(Alignment::SpaceBetween).content(Content::Flex).padding((6.,2.)).corner_radius(8.).cursor(CursorIcon::Pointer)
                            .on_press(move |_| {
                                let m = mac.clone();
                                std::thread::spawn(move || { HyprlandBackend.disconnect_bluetooth_device(&m); });
                            })
                            .child(
                                rect().vertical().spacing(3.).content(Content::Flex)
                                    .child(rect().horizontal().spacing(6.).cross_align(Alignment::Center).content(Content::Flex)
                                        .child(label().font_size(11.).color(t.accent).text("●"))
                                        .child(label().font_size(12.).color(t.text).text(display.clone())))
                                    .child(label().font_size(10.).color(t.accent).text("Connected"))
                            )
                            .child(icon(CHEVRON_RIGHT, 12., t.text_dim))
                            .into_element()
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
                    rect().width(Size::fill()).horizontal().cross_align(Alignment::Center).main_align(Alignment::SpaceBetween).content(Content::Flex).padding((6.,2.)).corner_radius(8.).cursor(CursorIcon::Pointer)
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
                        .child(label().font_size(12.).color(t.text).text(display.clone()))
                        .into_element()
                );
            }
            if bt_list.is_empty() {
                rows.push(tile_sub("No devices found — scanning...").into_element());
            }
            let list_h = (rows.len().min(5) as f32 * 32.0 + 8.0).clamp(40.0, 120.0);
            card = card.child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::px(list_h))
                    .show_scrollbar(rows.len() > 4)
                    .child(rect().width(Size::fill()).vertical().spacing(2.).content(Content::Flex).children(rows))
            );
        }
        card
    };
    let wired_card =
        tile().child(tile_head(Some(WIRED), "Wired", None::<Element>)).child(if let Some(ref info) = wired_info {
            if info.ip_address.is_empty() {
                setting_row(info.interface.clone(), None::<String>, false, label().font_size(12.).color(t.accent).text("●"))
                    .into_element()
            } else {
                setting_row(
                    info.interface.clone(),
                    Some(info.ip_address.clone()),
                    false,
                    label().font_size(12.).color(t.accent).text("●"),
                )
                .into_element()
            }
        } else {
            setting_row(
                "No connection".to_string(),
                None::<String>,
                false,
                label().font_size(12.).color(t.text_dim).text("○"),
            )
            .into_element()
        });
    let focus_card = tile().child(tile_head(
        Some(FOCUS),
        "Focus",
        Some({
            let mut f = dnd;
            pill_switch(*f.read(), move |v| {
                f.set_if_modified(v);
                apply(SettingKey::DoNotDisturb, if v { "on".to_string().into() } else { "off".to_string().into() });
            })
        }),
    ));
    let sound_card = tile()
        .child(tile_head(Some(SOUND), "Sound", None::<Element>))
        .child({
            let mut v = volume;
            slider_row(Some(SOUND), *v.read(), move |nv| {
                v.set_if_modified(nv);
                apply(SettingKey::AudioVolume, nv.into());
                let pct = nv as i32;
                std::thread::spawn(move || { HyprlandBackend.set_volume(pct); });
            })
        });
    let brightness_card = tile().child(tile_head(Some(DISPLAY), "Brightness", None::<Element>)).child({
        let mut b = brightness;
        slider_row(Some(SUN), *b.read(), move |nv| {
            b.set_if_modified(nv);
            apply(SettingKey::DisplayBrightness, nv.into());
            let pct = nv as u32;
            std::thread::spawn(move || { HyprlandBackend.set_brightness(pct); });
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
                    .child(label().font_size(12.).color(t.text_dim).text(if bat_status.is_empty() { "Unknown".to_string() } else { bat_status })),
            ),
    );
    let power_btn = |label_text: &'static str, svg: &'static str, bg: Color, fg: Color, action: fn()| {
        let txt = label_text.to_string();
        rect()
            .width(Size::flex(1.))
            .height(Size::px(60.))
            .corner_radius(14.)
            .background(bg)
            .border(Border::new().width(1.).fill(if bg == t.accent_red { t.accent_red } else { t.border }))
            .center()
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| action())
            .child(
                rect()
                    .vertical()
                    .center()
                    .spacing(5.)
                    .child(icon(svg, 18., fg))
                    .child(label().font_size(11.).font_weight(FontWeight::SEMI_BOLD).color(fg).text(txt.clone())),
            )
    };
    const POWER_ICON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v10"/><path d="M18.36 6.64A9 9 0 1 1 5.64 6.64"/></svg>"#;
    let actions_row = rect()
        .width(Size::fill())
        .horizontal()
        .spacing(10.)
        .content(Content::Flex)
        .child(power_btn("Lock", LOCK, t.panel_raised, t.text, || system::trigger_lock()))
        .child(power_btn("Log out", ARROW_RIGHT, t.panel_raised, t.text, || power_action("logout")))
        .child(power_btn("Restart", REFRESH_CW, t.panel_raised, t.text, || power_action("reboot")))
        .child(power_btn("Shut down", POWER_ICON, t.accent_red, Color::WHITE, || power_action("shutdown")));
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
                .child(
                    rect()
                        .width(Size::px(28.))
                        .height(Size::px(28.))
                        .corner_radius(999.)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border))
                        .center()
                        .cursor(CursorIcon::Pointer)
                        .on_press(|_| {
                            std::thread::spawn(|| {
                                let _ = std::process::Command::new("finick-settings").spawn()
                                    .or_else(|_| std::process::Command::new("hyprctl")
                                        .args(["dispatch", "exec", "finick-settings"])
                                        .spawn());
                            });
                        })
                        .child(icon(GENERAL, 14., t.text_dim)),
                )
                .child(label().font_size(12.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text("Overlay")),
        )
        .child(
            rect()
                .width(Size::px(28.))
                .height(Size::px(28.))
                .corner_radius(999.)
                .background(t.panel_raised)
                .border(Border::new().width(1.).fill(t.border))
                .center()
                .cursor(CursorIcon::Pointer)
                .on_press(|_| Platform::get().close_window(Platform::window_id()))
                .child(label().font_size(13.).color(t.text_dim).text("✕")),
        );
    let panel_inner_content = rect()
        .width(Size::fill())
        .vertical()
        .spacing(10.)
        .content(Content::Flex)
        .child(header)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(10.)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).vertical().spacing(10.).child(wifi_card).child(wired_card))
                .child(rect().width(Size::flex(1.)).vertical().spacing(10.).child(bt_card).child(focus_card)),
        )
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(10.)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).child(sound_card))
                .maybe_child(system::HyprlandBackend.supports_brightness().then(|| {
                    rect().width(Size::flex(1.)).child(brightness_card).into_element()
                })),
        )
        .maybe(has_battery, |el| el.child(battery_card))
        .child(actions_row);
    let panel_inner = rect()
        .width(Size::fill())
        .vertical()
        .padding(10.)
        .background(t.panel)
        .content(Content::Flex)
        .corner_radius(20.)
        .border(Border::new().width(1.).fill(t.border))
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .show_scrollbar(false)
                .child(panel_inner_content)
        );
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(6.)
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(panel_inner)
        .into_element()
}
