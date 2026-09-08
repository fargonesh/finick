#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

#[allow(unused_imports)]
use ipsea::settings::get_setting;
use {
    freya::prelude::*,
    ipsea::settings::{
        SETTINGS_SOCKET_NAME, SettingEntry, SettingKey, SettingValue, SettingsEvent, SubscriptionFilter, get_all_settings,
        set_and_apply,
    },
    serde::{Deserialize, Serialize},
    system::{HyprlandBackend, SystemBackend, WiredInfo},
    ui::*,
};

fn socket() -> String { SETTINGS_SOCKET_NAME.to_string() }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub level: NotificationLevel,
    pub timestamp: u64,
    pub app_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NotifRequest {
    Push { title: String, body: String, level: NotificationLevel, app_name: Option<String> },
    List,
    Dismiss { id: String },
    Clear,
    Subscribe,
}

#[allow(dead_code)]
fn apply_entry(
    entry: &SettingEntry,
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
) {
    match entry.key {
        SettingKey::WifiEnabled => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = wifi;
                s.set_if_modified(v);
            }
        }
        SettingKey::BluetoothEnabled => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = bt;
                s.set_if_modified(v);
            }
        }
        SettingKey::AudioVolume => {
            if let Some(v) = entry.value.as_f64() {
                let mut s = volume;
                s.set_if_modified(v);
            }
        }
        SettingKey::AudioMuted => {
            if let Some(v) = entry.value.as_bool() {
                let mut s = muted;
                s.set_if_modified(v);
            }
        }
        SettingKey::DisplayBrightness => {
            if let Some(v) = entry.value.as_f64() {
                let mut s = brightness;
                s.set_if_modified(v);
            }
        }
        SettingKey::DoNotDisturb => {
            let on = entry.value.as_str().map(|s| s != "off").unwrap_or(false) || entry.value.as_bool().unwrap_or(false);
            let mut s = dnd;
            s.set_if_modified(on);
        }
        SettingKey::PowerProfile => {}
        _ => {}
    }
}

fn parse_capacity_pct(capacity: &str) -> u8 { capacity.trim().trim_end_matches('%').parse::<u8>().unwrap_or(0).min(100) }

fn load_initial_batch(
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
    connected: State<bool>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<Vec<SettingEntry>>>();
    std::thread::spawn(move || match get_all_settings(socket()) {
        Ok(entries) => {
            let _ = tx.send(Some(entries));
        }
        Err(e) => {
            eprintln!("topbar get_all_settings offline: {e}");
            let _ = tx.send(None);
        }
    });
    spawn(async move {
        if let Some(opt) = rx.recv().await {
            match opt {
                Some(entries) => {
                    for entry in &entries {
                        apply_entry(entry, wifi, bt, volume, muted, brightness, dnd);
                    }
                    let mut c = connected;
                    c.set_if_modified(true);
                }
                None => {
                    let mut c = connected;
                    c.set_if_modified(false);
                }
            }
        }
    });
}

fn subscribe_live(
    wifi: State<bool>,
    bt: State<bool>,
    volume: State<f64>,
    muted: State<bool>,
    brightness: State<f64>,
    dnd: State<bool>,
    connected: State<bool>,
) {
    let rx = match ipsea::settings::subscribe_channel(socket(), SubscriptionFilter::all()) {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("topbar subscribe failed: {e}");
            return;
        }
    };
    let mut rx = rx;
    spawn(async move {
        while let Some(evt) = rx.recv().await {
            match evt {
                SettingsEvent::Changed { key, value, source: _ } => match key {
                    SettingKey::WifiEnabled => {
                        if let Some(v) = value.as_bool() {
                            let mut s = wifi;
                            s.set_if_modified(v);
                        }
                    }
                    SettingKey::BluetoothEnabled => {
                        if let Some(v) = value.as_bool() {
                            let mut s = bt;
                            s.set_if_modified(v);
                        }
                    }
                    SettingKey::AudioVolume => {
                        if let Some(v) = value.as_f64() {
                            let mut s = volume;
                            s.set_if_modified(v);
                        }
                    }
                    SettingKey::AudioMuted => {
                        if let Some(v) = value.as_bool() {
                            let mut s = muted;
                            s.set_if_modified(v);
                        }
                    }
                    SettingKey::DisplayBrightness => {
                        if let Some(v) = value.as_f64() {
                            let mut s = brightness;
                            s.set_if_modified(v);
                        }
                    }
                    SettingKey::DoNotDisturb => {
                        let on = value.as_str().map(|s| s != "off").unwrap_or(false) || value.as_bool().unwrap_or(false);
                        let mut s = dnd;
                        s.set_if_modified(on);
                    }
                    SettingKey::PowerProfile => {}
                    _ => {}
                },
                SettingsEvent::LockChanged { .. } => {}
                SettingsEvent::Reloaded => {
                    load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected);
                }
                SettingsEvent::Alert(_) => {}
            }
        }
    });
}

fn apply(key: SettingKey, val: SettingValue) {
    std::thread::spawn(move || {
        let _ = set_and_apply(socket(), key, val);
    });
}

fn clock_now() -> String { chrono::Local::now().format("%H:%M").to_string() }

fn power_action(action: &'static str) {
    std::thread::spawn(move || {
        let ok = match action {
            "logout" => HyprlandBackend.log_out(),
            "reboot" => HyprlandBackend.reboot(),
            _ => HyprlandBackend.power_off(),
        };
        if !ok {
            eprintln!("topbar power action {action} failed");
        }
    });
}

fn fetch_notifications_blocking() -> Vec<Notification> {
    let shared = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Notification>::new()));
    let shared2 = shared.clone();
    let _ = ipsea::send_command(
        "notifications".to_string(),
        &NotifRequest::List,
        Some(move |n: Notification| {
            if let Ok(mut g) = shared2.lock() {
                g.push(n);
            }
        }),
    );
    shared.lock().map(|g| g.clone()).unwrap_or_default()
}

fn clear_notifications() {
    std::thread::spawn(|| {
        let _: std::io::Result<()> = ipsea::send_command::<NotifRequest, Notification, fn(Notification)>(
            "notifications".to_string(),
            &NotifRequest::Clear,
            None,
        );
    });
}

fn app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();

    let wifi = use_state(|| true);
    let bt = use_state(|| true);
    let volume = use_state(|| 65.0);
    let muted = use_state(|| false);
    let brightness = use_state(|| 72.0);
    let dnd = use_state(|| false);
    let panel_open = use_state(|| false);
    let connected = use_state(|| false);
    let clock = use_state(clock_now);

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

    let notifications = use_state(|| Vec::<Notification>::new());
    let notif_initial = use_hook(|| fetch_notifications_blocking());
    {
        let mut n = notifications;
        n.set_if_modified(notif_initial.clone());
    }

    use_hook(move || {
        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected);
        subscribe_live(wifi, bt, volume, muted, brightness, dnd, connected);
        let mut clk = clock;
        let mut wired_state = wired;
        let mut pct_state = battery_pct;
        let mut status_state = battery_status;
        let mut notif_state = notifications;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                clk.set_if_modified(clock_now());
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
                let notifs = tokio::task::spawn_blocking(fetch_notifications_blocking).await.unwrap_or_default();
                notif_state.set_if_modified(notifs);
            }
        });
    });

    let clock_str = clock.read().clone();
    let wifi_on = *wifi.read();
    let bt_on = *bt.read();
    let is_muted = *muted.read();
    let _is_connected = *connected.read();
    let wired_info = wired.read().clone();
    let wired_on = wired_info.as_ref().map(|w| w.connected).unwrap_or(false);
    let show_panel = *panel_open.read();
    let bat_pct = *battery_pct.read();
    let bat_status = battery_status.read().clone();
    let notifs = notifications.read().clone();
    let notif_count = notifs.len();
    let power_snapshot = HyprlandBackend.get_power_info();

    let status = rect()
        .height(Size::px(36.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .padding((4., 16.))
        .background(t.panel)
        .border(Border::new().width(1.).fill(t.border))
        .content(Content::Flex)
        .child(rect().width(Size::flex(1.))) // Left spacer
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(12.)
                .content(Content::Flex)
                .child(
                    // Interactive sparse icons group
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .cursor(CursorIcon::Pointer)
                        .content(Content::Flex)
                        .on_press({
                            let mut p = panel_open;
                            move |_| {
                                let next = !*p.read();
                                p.set(next);
                            }
                        })
                        .child(icon(
                            if wired_on { WIRED } else { WIFI },
                            14.,
                            if wifi_on || wired_on { t.text } else { t.text_dim },
                        ))
                        .child(icon(BLUETOOTH, 14., if bt_on { t.text } else { t.text_dim }))
                        .child(icon(SOUND, 14., if is_muted { t.text_dim } else { t.text }))
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(4.)
                                .child(icon(BATTERY, 12., if bat_pct > 20 { t.text } else { t.accent_red }))
                                .child(label().font_size(12.).color(t.text).text(format!("{bat_pct}%"))),
                        ),
                )
                .child(
                    // Control center toggle icon
                    rect()
                        .padding((4., 6.))
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut p = panel_open;
                            move |_| {
                                let next = !*p.read();
                                p.set(next);
                            }
                        })
                        .child(icon(OVERVIEW, 14., t.text)),
                )
                .child(
                    // Clock
                    label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(clock_str),
                ),
        );

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
                    .child(secondary_button("Clear", || clear_notifications())),
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

    let actions_row = rect()
        .width(Size::fill())
        .horizontal()
        .spacing(8.)
        .content(Content::Flex)
        .child(rect().width(Size::flex(1.)).child(secondary_button("Log out", || power_action("logout"))))
        .child(rect().width(Size::flex(1.)).child(secondary_button("Restart", || power_action("reboot"))))
        .child(rect().width(Size::flex(1.)).child(danger_button("Shut down", || power_action("shutdown"))));

    let panel = if show_panel {
        rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .padding(12.)
            .background(t.bg)
            .content(Content::Flex)
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
            .child(battery_card)
            .child(notifications_card)
            .child(actions_row)
    } else {
        rect().width(Size::fill()).height(Size::px(0.))
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .vertical()
        .background(t.bg_base)
        .child(status)
        .child(rect().width(Size::fill()).height(Size::fill()).vertical().child(panel))
        .into_element()
}

fn main() {
    // Run hyprctl to set up window rules dynamically for the topbar
    std::process::Command::new("hyprctl").args(["keyword", "windowrulev2", "float, class:^(topbar)$"]).output().ok();
    std::process::Command::new("hyprctl").args(["keyword", "windowrulev2", "pin, class:^(topbar)$"]).output().ok();
    std::process::Command::new("hyprctl").args(["keyword", "windowrulev2", "move 0 0, class:^(topbar)$"]).output().ok();
    std::process::Command::new("hyprctl").args(["keyword", "windowrulev2", "noanim, class:^(topbar)$"]).output().ok();
    std::process::Command::new("hyprctl").args(["keyword", "windowrulev2", "noblur, class:^(topbar)$"]).output().ok();
    std::process::Command::new("hyprctl").args(["keyword", "monitor", ",addreserved,36,0,0,0"]).output().ok();

    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("topbar")
                .with_app_id("topbar")
                .with_size(1920., 600.)
                .with_decorations(false)
                .with_transparency(true),
        ),
    )
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        ipsea::settings::{SettingsRequest, SettingsResponse},
        std::sync::{Arc, Mutex},
    };

    fn spawn_toggle_server(socket: &str, wifi: Arc<Mutex<bool>>) {
        let socket = socket.to_string();
        std::thread::spawn(move || {
            let _ = ipsea::start_server(
                socket,
                move |req: SettingsRequest, sender: std::sync::mpsc::Sender<SettingsResponse>| match req {
                    SettingsRequest::Get { key } => {
                        if key == SettingKey::WifiEnabled {
                            let on = *wifi.lock().expect("lock");
                            let _ = sender.send(SettingsResponse::Setting(ipsea::settings::SettingEntry::new(
                                key,
                                ipsea::settings::SettingValue::Bool(on),
                            )));
                        } else {
                            let _ = sender.send(SettingsResponse::NotFound { key });
                        }
                    }
                    SettingsRequest::GetAll => {
                        let on = *wifi.lock().expect("lock");
                        let entries = vec![ipsea::settings::SettingEntry::new(
                            SettingKey::WifiEnabled,
                            ipsea::settings::SettingValue::Bool(on),
                        )];
                        for e in entries {
                            let _ = sender.send(SettingsResponse::Setting(e));
                        }
                    }
                    SettingsRequest::SetAndApply { key, value, source } => {
                        if key == SettingKey::WifiEnabled {
                            if let Some(on) = value.as_bool() {
                                *wifi.lock().expect("lock") = on;
                                let _ = sender.send(SettingsResponse::Ok);
                                let _ = sender.send(SettingsResponse::Event(SettingsEvent::Changed { key, value, source }));
                            }
                        } else {
                            let _ = sender.send(SettingsResponse::Ok);
                        }
                    }
                    SettingsRequest::Subscribe { .. } => {}
                    _ => {}
                },
            );
        });
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    #[test]
    fn test_topbar_toggle_round_trip() {
        let socket = format!("test-topbar-toggles-{}", std::process::id());
        let wifi = Arc::new(Mutex::new(true));
        spawn_toggle_server(&socket, wifi.clone());

        let entry = get_setting(&socket, SettingKey::WifiEnabled).expect("get failed").expect("entry missing");
        assert_eq!(entry.value.as_bool(), Some(true));

        let res = set_and_apply(&socket, SettingKey::WifiEnabled, false).expect("set_and_apply failed");
        assert!(res.is_ok());
        assert!(!*wifi.lock().expect("lock"));

        let entry = get_setting(&socket, SettingKey::WifiEnabled).expect("get failed").expect("entry missing");
        assert_eq!(entry.value.as_bool(), Some(false));
    }

    #[test]
    fn test_topbar_dead_socket_offline() {
        let res = get_setting("finick-topbar-dead-socket-xyz", SettingKey::WifiEnabled);
        assert!(res.is_err());
    }

    #[test]
    fn test_topbar_batch_offline_no_panic() {
        let res = get_all_settings("finick-topbar-dead-socket-xyz-batch");
        assert!(res.is_err());
        let n = fetch_notifications_blocking();
        assert!(n.is_empty());
    }
}
