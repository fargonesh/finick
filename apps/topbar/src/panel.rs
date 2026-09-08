use crate::state::{
    apply, clear_notifications, fetch_notifications_blocking, load_initial_batch, parse_capacity_pct, power_action,
    subscribe_live,
};
use {
    freya::prelude::*,
    ipsea::settings::SettingKey,
    system::{HyprlandBackend, SystemBackend, WiredInfo},
    ui::*,
};

pub fn control_panel_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();

    // Shared settings states
    let wifi = use_state(|| true);
    let bt = use_state(|| true);
    let volume = use_state(|| 65.0);
    let muted = use_state(|| false);
    let brightness = use_state(|| 72.0);
    let dnd = use_state(|| false);
    let connected = use_state(|| false);

    // Wired + battery + notifications — own state for panel
    let wired = use_state(|| Option::<WiredInfo>::None);
    let wired_initial = use_hook(|| HyprlandBackend.get_wired_info());
    {
        let mut wired_state = wired;
        let init = wired_initial;
        if init != *wired_state.read() {
            wired_state.set(init);
        }
    }

    let battery_pct = use_state(|| 0u8);
    let battery_status = use_state(|| "Unknown".to_string());
    let battery_initial = use_hook(|| HyprlandBackend.get_power_info());
    {
        let mut pct = battery_pct;
        let mut st = battery_status;
        let info = battery_initial.clone();
        let p = parse_capacity_pct(&info.capacity);
        pct.set_if_modified(p);
        st.set_if_modified(info.status.clone());
    }

    let notifications = use_state(Vec::<crate::state::Notification>::new);
    let notif_initial = use_hook(fetch_notifications_blocking);
    {
        let mut n = notifications;
        n.set_if_modified(notif_initial.clone());
    }

    use_hook(move || {
        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected);
        subscribe_live(wifi, bt, volume, muted, brightness, dnd, connected);
        let mut wired_state = wired;
        let mut pct_state = battery_pct;
        let mut status_state = battery_status;
        let mut notif_state = notifications;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
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
                let notifs = tokio::task::spawn_blocking(fetch_notifications_blocking).await.unwrap_or_default();
                notif_state.set_if_modified(notifs);
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
                        if s.contains("\"class\": \"topbar-panel\"") || s.contains("\"class\": \"topbar\"") {
                            return false;
                        }
                        return true;
                    }
                    false
                })
                .await
                .unwrap_or(false);
                if should_close {
                    if let Some(ctx) = GlobalContexts::get().try_get_context::<crate::PanelWindowId>() {
                        if let Ok(mut g) = ctx.0.lock() {
                            *g = None;
                        }
                    }
                    Platform::get().close_window(panel_id);
                    break;
                }
            }
        });
    });

    let _wifi_on = *wifi.read();
    let _bt_on = *bt.read();
    let _is_connected = *connected.read();
    let wired_info = wired.read().clone();
    let bat_pct = *battery_pct.read();
    let bat_status = battery_status.read().clone();
    let notifs = notifications.read().clone();
    let notif_count = notifs.len();
    let power_snapshot = HyprlandBackend.get_power_info();
    let has_battery = power_snapshot.capacity != "Unknown" && power_snapshot.status != "Unknown";

    let wifi_card = tile().child(tile_head(
        Some(WIFI),
        "Wi-Fi",
        Some({
            let mut w = wifi;
            pill_switch(*w.read(), move |v| {
                w.set_if_modified(v);
                apply(SettingKey::WifiEnabled, v.into());
            })
        }),
    ));

    let bt_card = tile().child(tile_head(
        Some(BLUETOOTH),
        "Bluetooth",
        Some({
            let mut b = bt;
            pill_switch(*b.read(), move |v| {
                b.set_if_modified(v);
                apply(SettingKey::BluetoothEnabled, v.into());
            })
        }),
    ));

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
        .child(tile_head(
            Some(SOUND),
            "Sound",
            Some({
                let mut m = muted;
                pill_switch(*m.read(), move |v| {
                    m.set_if_modified(v);
                    apply(SettingKey::AudioMuted, v.into());
                })
            }),
        ))
        .child({
            let mut v = volume;
            slider_row(Some(SOUND), *v.read(), move |nv| {
                v.set_if_modified(nv);
                apply(SettingKey::AudioVolume, nv.into());
            })
        });

    let brightness_card = tile().child(tile_head(Some(DISPLAY), "Brightness", None::<Element>)).child({
        let mut b = brightness;
        slider_row(Some(SUN), *b.read(), move |nv| {
            b.set_if_modified(nv);
            apply(SettingKey::DisplayBrightness, nv.into());
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
                    .child(label().font_size(12.).color(t.text_dim).text(bat_status))
                    .maybe_child(
                        power_snapshot
                            .health_percent
                            .map(|h| label().font_size(11.).color(t.text_dim).text(format!("Health {h}%"))),
                    )
                    .maybe_child(
                        power_snapshot
                            .cycle_count
                            .map(|c| label().font_size(11.).color(t.text_dim).text(format!("{c} cycles"))),
                    ),
            ),
    );

    let notifications_card = tile()
        .child(tile_head(
            Some(NOTIFICATIONS),
            format!("Notifications{}", if notif_count > 0 { format!(" · {notif_count}") } else { String::new() }),
            Some(
                rect()
                    .cursor(CursorIcon::Pointer)
                    .on_press(move |_| {
                        clear_notifications();
                    })
                    .child(secondary_button("Clear", clear_notifications)),
            ),
        ))
        .maybe(notifs.is_empty(), |el| el.child(tile_sub("No notifications")))
        .maybe(!notifs.is_empty(), |el| {
            let mut e = el;
            for n in notifs.iter().take(3) {
                let title = n.title.clone();
                let body = n.body.clone();
                let lvl = format!("{:?}", n.level);
                e = e.child(setting_row(title, Some(body), true, label().font_size(10.).color(t.text_dim).text(lvl)));
            }
            e
        });

    let power_btn = |label_text: &'static str, svg: &'static str, bg: Color, fg: Color, action: fn()| {
        let txt = label_text.to_string();
        rect()
            .width(Size::flex(1.))
            .height(Size::px(68.))
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
        .child(power_btn("Log out", ARROW_RIGHT, t.panel_raised, t.text, || power_action("logout")))
        .child(power_btn("Restart", REFRESH_CW, t.panel_raised, t.text, || power_action("reboot")))
        .child(power_btn("Shut down", POWER_ICON, t.accent_red, Color::WHITE, || power_action("shutdown")));

    let header = rect()
        .width(Size::fill())
        .horizontal()
        .main_align(Alignment::SpaceBetween)
        .cross_align(Alignment::Center)
        .child(label().font_size(15.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text("Control Panel"))
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

    let panel_inner = rect()
        .width(Size::fill())
        .vertical()
        .spacing(GAP)
        .padding(12.)
        .background(t.panel)
        .content(Content::Flex)
        .corner_radius(20.)
        .border(Border::new().width(1.).fill(t.border))
        .child(header)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).vertical().spacing(GAP).child(wifi_card).child(wired_card))
                .child(rect().width(Size::flex(1.)).vertical().spacing(GAP).child(bt_card).child(focus_card)),
        )
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .content(Content::Flex)
                .child(rect().width(Size::flex(1.)).child(sound_card))
                .child(rect().width(Size::flex(1.)).child(brightness_card)),
        )
        .maybe(has_battery, |el| el.child(battery_card))
        .child(notifications_card)
        .child(actions_row);

    rect()
        .width(Size::px(400.))
        .height(Size::px(640.))
        .padding(8.)
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(panel_inner)
        .into_element()
}
