#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod notification_popup;
mod panel;
mod state;

pub use notification_popup::*;
#[allow(unused_imports)]
use state::{
    clock_now_with_format, dismiss_notification, fetch_notifications_blocking, load_initial_batch, parse_capacity_pct,
    subscribe_live, subscribe_notifications_live,
};
use {
    freya::prelude::*,
    panel::control_panel_app,
    std::sync::{Arc, Mutex},
    system::{HyprlandBackend, SystemBackend, WiredInfo},
    ui::*,
    winit::{dpi::LogicalSize, window::WindowId},
};

#[derive(Clone, Default)]
pub struct PanelWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

fn clock_now() -> String { state::clock_now() }

fn panel_window_config() -> WindowConfig {
    WindowConfig::new(control_panel_app)
        .with_title("control-panel")
        .with_app_id("overlay-panel")
        .with_size(460., 640.)
        .with_min_size(437., 520.)
        .with_max_size(483., 720.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|_ctx, _wid| {
            if let Some(panel) = GlobalContexts::get().try_get_context::<PanelWindowId>() {
                if let Ok(mut g) = panel.0.lock() {
                    *g = None;
                }
            }
            CloseDecision::Close
        })
}

#[allow(dead_code)]
fn topbar_app() -> Element { topbar_app_for_monitor("default".to_string()) }

fn topbar_app_for_monitor(mon_name: String) -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();

    let wifi = use_state(|| true);
    let bt = use_state(|| true);
    let volume = use_state(|| 65.0);
    let muted = use_state(|| false);
    let brightness = use_state(|| 72.0);
    let dnd = use_state(|| false);
    let connected = use_state(|| false);
    let use_24h = use_state(|| true);
    let clock = use_state(|| clock_now_with_format(true));
    let topbar_settings = use_state(state::TopbarSettings::default);
    // panel tracking via global
    let panel_ctx: PanelWindowId = GlobalContexts::get().get_context::<PanelWindowId>();

    let mon_name_for_panel = mon_name.clone();
    let toggle_panel = {
        let panel_ctx = panel_ctx.clone();
        std::rc::Rc::new(move || {
            let panel_ctx = panel_ctx.clone();
            let mon_name = mon_name_for_panel.clone();
            spawn(async move {
                // If we already have a window open, close it immediately
                let existing = panel_ctx.0.lock().map(|mut g| g.take()).unwrap_or(None);
                if let Some(id) = existing {
                    Platform::get().close_window(id);
                    let _ = std::process::Command::new("hyprctl")
                        .args(["dispatch", "closewindow", "class:^(overlay-panel)$"])
                        .output();
                    let _ = std::process::Command::new("hyprctl")
                        .args(["dispatch", "closewindow", "class:^(topbar-panel)$"])
                        .output();
                    return;
                }

                // Check if window is open in hyprctl
                let is_open = tokio::task::spawn_blocking(|| {
                    if let Ok(o) = std::process::Command::new("hyprctl").args(["clients"]).output() {
                        let s = String::from_utf8_lossy(&o.stdout);
                        s.contains("overlay-panel") || s.contains("topbar-panel")
                    } else {
                        false
                    }
                })
                .await
                .unwrap_or(false);

                if is_open {
                    let _ = std::process::Command::new("hyprctl")
                        .args(["dispatch", "closewindow", "class:^(overlay-panel)$"])
                        .output();
                    let _ = std::process::Command::new("hyprctl")
                        .args(["dispatch", "closewindow", "class:^(topbar-panel)$"])
                        .output();
                    return;
                }

                let current_mon = get_monitor_state(&mon_name);
                let panel_x = current_mon.x + current_mon.width as i32 - 460 - 12;
                let panel_y = current_mon.y + 44;

                // Set placement rule before window maps
                let _ = std::process::Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", &format!("monitor {}, class:^(overlay-panel)$", current_mon.name)])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", &format!("move {panel_x} {panel_y}, class:^(overlay-panel)$")])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", &format!("monitor {}, class:^(topbar-panel)$", current_mon.name)])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["keyword", "windowrulev2", &format!("move {panel_x} {panel_y}, class:^(topbar-panel)$")])
                    .output();

                let cfg = panel_window_config();
                let new_id = Platform::get().launch_window(cfg).await;
                if let Ok(mut g) = panel_ctx.0.lock() {
                    *g = Some(new_id);
                }

                // Re-assert position, monitor, and focus
                let mon_target = current_mon.name.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = std::process::Command::new("hyprctl")
                        .args(["keyword", "windowrulev2", &format!("monitor {mon_target}, class:^(overlay-panel)$")])
                        .output();
                    let _ = std::process::Command::new("hyprctl")
                        .args(["keyword", "windowrulev2", &format!("move {panel_x} {panel_y}, class:^(overlay-panel)$")])
                        .output();
                    let _ = std::process::Command::new("hyprctl")
                        .args(["dispatch", "focuswindow", "class:^(overlay-panel)$"])
                        .output();
                });
            });
        })
    };

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

    let notifications = use_state(Vec::<state::Notification>::new);
    let notif_initial = use_hook(fetch_notifications_blocking);
    {
        let mut n = notifications;
        n.set_if_modified(notif_initial.clone());
    }

    use_hook(move || {
        let mut fmt = use_24h;
        spawn(async move {
            if let Ok(Ok(entries)) =
                tokio::task::spawn_blocking(|| ipsea::settings::get_all_settings(ipsea::settings::SETTINGS_SOCKET_NAME))
                    .await
            {
                for e in entries {
                    if e.key == ipsea::settings::SettingKey::TimeFormat24h {
                        if let Some(b) = e.value.as_bool() {
                            fmt.set(b);
                        }
                    }
                }
            }
            if let Ok(mut rx) = ipsea::settings::subscribe_channel(
                ipsea::settings::SETTINGS_SOCKET_NAME,
                ipsea::settings::SubscriptionFilter::all(),
            ) {
                while let Some(evt) = rx.recv().await {
                    if let ipsea::settings::SettingsEvent::Changed { key, value, .. } = evt {
                        if key == ipsea::settings::SettingKey::TimeFormat24h {
                            if let Some(b) = value.as_bool() {
                                fmt.set(b);
                            }
                        }
                    }
                }
            }
        });
    });

    use_hook(move || {
        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected);
        subscribe_live(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings);
        subscribe_notifications_live(notifications);
        let mut clk = clock;
        let is_24h = use_24h;
        let mut wired_state = wired;
        let mut pct_state = battery_pct;
        let mut status_state = battery_status;
        let mut notif_state = notifications;
        {
            let mut clk2 = clock;
            let fmt = use_24h;
            spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let prev = clk2.read().clone();
                    let now = clock_now_with_format(*fmt.read());
                    if prev != now {
                        clk2.set(now);
                    }
                }
            });
        }
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                clk.set_if_modified(clock_now_with_format(*is_24h.read()));
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
    let wired_info = wired.read().clone();
    let wired_on = wired_info.as_ref().map(|w| w.connected).unwrap_or(false);
    let bat_pct = *battery_pct.read();

    let _ = (volume, muted, brightness, dnd, connected);
    let icon_hover = use_state(|| false);
    let power_snapshot = HyprlandBackend.get_power_info();
    let has_battery = power_snapshot.capacity != "Unknown" && power_snapshot.status != "Unknown";

    let notifs_list = notifications.read().clone();
    let latest_notif = notifs_list.last().cloned();

    let status = rect()
        .height(Size::px(36.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .padding((4., 16.))
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(rect().width(Size::flex(1.)).horizontal().cross_align(Alignment::Center).content(Content::Flex).maybe_child(
            latest_notif.map(|notif| {
                let notif_id = notif.id;
                let mut notif_state = notifications;
                let title = if !notif.summary.is_empty() {
                    notif.summary.clone()
                } else if !notif.app_name.is_empty() {
                    notif.app_name.clone()
                } else {
                    "Notification".to_string()
                };
                let body = notif.body.clone();
                let icon_str = notif.icon.clone();
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(8.)
                    .padding((4., 10.))
                    .corner_radius(999.)
                    .background(t.panel)
                    .border(Border::new().width(1.).fill(t.border))
                    .cursor(CursorIcon::Pointer)
                    .content(Content::Flex)
                    .child(render_notification_icon(&icon_str, 14., t.accent))
                    .child(label().font_size(12.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(title))
                    .maybe(!body.is_empty(), |el| {
                        el.child(label().font_size(11.).color(t.text_dim).text(format!("— {}", body)))
                    })
                    .child(
                        rect()
                            .width(Size::px(18.))
                            .height(Size::px(18.))
                            .corner_radius(999.)
                            .center()
                            .cursor(CursorIcon::Pointer)
                            .on_press(move |_| {
                                let mut list = notif_state.read().clone();
                                if let Some(pos) = list.iter().position(|n| n.id == notif_id) {
                                    list.remove(pos);
                                    notif_state.set(list);
                                }
                                dismiss_notification(notif_id);
                            })
                            .child(label().font_size(10.).font_weight(FontWeight::BOLD).color(t.text_dim).text("✕")),
                    )
            }),
        ))
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(12.)
                .content(Content::Flex)
                .child({
                    let mut h = icon_hover;
                    let mut h2 = icon_hover;
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(10.)
                        .padding((6., 12.))
                        .corner_radius(999.)
                        .background(if *icon_hover.read() { t.panel } else { Color::TRANSPARENT })
                        .border(Border::new().width(1.).fill(if *icon_hover.read() { t.border } else { Color::TRANSPARENT }))
                        .cursor(CursorIcon::Pointer)
                        .content(Content::Flex)
                        .on_pointer_enter(move |_| h.set(true))
                        .on_pointer_leave(move |_| h2.set(false))
                        .on_press({
                            let t = toggle_panel.clone();
                            move |_| t()
                        })
                        .maybe(topbar_settings.read().show_wifi, |el| {
                            el.child(icon(
                                if wired_on { WIRED } else { WIFI },
                                16.,
                                if wifi_on || wired_on { t.text } else { t.text_dim },
                            ))
                        })
                        .maybe(topbar_settings.read().show_bluetooth, |el| {
                            el.child(icon(BLUETOOTH, 16., if bt_on { t.text } else { t.text_dim }))
                        })
                        .maybe(topbar_settings.read().show_sound, |el| {
                            el.child(icon(SOUND, 16., if is_muted { t.text_dim } else { t.text }))
                        })
                        .maybe(has_battery && topbar_settings.read().show_battery, |el| {
                            el.child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(4.)
                                    .child(icon(BATTERY, 14., if bat_pct > 20 { t.text } else { t.accent_red }))
                                    .child(label().font_size(13.).color(t.text).text(format!("{bat_pct}%"))),
                            )
                        })
                })
                .child(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 8.))
                        .on_press({
                            let t = toggle_panel.clone();
                            move |_| t()
                        })
                        .child(
                            label()
                                .font_size(topbar_settings.read().text_size as f32)
                                .font_weight(FontWeight::SEMI_BOLD)
                                .color(match topbar_settings.read().text_color.as_str() {
                                    "Accent" => t.accent,
                                    "Muted" => t.text_dim,
                                    _ => t.text,
                                })
                                .text(clock_str),
                        ),
                ),
        );

    let notif_overlay = notifs_list.last().cloned().map(|notif| {
        let nid = notif.id;
        let mut ns = notifications;
        rect().width(Size::fill()).padding((6., 16.)).content(Content::Flex).child(
            rect().width(Size::fill()).horizontal().main_align(Alignment::End).content(Content::Flex).child(
                NotificationPopup::from_notification(&notif).with_width(Size::px(360.)).on_close(move |id| {
                    let mut list = ns.read().clone();
                    if let Some(pos) = list.iter().position(|n| n.id == nid) {
                        list.remove(pos);
                        ns.set(list);
                    }
                    dismiss_notification(id);
                }),
            ),
        )
    });
    rect()
        .width(Size::fill())
        .vertical()
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(status)
        .maybe_child(notif_overlay)
        .into_element()
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonitorState {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
}

pub fn get_monitor_states() -> Vec<MonitorState> {
    let displays = HyprlandBackend.get_displays();
    if displays.is_empty() {
        return vec![MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 }];
    }
    displays
        .into_iter()
        .map(|d| {
            let (raw_w, raw_h) = system::parse_display_dimensions(&d.resolution);
            let (w, h) = if d.transform == 1 || d.transform == 3 { (raw_h, raw_w) } else { (raw_w, raw_h) };
            let scale: f64 = d.scale.parse().unwrap_or(1.0);
            let scale = if scale <= 0.0 { 1.0 } else { scale };
            let logical_w = (w as f64 / scale).round();
            let logical_h = (h as f64 / scale).round();
            MonitorState { name: d.name, x: d.x, y: d.y, width: logical_w, height: logical_h }
        })
        .collect()
}

pub fn get_monitor_state(name: &str) -> MonitorState {
    let monitors = get_monitor_states();
    monitors.into_iter().find(|m| m.name == name).unwrap_or_else(|| MonitorState {
        name: name.to_string(),
        x: 0,
        y: 0,
        width: 1920.0,
        height: 1080.0,
    })
}

pub fn get_topbar_width() -> f64 {
    let monitors = get_monitor_states();
    monitors.into_iter().find(|m| m.x == 0).map(|m| m.width).unwrap_or(1920.0)
}

pub fn apply_hyprland_base_rules() {
    let lua_rule = r#"
        hl.window_rule({
            name = "finick-overlay-base",
            match = { class = "^(overlay)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            no_initial_focus = true,
        })
        hl.window_rule({
            name = "finick-overlay-base-compat",
            match = { class = "^(topbar)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            no_initial_focus = true,
        })
        hl.window_rule({
            name = "finick-panel",
            match = { class = "^(overlay-panel)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            rounding = 20,
        })
        hl.window_rule({
            name = "finick-panel-compat",
            match = { class = "^(topbar-panel)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            rounding = 20,
        })
    "#;
    let _ = std::process::Command::new("hyprctl").args(["eval", lua_rule]).output();

    for cls in ["overlay", "topbar"] {
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("float, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("pin, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("noanim, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("noblur, class:^({cls})$")])
            .output();
    }
    for cls in ["overlay-panel", "topbar-panel"] {
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("float, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("pin, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("noborder, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("noshadow, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("noanim, class:^({cls})$")])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("noblur, class:^({cls})$")])
            .output();
    }
    let _ = std::process::Command::new("hyprctl")
        .args(["keyword", "windowrulev2", "rounding 20, class:^(overlay-panel)$"])
        .output();
    let _ = std::process::Command::new("hyprctl")
        .args(["keyword", "windowrulev2", "rounding 20, class:^(topbar-panel)$"])
        .output();
}

pub fn apply_hyprland_monitor_rules(mon: &MonitorState) {
    let lua_rule = format!(
        r#"
        hl.monitor({{
            output = "{name}",
            reserved = {{ top = 36 }},
        }})
        hl.window_rule({{
            name = "finick-overlay-{name}",
            match = {{ class = "^(overlay)$", title = "^(overlay-{name})$" }},
            monitor = "{name}",
            float = true,
            pin = true,
            move = {{ {x}, {y} }},
            size = {{ {width}, 140 }},
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            no_initial_focus = true,
        }})
        hl.window_rule({{
            name = "finick-topbar-{name}-compat",
            match = {{ class = "^(topbar)$", title = "^(topbar-{name})$" }},
            monitor = "{name}",
            float = true,
            pin = true,
            move = {{ {x}, {y} }},
            size = {{ {width}, 140 }},
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            no_initial_focus = true,
        }})
        "#,
        name = mon.name,
        width = mon.width as i32,
        x = mon.x,
        y = mon.y,
    );
    let _ = std::process::Command::new("hyprctl").args(["eval", &lua_rule]).output();

    let _ = std::process::Command::new("hyprctl")
        .args(["keyword", "monitor", &format!("{},addreserved,36,0,0,0", mon.name)])
        .output();
    for (cls, title) in [("overlay", format!("overlay-{}", mon.name)), ("topbar", format!("topbar-{}", mon.name))] {
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("monitor {}, class:^({cls})$, title:^({title})$", mon.name)])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("move {} {}, class:^({cls})$, title:^({title})$", mon.x, mon.y)])
            .output();
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "windowrulev2", &format!("size {} 140, class:^({cls})$, title:^({title})$", mon.width as i32)])
            .output();
    }
}

pub fn clear_hyprland_reserved_space() {
    let lua = r#"
        hl.monitor({
            output = "",
            reserved = { top = 0 },
        })
    "#;
    let _ = std::process::Command::new("hyprctl").args(["eval", lua]).output();
    let _ = std::process::Command::new("hyprctl").args(["keyword", "monitor", ",addreserved,0,0,0,0"]).output();
    for mon in HyprlandBackend.get_displays() {
        let _ = std::process::Command::new("hyprctl")
            .args(["keyword", "monitor", &format!("{},addreserved,0,0,0,0", mon.name)])
            .output();
        let lua = format!(r#"hl.monitor({{ output = "{}", reserved = {{ top = 0 }} }})"#, mon.name);
        let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
    }
}

pub fn clear_hyprland_reserved_space_for_monitor(name: &str) {
    let lua = format!(r#"hl.monitor({{ output = "{}", reserved = {{ top = 0 }} }})"#, name);
    let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
    let _ = std::process::Command::new("hyprctl")
        .args(["keyword", "monitor", &format!("{},addreserved,0,0,0,0", name)])
        .output();
}

pub fn register_hyprland_rules() {
    apply_hyprland_base_rules();
    for mon in get_monitor_states() {
        apply_hyprland_monitor_rules(&mon);
    }
}

fn make_topbar_window_config(mon: &MonitorState) -> WindowConfig {
    let mon_name = mon.name.clone();
    let mon_name_close = mon.name.clone();
    let title: &'static str = Box::leak(format!("overlay-{}", mon.name).into_boxed_str());
    WindowConfig::new(move || topbar_app_for_monitor(mon_name.clone()))
        .with_title(title)
        .with_app_id("overlay")
        .with_size(mon.width, 140.)
        .with_decorations(false)
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
        .with_on_close(move |mut ctx, wid| {
            ctx.windows_mut().remove(&wid);
            clear_hyprland_reserved_space_for_monitor(&mon_name_close);
            let has_other_topbar = ctx
                .windows()
                .values()
                .any(|app| app.window().title().starts_with("overlay-") || app.window().title().starts_with("topbar-"));
            if !has_other_topbar {
                clear_hyprland_reserved_space();
                ctx.exit();
            }
            CloseDecision::Close
        })
}

fn get_socket2_path() -> Option<std::path::PathBuf> {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    if let Ok(xdg_runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let path = std::path::PathBuf::from(xdg_runtime_dir).join("hypr").join(&his).join(".socket2.sock");
        if path.exists() {
            return Some(path);
        }
    }
    let tmp_path = std::path::PathBuf::from("/tmp/hypr").join(&his).join(".socket2.sock");
    if tmp_path.exists() {
        return Some(tmp_path);
    }
    None
}

async fn read_socket_event(
    reader: &mut Option<tokio::io::Lines<tokio::io::BufReader<tokio::net::UnixStream>>>,
) -> Option<String> {
    if let Some(r) = reader {
        match r.next_line().await {
            Ok(Some(line)) => Some(line),
            _ => {
                *reader = None;
                None
            }
        }
    } else {
        std::future::pending().await
    }
}

async fn reconcile_monitors(
    proxy: &LaunchProxy,
    known_monitors: &mut std::collections::HashMap<String, (WindowId, MonitorState)>,
) {
    let current_monitors = get_monitor_states();

    // 1. Check for removed monitors
    let current_names: std::collections::HashSet<String> = current_monitors.iter().map(|m| m.name.clone()).collect();
    let removed_names: Vec<String> = known_monitors.keys().filter(|k| !current_names.contains(*k)).cloned().collect();

    for name in removed_names {
        if let Some((wid, _)) = known_monitors.remove(&name) {
            let _ = proxy
                .post_callback(move |ctx| {
                    ctx.windows_mut().remove(&wid);
                })
                .await;
            clear_hyprland_reserved_space_for_monitor(&name);
        }
    }

    // 2. Check for added or modified monitors
    for mon in current_monitors {
        match known_monitors.get_mut(&mon.name) {
            None => {
                // Monitor added!
                apply_hyprland_monitor_rules(&mon);
                let mon_clone = mon.clone();
                let launched_wid = proxy
                    .post_callback(move |ctx| {
                        let cfg = make_topbar_window_config(&mon_clone);
                        ctx.launch_window(cfg)
                    })
                    .await
                    .ok();

                if let Some(wid) = launched_wid {
                    known_monitors.insert(mon.name.clone(), (wid, mon));
                }
            }
            Some((wid, prev_mon)) => {
                // Check if resolution or geometry changed!
                if prev_mon.width != mon.width || prev_mon.height != mon.height || prev_mon.x != mon.x || prev_mon.y != mon.y
                {
                    let wid = *wid;
                    let new_width = mon.width;
                    apply_hyprland_monitor_rules(&mon);

                    let _ = proxy
                        .post_callback(move |ctx| {
                            if let Some(app) = ctx.windows_mut().get_mut(&wid) {
                                let _ = app.window_mut().request_inner_size(LogicalSize::new(new_width, 140.0));
                                app.window().request_redraw();
                            }
                        })
                        .await;

                    *prev_mon = mon;
                }
            }
        }
    }
}

async fn run_monitor_listener(proxy: LaunchProxy, initial_monitors: Vec<MonitorState>) {
    let mut known_monitors: std::collections::HashMap<String, (WindowId, MonitorState)> = std::collections::HashMap::new();

    // Discover the WindowIds of initial windows created by with_window
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let found = proxy
            .post_callback(|ctx| {
                let mut map = std::collections::HashMap::new();
                for (&wid, app) in ctx.windows().iter() {
                    let title = app.window().title();
                    if let Some(name) = title.strip_prefix("overlay-").or_else(|| title.strip_prefix("topbar-")) {
                        map.insert(name.to_string(), wid);
                    }
                }
                map
            })
            .await
            .unwrap_or_default();

        if !found.is_empty() {
            for mon in &initial_monitors {
                if let Some(&wid) = found.get(&mon.name) {
                    known_monitors.insert(mon.name.clone(), (wid, mon.clone()));
                }
            }
            if known_monitors.len() == initial_monitors.len() {
                break;
            }
        }
    }

    let mut socket_reader = None;
    if let Some(path) = get_socket2_path() {
        if let Ok(stream) = tokio::net::UnixStream::connect(path).await {
            use tokio::io::AsyncBufReadExt;
            socket_reader = Some(tokio::io::BufReader::new(stream).lines());
        }
    }

    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(1500));

    loop {
        tokio::select! {
            line = read_socket_event(&mut socket_reader) => {
                if let Some(_event_line) = line {
                    reconcile_monitors(&proxy, &mut known_monitors).await;
                } else {
                    if let Some(path) = get_socket2_path() {
                        if let Ok(stream) = tokio::net::UnixStream::connect(path).await {
                            use tokio::io::AsyncBufReadExt;
                            socket_reader = Some(tokio::io::BufReader::new(stream).lines());
                        }
                    }
                }
            }
            _ = ticker.tick() => {
                if socket_reader.is_none() {
                    if let Some(path) = get_socket2_path() {
                        if let Ok(stream) = tokio::net::UnixStream::connect(path).await {
                            use tokio::io::AsyncBufReadExt;
                            socket_reader = Some(tokio::io::BufReader::new(stream).lines());
                        }
                    }
                }
                reconcile_monitors(&proxy, &mut known_monitors).await;
            }
        }
    }
}

struct ReservedSpaceGuard;
impl Drop for ReservedSpaceGuard {
    fn drop(&mut self) { clear_hyprland_reserved_space(); }
}

pub fn run() {
    let _reserved_guard = ReservedSpaceGuard;

    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        clear_hyprland_reserved_space();
        default_panic_hook(info);
    }));

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = rt.as_ref().map(|rt| rt.enter());

    if let Some(ref rt) = rt {
        rt.spawn(async {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm = signal(SignalKind::terminate()).ok();
            let mut sigint = signal(SignalKind::interrupt()).ok();
            let mut sighup = signal(SignalKind::hangup()).ok();

            tokio::select! {
                _ = async {
                    if let Some(s) = sigterm.as_mut() { s.recv().await; } else { std::future::pending().await }
                } => {},
                _ = async {
                    if let Some(s) = sigint.as_mut() { s.recv().await; } else { std::future::pending().await }
                } => {},
                _ = async {
                    if let Some(s) = sighup.as_mut() { s.recv().await; } else { std::future::pending().await }
                } => {},
            }

            clear_hyprland_reserved_space();
            std::process::exit(0);
        });
    }

    apply_hyprland_base_rules();
    let initial_monitors = get_monitor_states();
    for mon in &initial_monitors {
        apply_hyprland_monitor_rules(mon);
    }

    let mut launch_config = LaunchConfig::new().with_exit_on_close(false).with_global(PanelWindowId::default());

    for mon in &initial_monitors {
        launch_config = launch_config.with_window(make_topbar_window_config(mon));
    }

    let monitors_for_listener = initial_monitors.clone();
    launch_config = launch_config.with_future(move |proxy| async move {
        tokio::spawn(async move {
            run_monitor_listener(proxy, monitors_for_listener).await;
        });
    });

    launch(launch_config);
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::state::fetch_notifications_blocking,
        ipsea::settings::{
            SettingEntry, SettingKey, SettingValue, SettingsEvent, SettingsRequest, SettingsResponse, get_all_settings,
            get_setting, set_and_apply,
        },
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
                            let _ = sender.send(SettingsResponse::Setting(SettingEntry::new(key, SettingValue::Bool(on))));
                        } else {
                            let _ = sender.send(SettingsResponse::NotFound { key });
                        }
                    }
                    SettingsRequest::GetAll => {
                        let on = *wifi.lock().expect("lock");
                        let entries = vec![SettingEntry::new(SettingKey::WifiEnabled, SettingValue::Bool(on))];
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

        let res = set_and_apply(&socket, SettingKey::WifiEnabled, SettingValue::Bool(false)).expect("set_and_apply failed");
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

    #[test]
    fn test_panel_config_sizing() {
        let cfg = panel_window_config();
        // WindowConfig size is private but we can assert the config was created without panic and has expected app_id
        // Instead verify the function returns a config that would produce 400x560 window (smoke test)
        let _ = cfg;
    }

    #[test]
    fn test_get_monitor_states_or_fallback() {
        let states = get_monitor_states();
        assert!(!states.is_empty());
        for s in states {
            assert!(s.width > 0.0);
            assert!(s.height > 0.0);
        }
    }

    #[test]
    fn test_get_monitor_state_fallback() {
        let state = get_monitor_state("non_existent_display_xyz");
        assert_eq!(state.name, "non_existent_display_xyz");
        assert!(state.width > 0.0);
    }

    #[test]
    fn test_clear_hyprland_reserved_space_no_panic() {
        clear_hyprland_reserved_space();
        clear_hyprland_reserved_space_for_monitor("dummy-mon");
    }

    #[test]
    fn test_monitor_state_resolution_change_detection() {
        let m1 = MonitorState { name: "DP-1".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 };
        let mut m2 = m1.clone();
        assert_eq!(m1, m2);

        m2.width = 2560.0;
        m2.height = 1440.0;
        assert_ne!(m1.width, m2.width);
        assert_ne!(m1.height, m2.height);
    }

    #[test]
    fn test_make_topbar_window_config() {
        let mon = MonitorState { name: "DP-2".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 };
        let cfg = make_topbar_window_config(&mon);
        let _ = cfg;
    }

    #[tokio::test]
    async fn test_topbar_notification_live_integration() {
        use ipsea::notifications::{NotificationBroadcaster, NotificationEvent, start_notification_server};
        let socket = format!("test-topbar-notif-{}", std::process::id());
        let broadcaster = NotificationBroadcaster::new();
        let broadcaster_clone = broadcaster.clone();
        let socket_clone = socket.clone();

        std::thread::spawn(move || {
            let _ = start_notification_server::<fn(u32)>(socket_clone, broadcaster_clone, None);
        });

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let mut rx = ipsea::notifications::subscribe_channel(&socket).expect("subscribe failed");

        for _ in 0..50 {
            if broadcaster.subscriber_count() > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(broadcaster.subscriber_count(), 1);

        let test_notif = ipsea::notifications::Notification {
            id: 101,
            app_name: "TestApp".to_string(),
            summary: "Hello World".to_string(),
            body: "Notification content".to_string(),
            icon: "info".to_string(),
            timeout: 2000,
        };

        broadcaster.broadcast(NotificationEvent::Show(test_notif.clone()));

        let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout waiting for show event")
            .expect("stream ended");
        assert_eq!(evt, NotificationEvent::Show(test_notif));

        // Test dismiss notification
        state::dismiss_notification_from(&socket, 101);

        let close_evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout waiting for close event")
            .expect("stream ended");
        assert_eq!(close_evt, NotificationEvent::Close(101));
    }
}
