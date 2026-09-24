#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

pub mod clipboard_manager;
pub mod launcher;
mod notification_panel;
mod notification_popup;
pub mod osd;
mod panel;
pub mod screenshot;
pub mod session;
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

#[derive(Clone, Default)]
pub struct NotificationPanelWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

#[derive(Clone, Default)]
pub struct ClipboardWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

#[derive(Clone, Default)]
pub struct ScreenshotWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

fn clock_now() -> String { state::clock_now() }

fn panel_window_config() -> WindowConfig {
    WindowConfig::new(control_panel_app)
        .with_title("control-panel")
        .with_app_id("overlay-panel")
        .with_size(460., 700.)
        .with_min_size(460., 700.)
        .with_max_size(460., 700.)
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
    let theme_state = use_init_app_theme(get_theme());
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
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-panel)$' }))"])
                        .output();
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(topbar-panel)$' }))"])
                        .output();
                    return;
                }
                if let Some(notif_ctx) = GlobalContexts::get().try_get_context::<crate::NotificationPanelWindowId>() {
                    if let Ok(mut g) = notif_ctx.0.lock() {
                        if let Some(nid) = g.take() {
                            Platform::get().close_window(nid);
                            let _ = std::process::Command::new("hyprctl")
                                .args([
                                    "eval",
                                    "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-notifications)$' }))",
                                ])
                                .output();
                        }
                    }
                }
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                    .output();
                if let Some(cw) = GlobalContexts::get().try_get_context::<crate::ClipboardWindowId>() {
                    if let Ok(mut g) = cw.0.lock() {
                        *g = None;
                    }
                }
                if let Some(sw) = GlobalContexts::get().try_get_context::<crate::ScreenshotWindowId>() {
                    if let Ok(mut g) = sw.0.lock() {
                        *g = None;
                    }
                }

                let current_mon = get_monitor_state(&mon_name);
                let panel_x = current_mon.x + current_mon.width as i32 - 460 - 12;
                let panel_y = current_mon.y + 44;

                // Set placement rule before window maps via Hyprland 0.55 Lua API
                let lua_rule = format!(
                    r#"
                    hl.window_rule({{
                        name = "finick-panel-position",
                        match = {{ class = "^(overlay-panel)$" }},
                        monitor = "{mon}",
                        float = true,
                        pin = true,
                        move = {{ {panel_x}, {panel_y} }},
                        border_size = 0,
                        no_shadow = true,
                        no_anim = true,
                        no_blur = true,
                        rounding = 20,
                    }})
                    hl.window_rule({{
                        name = "finick-panel-compat-position",
                        match = {{ class = "^(topbar-panel)$" }},
                        monitor = "{mon}",
                        float = true,
                        pin = true,
                        move = {{ {panel_x}, {panel_y} }},
                        border_size = 0,
                        no_shadow = true,
                        no_anim = true,
                        no_blur = true,
                        rounding = 20,
                    }})
                    "#,
                    mon = current_mon.name,
                    panel_x = panel_x,
                    panel_y = panel_y,
                );
                let _ = std::process::Command::new("hyprctl").args(["eval", &lua_rule]).output();
                tokio::time::sleep(std::time::Duration::from_millis(35)).await;

                let cfg = panel_window_config();
                let new_id = Platform::get().launch_window(cfg).await;
                if let Ok(mut g) = panel_ctx.0.lock() {
                    *g = Some(new_id);
                }

                tokio::spawn(async move {
                    for delay in [30, 80] {
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        let lua = format!(
                            r#"
                            local win = nil
                            for _, w in ipairs(hl.get_windows()) do
                                if w.title == 'control-panel' or w.class == 'overlay-panel' or w.class == 'topbar-panel' then
                                    win = w
                                    break
                                end
                            end
                            if win then
                                local addr = 'address:' .. tostring(win.address)
                                hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                hl.dispatch(hl.dsp.window.move({{ window = addr, x = {panel_x}, y = {panel_y} }}))
                                hl.dispatch(hl.dsp.window.resize({{ window = addr, x = 460, y = 700, relative = false }}))
                                return true
                            end
                            return false
                            "#,
                            panel_x = panel_x,
                            panel_y = panel_y,
                        );
                        if let Ok(out) = std::process::Command::new("hyprctl").args(["eval", &lua]).output() {
                            let s = String::from_utf8_lossy(&out.stdout);
                            if s.contains("true") {
                                break;
                            }
                        }
                    }
                });
            });
        })
    };

    let notif_panel_ctx: NotificationPanelWindowId = GlobalContexts::get().get_context::<NotificationPanelWindowId>();
    let mon_name_for_notif = mon_name.clone();
    let toggle_notifications = {
        let notif_panel_ctx = notif_panel_ctx.clone();
        std::rc::Rc::new(move || {
            let notif_panel_ctx = notif_panel_ctx.clone();
            let mon_name = mon_name_for_notif.clone();
            spawn(async move {
                let existing = notif_panel_ctx.0.lock().map(|mut g| g.take()).unwrap_or(None);
                if let Some(id) = existing {
                    Platform::get().close_window(id);
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-notifications)$' }))"])
                        .output();
                    return;
                }
                if let Some(panel_ctx) = GlobalContexts::get().try_get_context::<PanelWindowId>() {
                    if let Ok(mut g) = panel_ctx.0.lock() {
                        if let Some(pid) = g.take() {
                            Platform::get().close_window(pid);
                        }
                    }
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-panel)$' }))"])
                        .output();
                }
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                    .output();
                if let Some(cw) = GlobalContexts::get().try_get_context::<crate::ClipboardWindowId>() {
                    if let Ok(mut g) = cw.0.lock() {
                        *g = None;
                    }
                }
                if let Some(sw) = GlobalContexts::get().try_get_context::<crate::ScreenshotWindowId>() {
                    if let Ok(mut g) = sw.0.lock() {
                        *g = None;
                    }
                }

                let current_mon = get_monitor_state(&mon_name);
                let panel_x = current_mon.x + current_mon.width as i32 - 400 - 12;
                let panel_y = current_mon.y + 44;
                let lua_rule = format!(
                    r#"
                    hl.window_rule({{
                        name = "finick-notifications-position",
                        match = {{ class = "^(overlay-notifications)$" }},
                        monitor = "{mon}",
                        float = true,
                        pin = true,
                        move = {{ {panel_x}, {panel_y} }},
                        border_size = 0,
                        no_shadow = true,
                        no_anim = true,
                        no_blur = true,
                        rounding = 20,
                    }})
                    "#,
                    mon = current_mon.name,
                    panel_x = panel_x,
                    panel_y = panel_y,
                );
                let _ = std::process::Command::new("hyprctl").args(["eval", &lua_rule]).output();
                tokio::time::sleep(std::time::Duration::from_millis(35)).await;
                let cfg = crate::notification_panel::notification_panel_window_config();
                let new_id = Platform::get().launch_window(cfg).await;
                if let Ok(mut g) = notif_panel_ctx.0.lock() {
                    *g = Some(new_id);
                }
                tokio::spawn(async move {
                    for delay in [30, 80] {
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        let lua = format!(
                            r#"
                            for _, w in ipairs(hl.get_windows()) do
                                if w.class == 'overlay-notifications' or w.title == 'notifications-panel' then
                                    local addr = 'address:' .. tostring(w.address)
                                    hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                    hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                    hl.dispatch(hl.dsp.window.move({{ window = addr, x = {panel_x}, y = {panel_y} }}))
                                    hl.dispatch(hl.dsp.window.resize({{ window = addr, x = 400, y = 520, relative = false }}))
                                    return true
                                end
                            end
                            return false
                            "#,
                            panel_x = panel_x,
                            panel_y = panel_y,
                        );
                        if let Ok(out) = std::process::Command::new("hyprctl").args(["eval", &lua]).output() {
                            let s = String::from_utf8_lossy(&out.stdout);
                            if s.contains("true") {
                                break;
                            }
                        }
                    }
                });
            });
        })
    };

    let wired = use_state(|| Option::<WiredInfo>::None);
    let battery_pct = use_state(|| 0u8);
    let battery_status = use_state(|| "Unknown".to_string());
    let notifications = use_state(Vec::<state::Notification>::new);
    let overlay_toasts = use_state(Vec::<state::Notification>::new);
    let workspaces = use_state(|| system::get_workspaces());
    let active_workspaces = use_state(|| system::get_active_workspaces());

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
        load_initial_batch(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
        subscribe_live(wifi, bt, volume, muted, brightness, dnd, connected, topbar_settings, theme_state);
        subscribe_notifications_live(notifications);
        let mut ws = workspaces;
        let mut aw = active_workspaces;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let new_ws = tokio::task::spawn_blocking(system::get_workspaces).await.unwrap_or_default();
                let new_aw = tokio::task::spawn_blocking(system::get_active_workspaces).await.unwrap_or_default();
                if new_ws != *ws.read() {
                    ws.set(new_ws);
                }
                if new_aw != *aw.read() {
                    aw.set(new_aw);
                }
            }
        });

        let mut ot_state = overlay_toasts;
        spawn(async move {
            loop {
                if let Ok(mut rx) = ipsea::notifications::subscribe_channel(ipsea::notifications::NOTIFICATIONS_SOCKET_NAME)
                {
                    while let Some(evt) = rx.recv().await {
                        match evt {
                            ipsea::notifications::NotificationEvent::Show(notif) => {
                                let id = notif.id;
                                let timeout = if notif.timeout > 0 { notif.timeout as u64 } else { 5000 };

                                let mut current = ot_state.read().clone();
                                if let Some(idx) = current.iter().position(|n| n.id == id) {
                                    current[idx] = notif.clone();
                                } else {
                                    current.push(notif.clone());
                                }
                                ot_state.set(current);

                                let mut ot_dismiss = ot_state;
                                spawn(async move {
                                    tokio::time::sleep(std::time::Duration::from_millis(timeout)).await;
                                    let mut list = ot_dismiss.read().clone();
                                    if let Some(pos) = list.iter().position(|n| n.id == id) {
                                        list.remove(pos);
                                        ot_dismiss.set(list);
                                    }
                                });
                            }
                            ipsea::notifications::NotificationEvent::Close(id) => {
                                let mut current = ot_state.read().clone();
                                if let Some(pos) = current.iter().position(|n| n.id == id) {
                                    current.remove(pos);
                                    ot_state.set(current);
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        });
        let mut clk = clock;
        let is_24h = use_24h;
        let mut wired_state = wired;
        let mut pct_state = battery_pct;
        let mut status_state = battery_status;
        let mut notif_state = notifications;
        spawn(async move {
            let initial_notifs = tokio::task::spawn_blocking(fetch_notifications_blocking).await.unwrap_or_default();
            notif_state.set_if_modified(initial_notifs);
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
    let cb_hover = use_state(|| false);
    let ss_hover = use_state(|| false);
    let notif_hover = use_state(|| false);
    let has_battery = *battery_status.read() != "Unknown" && !battery_status.read().is_empty();
    let t = {
        let tb = topbar_settings.read().clone();
        let base = match tb.theme.as_str() {
            "light" => ui::LIGHT_THEME,
            "dark" => ui::DARK_THEME,
            _ => t,
        };
        let acc = ui::ACCENTS.iter().find(|a| a.name == t.accent_name).copied().unwrap_or(ui::ACCENT_INDIGO);
        base.with_accent(acc)
    };

    let notifs_list = notifications.read().clone();
    let notif_count = notifs_list.len();

    let active_ws_id = active_workspaces.read().get(&mon_name).copied().unwrap_or(1);
    let tb_snapshot = topbar_settings.read().clone();
    let is_light_theme = t.mode == ui::ThemeMode::Light;
    let tb_text_kind = tb_snapshot.effective_text_color(is_light_theme).to_string();
    let tb_text_size = tb_snapshot.text_size.clamp(10.0, 18.0) as f32;
    let resolve_base = |kind: &str| match kind {
        "Accent" => t.accent,
        "Muted" => t.text_dim,
        _ => t.text,
    };

    let ws_data = workspaces.read().clone();
    let mut visible_ws: Vec<system::WorkspaceInfo> = ws_data
        .into_iter()
        .filter(|w| w.monitor.is_empty() || w.monitor == mon_name)
        .filter(|w| w.windows > 0 || w.id == active_ws_id)
        .collect();
    if visible_ws.is_empty() {
        visible_ws.push(system::WorkspaceInfo {
            id: active_ws_id,
            name: active_ws_id.to_string(),
            monitor: mon_name.clone(),
            windows: 0,
        });
    }
    visible_ws.sort_by_key(|w| w.id);
    visible_ws.dedup_by_key(|w| w.id);
    visible_ws.truncate(10);
    let workspaces_ui = rect()
        .height(Size::px(28.))
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .spacing(4.)
        .children(visible_ws.into_iter().map(|ws| {
            let w_id = ws.id;
            let name = ws.name.clone();
            let is_active = w_id == active_ws_id;
            let bg = if is_active { t.accent } else { Color::TRANSPARENT };
            let label_color = if is_active { Color::WHITE } else { resolve_base(&tb_text_kind) };
            rect()
                .width(Size::px(28.))
                .height(Size::px(28.))
                .corner_radius(8.)
                .background(bg)
                .border(Border::new().width(1.).fill(if is_active { t.accent } else { Color::TRANSPARENT }))
                .center()
                .cursor(CursorIcon::Pointer)
                .on_press(move |_| {
                    let id_str = w_id.to_string();
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("hyprctl").args(["dispatch", "workspace", &id_str]).output();
                    });
                })
                .child(
                    label()
                        .font_size(tb_text_size)
                        .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::SEMI_BOLD })
                        .color(label_color)
                        .text(name),
                )
        }));

    let show_net = (wired_on && tb_snapshot.show_wired) || (!wired_on && tb_snapshot.show_wifi);
    let show_bt_icon = tb_snapshot.show_bluetooth;
    let show_sound_icon = tb_snapshot.show_sound;
    let show_batt = has_battery && tb_snapshot.show_battery;
    let show_notif_icon = tb_snapshot.show_notifications;
    let tb_icon_size = tb_snapshot.icon_size.clamp(12., 20.) as f32;
    let tb_icon_stroke = tb_snapshot.icon_stroke.clamp(1.0, 3.0) as f32;
    let tb_base = resolve_base(&tb_text_kind);
    let status = rect()
        .height(Size::px(36.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .padding((4., 16.))
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(workspaces_ui)
        .child(rect().width(Size::flex(1.)))
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(12.)
                .content(Content::Flex)
                .child({
                    let mut h = cb_hover;
                    let mut h2 = cb_hover;
                    let is_h = *cb_hover.read();
                    let sz = tb_icon_size;
                    let sw = tb_icon_stroke;
                    let base = tb_base;
                    rect()
                        .width(Size::px(28.))
                        .height(Size::px(28.))
                        .corner_radius(8.)
                        .background(if is_h { t.accent } else { Color::TRANSPARENT })
                        .border(Border::new().width(1.).fill(if is_h { t.accent } else { Color::TRANSPARENT }))
                        .center()
                        .cursor(CursorIcon::Pointer)
                        .on_pointer_enter(move |_| h.set(true))
                        .on_pointer_leave(move |_| h2.set(false))
                        .on_press(|_| {
                            std::thread::spawn(|| {
                                let _ = ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::Clipboard);
                            });
                        })
                        .child(icon_stroked(COPY, sz - 1., sw, if is_h { Color::WHITE } else { base }))
                })
                .child({
                    let mut h = ss_hover;
                    let mut h2 = ss_hover;
                    let is_h = *ss_hover.read();
                    let sz = tb_icon_size;
                    let sw = tb_icon_stroke;
                    let base = tb_base;
                    rect()
                        .width(Size::px(28.))
                        .height(Size::px(28.))
                        .corner_radius(8.)
                        .background(if is_h { t.panel_raised } else { Color::TRANSPARENT })
                        .border(Border::new().width(1.).fill(if is_h { t.border } else { Color::TRANSPARENT }))
                        .center()
                        .cursor(CursorIcon::Pointer)
                        .on_pointer_enter(move |_| h.set(true))
                        .on_pointer_leave(move |_| h2.set(false))
                        .on_press(|_| {
                            std::thread::spawn(|| {
                                let _ = ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::Screenshot);
                            });
                        })
                        .child(icon_stroked(CAMERA, sz - 1., sw, base))
                })
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
                        .maybe(show_net, |el| {
                            let sz = tb_icon_size;
                            let sw = tb_icon_stroke;
                            let base = tb_base;
                            el.child(
                                icon_stroked(
                                    if wired_on { WIRED } else { WIFI },
                                    sz,
                                    sw,
                                    if wifi_on || wired_on { base } else { t.text_dim },
                                )
                                .into_element(),
                            )
                        })
                        .maybe(show_bt_icon, |el| {
                            let sz = tb_icon_size;
                            let sw = tb_icon_stroke;
                            let base = tb_base;
                            el.child(icon_stroked(BLUETOOTH, sz, sw, if bt_on { base } else { t.text_dim }).into_element())
                        })
                        .maybe(show_sound_icon, |el| {
                            let sz = tb_icon_size;
                            let sw = tb_icon_stroke;
                            let base = tb_base;
                            el.child(icon_stroked(SOUND, sz, sw, if is_muted { t.text_dim } else { base }).into_element())
                        })
                        .maybe(show_batt, |el| {
                            let base = tb_base;
                            let sz = (tb_icon_size - 1.) as f32;
                            let sw = tb_icon_stroke;
                            el.child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(4.)
                                    .content(Content::Flex)
                                    .child(icon_stroked(BATTERY, sz, sw, if bat_pct > 20 { base } else { t.accent_red }))
                                    .child(
                                        label()
                                            .font_size(12.)
                                            .color(if bat_pct > 20 { base } else { t.accent_red })
                                            .text(format!("{bat_pct}%")),
                                    ),
                            )
                        })
                })
                .maybe(show_notif_icon, |el| {
                    el.child({
                        let mut h = notif_hover;
                        let mut h2 = notif_hover;
                        let is_h = *notif_hover.read();
                        let sz = tb_icon_size;
                        let sw = tb_icon_stroke;
                        let base = tb_base;
                        let count = notif_count;
                        let has_notif = count > 0;
                        rect()
                            .width(Size::px(28.))
                            .height(Size::px(28.))
                            .corner_radius(8.)
                            .background(if is_h {
                                t.panel_raised
                            } else if has_notif {
                                t.panel
                            } else {
                                Color::TRANSPARENT
                            })
                            .border(Border::new().width(1.).fill(if is_h || has_notif {
                                t.border
                            } else {
                                Color::TRANSPARENT
                            }))
                            .center()
                            .cursor(CursorIcon::Pointer)
                            .on_pointer_enter(move |_| h.set(true))
                            .on_pointer_leave(move |_| h2.set(false))
                            .on_press({
                                let tn = toggle_notifications.clone();
                                move |_| tn()
                            })
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(2.)
                                    .content(Content::Flex)
                                    .child(icon_stroked(NOTIFICATIONS, sz - 1., sw, if has_notif { t.accent } else { base }))
                                    .maybe(has_notif, |el| {
                                        el.child(
                                            rect()
                                                .padding((1., 4.))
                                                .corner_radius(999.)
                                                .background(t.accent_red)
                                                .center()
                                                .child(
                                                    label()
                                                        .font_size(9.)
                                                        .font_weight(FontWeight::BOLD)
                                                        .color(Color::WHITE)
                                                        .text(if count > 99 {
                                                            "99+".to_string()
                                                        } else {
                                                            count.to_string()
                                                        }),
                                                ),
                                        )
                                    }),
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
                                .font_size(tb_text_size)
                                .font_weight(FontWeight::SEMI_BOLD)
                                .color(tb_base)
                                .text(clock_str),
                        ),
                ),
        );

    rect()
        .width(Size::fill())
        .height(Size::px(36.))
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(status)
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
    let modal_rule = r#"
        hl.window_rule({ name = "finick-modal", match = { class = "^(overlay-modal)$" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 0 })
        hl.window_rule({ name = "finick-modal-compat", match = { class = "^(overlay-modal)$", title = "^(overlay-modal)$" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, no_blur = true })
        hl.window_rule({ name = "finick-clipboard", match = { class = "^(overlay-clipboard)$" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 })
        hl.window_rule({ name = "finick-clipboard-title", match = { title = "^(clipboard)$" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 })
        hl.window_rule({ name = "finick-clipboard-any", match = { class = "overlay-clipboard" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, rounding = 20 })
        hl.window_rule({ name = "finick-screenshot", match = { class = "^(overlay-screenshot)$" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 })
        hl.window_rule({ name = "finick-screenshot-title", match = { title = "^(screenshot)$" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 })
        hl.window_rule({ name = "finick-screenshot-any", match = { class = "overlay-screenshot" }, float = true, pin = true, border_size = 0, no_shadow = true, no_anim = true, rounding = 20 })
    "#;
    let _ = std::process::Command::new("hyprctl").args(["eval", modal_rule]).output();
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
        hl.window_rule({
            name = "finick-notifications",
            match = { class = "^(overlay-notifications)$" },
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
            size = {{ {width}, 36 }},
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
            size = {{ {width}, 36 }},
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

/// Binds SUPER+L to the finick locker if no binding already uses it.
/// Runs in a background thread so a missing hyprctl never blocks startup.
pub fn ensure_super_l_lock_binding() {
    std::thread::spawn(|| {
        let already_bound = std::process::Command::new("hyprctl")
            .args(["binds", "-j"])
            .output()
            .ok()
            .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
            .and_then(|v| v.as_array().cloned())
            .map(|binds| {
                binds.iter().any(|b| {
                    let key = b["key"].as_str().unwrap_or("").to_lowercase();
                    let modmask = b["modmask"].as_i64().unwrap_or(0);
                    (key == "l") && (modmask & 64 != 0)
                })
            })
            .unwrap_or(false);
        if !already_bound {
            let locker_cmd =
                system::locker_binary().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "locker".to_string());
            let bind = format!("SUPER, L, exec, {locker_cmd}");
            let out = std::process::Command::new("hyprctl").args(["keyword", "bind", &bind]).output();
            if let Err(e) = out {
                eprintln!("[finick] failed to bind SUPER+L to locker: {e}");
            }
        }
    });
}

pub fn register_hyprland_rules() {
    apply_hyprland_base_rules();
    for mon in get_monitor_states() {
        apply_hyprland_monitor_rules(&mon);
    }
}

fn make_topbar_window_config(mon: &MonitorState) -> WindowConfig {
    let mon_name = mon.name.clone();
    let title: &'static str = Box::leak(format!("overlay-{}", mon.name).into_boxed_str());
    WindowConfig::new(move || topbar_app_for_monitor(mon_name.clone()))
        .with_title(title)
        .with_app_id("overlay")
        .with_size(mon.width, 36.)
        .with_decorations(false)
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|_ctx, _wid| {
            // Prevent exit shortcuts (like SUPER+Q / killactive) from closing the desktop overlay
            CloseDecision::KeepOpen
        })
}

fn get_socket2_path() -> Option<std::path::PathBuf> {
    if let Ok(his) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        if let Ok(xdg_runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = std::path::PathBuf::from(&xdg_runtime_dir).join("hypr").join(&his).join(".socket2.sock");
            if path.exists() {
                return Some(path);
            }
        }
        let tmp_path = std::path::PathBuf::from("/tmp/hypr").join(&his).join(".socket2.sock");
        if tmp_path.exists() {
            return Some(tmp_path);
        }
    }

    // Fallback: scan XDG_RUNTIME_DIR/hypr for the newest socket2.sock
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        let hypr_dir = std::path::PathBuf::from(xdg).join("hypr");
        if let Ok(entries) = std::fs::read_dir(hypr_dir) {
            let mut candidates = Vec::new();
            for entry in entries.flatten() {
                let sock = entry.path().join(".socket2.sock");
                if sock.exists() {
                    if let Ok(meta) = sock.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            candidates.push((mtime, sock));
                        }
                    }
                }
            }
            candidates.sort_by(|a, b| b.0.cmp(&a.0));
            if let Some((_, newest)) = candidates.into_iter().next() {
                return Some(newest);
            }
        }
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

    // Re-sync any window IDs from ctx.windows() that might have been added or missed
    let active_windows = proxy
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

    for (name, wid) in &active_windows {
        if !known_monitors.contains_key(name) {
            if let Some(mon) = current_monitors.iter().find(|m| &m.name == name) {
                known_monitors.insert(name.clone(), (*wid, mon.clone()));
            }
        }
    }

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
                    known_monitors.insert(mon.name.clone(), (wid, mon.clone()));
                }

                // Ensure it is positioned and sized in Hyprland
                let name = mon.name.clone();
                let width = mon.width as i32;
                let x = mon.x;
                let y = mon.y;
                tokio::spawn(async move {
                    for delay in [50, 150, 300, 600] {
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        let lua = format!(
                            r#"
                            for _, w in ipairs(hl.get_windows()) do
                                if w.title == 'overlay-{name}' or w.title == 'topbar-{name}' then
                                    local addr = 'address:' .. tostring(w.address)
                                    hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                    hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                    hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {width}, y = 36 }}))
                                    hl.dispatch(hl.dsp.window.move({{ window = addr, x = {x}, y = {y} }}))
                                end
                            end
                            "#,
                            name = name,
                            width = width,
                            x = x,
                            y = y,
                        );
                        let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
                    }
                });
            }
            Some((wid, prev_mon)) => {
                // Check if resolution or geometry changed!
                if prev_mon.width != mon.width || prev_mon.height != mon.height || prev_mon.x != mon.x || prev_mon.y != mon.y
                {
                    let wid = *wid;
                    let new_width = mon.width;
                    let name = mon.name.clone();
                    let width = mon.width as i32;
                    let x = mon.x;
                    let y = mon.y;

                    apply_hyprland_monitor_rules(&mon);

                    // Resize and move window directly in Hyprland
                    let lua = format!(
                        r#"
                        for _, w in ipairs(hl.get_windows()) do
                            if w.title == 'overlay-{name}' or w.title == 'topbar-{name}' then
                                local addr = 'address:' .. tostring(w.address)
                                hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {width}, y = 36 }}))
                                hl.dispatch(hl.dsp.window.move({{ window = addr, x = {x}, y = {y} }}))
                            end
                        end
                        "#,
                        name = name,
                        width = width,
                        x = x,
                        y = y,
                    );
                    let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();

                    // Request resize in winit/Freya
                    let _ = proxy
                        .post_callback(move |ctx| {
                            if let Some(app) = ctx.windows_mut().get_mut(&wid) {
                                let _ = app.window_mut().request_inner_size(LogicalSize::new(new_width, 36.0));
                                app.window().request_redraw();
                            }
                        })
                        .await;

                    *prev_mon = mon;
                }
            }
        }
    }

    // Ensure all overlay windows in Hyprland match monitor bounds, are floated and pinned
    let audit_lua = r#"
        for _, w in ipairs(hl.get_windows()) do
            for _, mon in ipairs(hl.get_monitors()) do
                if w.title == 'overlay-' .. mon.name or w.title == 'topbar-' .. mon.name then
                    local addr = 'address:' .. tostring(w.address)
                    if not w.floating or not w.pinned or math.floor(w.size.x) ~= math.floor(mon.width) or math.floor(w.size.y) ~= 36 or math.floor(w.at.x) ~= math.floor(mon.x) or math.floor(w.at.y) ~= math.floor(mon.y) then
                        hl.dispatch(hl.dsp.window.float({ window = addr }))
                        hl.dispatch(hl.dsp.window.pin({ window = addr }))
                        hl.dispatch(hl.dsp.window.resize({ window = addr, x = math.floor(mon.width), y = 36 }))
                        hl.dispatch(hl.dsp.window.move({ window = addr, x = math.floor(mon.x), y = math.floor(mon.y) }))
                    end
                end
            end
        end
    "#;
    let _ = std::process::Command::new("hyprctl").args(["eval", audit_lua]).output();
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

    // Ensure initial overlay windows are accurately positioned and sized in Hyprland
    for mon in &initial_monitors {
        let name = &mon.name;
        let width = mon.width as i32;
        let x = mon.x;
        let y = mon.y;
        let lua = format!(
            r#"
            for _, w in ipairs(hl.get_windows()) do
                if w.title == 'overlay-{name}' or w.title == 'topbar-{name}' then
                    local addr = 'address:' .. tostring(w.address)
                    hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                    hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                    hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {width}, y = 36 }}))
                    hl.dispatch(hl.dsp.window.move({{ window = addr, x = {x}, y = {y} }}))
                end
            end
            "#,
            name = name,
            width = width,
            x = x,
            y = y,
        );
        let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
    }

    let mut socket_reader = None;
    if let Some(path) = get_socket2_path() {
        if let Ok(stream) = tokio::net::UnixStream::connect(path).await {
            use tokio::io::AsyncBufReadExt;
            socket_reader = Some(tokio::io::BufReader::new(stream).lines());
        }
    }

    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(500));

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
            let mut sigquit = signal(SignalKind::quit()).ok();

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
                _ = async {
                    if let Some(s) = sigquit.as_mut() { s.recv().await; } else { std::future::pending().await }
                } => {},
            }

            clear_hyprland_reserved_space();
            std::process::exit(0);
        });
    }

    apply_hyprland_base_rules();
    crate::clipboard_manager::start_clipboard_service();
    ensure_super_l_lock_binding();
    let initial_monitors = get_monitor_states();
    for mon in &initial_monitors {
        apply_hyprland_monitor_rules(mon);
    }

    let mut launch_config = LaunchConfig::new()
        .with_exit_on_close(false)
        .with_global(PanelWindowId::default())
        .with_global(NotificationPanelWindowId::default())
        .with_global(ClipboardWindowId::default())
        .with_global(ScreenshotWindowId::default())
        .with_global(modal::ModalState::default());

    for mon in &initial_monitors {
        launch_config = launch_config.with_window(make_topbar_window_config(mon));
    }

    let monitors_for_listener = initial_monitors.clone();
    launch_config = launch_config.with_future(move |proxy| async move {
        let proxy_clone = proxy.clone();
        let handle = tokio::runtime::Handle::current();
        let handle2 = handle.clone();
        tokio::spawn(async move {
            let handle = handle2;
            let _ = ipsea::start_server(
                ipsea::modals::MODALS_SOCKET_NAME,
                move |req: ipsea::modals::ModalRequest, tx| {
                    if let ipsea::modals::ModalRequest::CloseAll = req {
                        let pc = proxy_clone.clone();
                        let h = handle.clone();
                        h.spawn(async move {
                            let _ = pc.post_callback(|ctx| {
                                let ids: Vec<winit::window::WindowId> = ctx.windows().keys().copied().collect();
                                for id in ids {
                                    if let Some(app) = ctx.windows().get(&id) {
                                        let title = app.window().title();
                                        if title == "overlay-modal" || title == "clipboard" || title == "screenshot" || title == "notifications-panel" || title == "control-panel" {
                                            ctx.windows_mut().remove(&id);
                                        }
                                    }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::PanelWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::NotificationPanelWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(modal_ctx) = ctx.global_contexts.try_get_context::<modal::ModalState>() {
                                    if let Ok(mut g) = modal_ctx.req.lock() { *g = None; }
                                }
                            }).await;
                        });
                        let _ = std::process::Command::new("hyprctl").args(["dispatch", "closewindow", "class:^(overlay-modal)$"]).output();
                        let _ = std::process::Command::new("hyprctl").args(["dispatch", "closewindow", "class:^(overlay-clipboard)$"]).output();
                        let _ = std::process::Command::new("hyprctl").args(["dispatch", "closewindow", "class:^(overlay-screenshot)$"]).output();
                        let _ = std::process::Command::new("hyprctl").args(["dispatch", "closewindow", "class:^(overlay-notifications)$"]).output();
                        let _ = std::process::Command::new("hyprctl").args(["dispatch", "closewindow", "class:^(overlay-panel)$"]).output();
                    } else if let ipsea::modals::ModalRequest::Clipboard = req {
                        let proxy = proxy_clone.clone();
                        let h = handle.clone();
                        let tx2 = tx.clone();
                        h.spawn(async move {
                            let was_open = proxy.post_callback(|ctx| {
                                let mut was_open = false;
                                if let Some(cw) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                    if let Ok(mut g) = cw.0.lock() {
                                        if let Some(id) = g.take() {
                                            ctx.windows_mut().remove(&id);
                                            was_open = true;
                                        }
                                    }
                                }
                                was_open
                            }).await.unwrap_or(false);
                            if was_open {
                                let _ = std::process::Command::new("hyprctl")
                                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                                    .output();
                                let _ = tx2.send(ipsea::modals::ModalResponse::Success { data: None });
                                return;
                            }
                            let _ = std::process::Command::new("hyprctl")
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-panel)$' }))"])
                                .output();
                            let _ = std::process::Command::new("hyprctl")
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-notifications)$' }))"])
                                .output();
                            let _ = std::process::Command::new("hyprctl")
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                                .output();
                            let _ = proxy.post_callback(|ctx| {
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::PanelWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::NotificationPanelWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                let ids: Vec<winit::window::WindowId> = ctx.windows().keys().copied().collect();
                                for id in ids {
                                    if let Some(app) = ctx.windows().get(&id) {
                                        let title = app.window().title();
                                        if title == "screenshot" || title == "control-panel" || title == "notifications-panel" {
                                            ctx.windows_mut().remove(&id);
                                        }
                                    }
                                }
                            }).await;
                            let mon = get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
                            let w = 520;
                            let hgt = 380;
                            let px = mon.x + mon.width as i32 - w - 12;
                            let py = mon.y + 44;
                            let mon_name = mon.name.clone();
                            let _ = std::process::Command::new("hyprctl").args(["eval", &format!(r#"hl.window_rule({{ name = "finick-clipboard-live-{mon_name}", match = {{ class = "^(overlay-clipboard)$" }}, monitor = "{mon_name}", float = true, pin = true, move = {{ {px}, {py} }}, size = {{ {w}, {hgt} }}, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 }})"#, mon_name=mon_name, px=px, py=py, w=w, hgt=hgt)]).output();
                            tokio::time::sleep(std::time::Duration::from_millis(35)).await;
                            let _ = proxy.post_callback(move |ctx| {
                                let cfg = crate::clipboard_manager::clipboard_window_config();
                                let wid = ctx.launch_window(cfg);
                                if let Some(cw) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                    if let Ok(mut g) = cw.0.lock() { *g = Some(wid); }
                                }
                            }).await;
                            let _ = tx2.send(ipsea::modals::ModalResponse::Success { data: None });
                            tokio::spawn(async move {
                                for delay in [30, 80] {
                                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                                    let lua = format!(
                                        r#"
                                        for _, w in ipairs(hl.get_windows()) do
                                            if w.class == 'overlay-clipboard' or w.title == 'clipboard' then
                                                local addr = 'address:' .. tostring(w.address)
                                                hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                                hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                                hl.dispatch(hl.dsp.window.move({{ window = addr, x = {px}, y = {py} }}))
                                                hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {w}, y = {hgt}, relative = false }}))
                                                break
                                            end
                                        end
                                        "#,
                                        px = px, py = py, w = w, hgt = hgt,
                                    );
                                    let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
                                }
                            });
                        });
                    } else if let ipsea::modals::ModalRequest::Screenshot = req {
                        let proxy = proxy_clone.clone();
                        let h = handle.clone();
                        let tx2 = tx.clone();
                        h.spawn(async move {
                            let was_open = proxy.post_callback(|ctx| {
                                let mut was_open = false;
                                if let Some(sw) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                    if let Ok(mut g) = sw.0.lock() {
                                        if let Some(id) = g.take() {
                                            ctx.windows_mut().remove(&id);
                                            was_open = true;
                                        }
                                    }
                                }
                                was_open
                            }).await.unwrap_or(false);
                            if was_open {
                                let _ = std::process::Command::new("hyprctl")
                                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                                    .output();
                                let _ = tx2.send(ipsea::modals::ModalResponse::Success { data: None });
                                return;
                            }
                            let _ = std::process::Command::new("hyprctl")
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-panel)$' }))"])
                                .output();
                            let _ = std::process::Command::new("hyprctl")
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-notifications)$' }))"])
                                .output();
                            let _ = std::process::Command::new("hyprctl")
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                                .output();
                            let _ = proxy.post_callback(|ctx| {
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::PanelWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::NotificationPanelWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                let ids: Vec<winit::window::WindowId> = ctx.windows().keys().copied().collect();
                                for id in ids {
                                    if let Some(app) = ctx.windows().get(&id) {
                                        let title = app.window().title();
                                        if title == "clipboard" || title == "control-panel" || title == "notifications-panel" {
                                            ctx.windows_mut().remove(&id);
                                        }
                                    }
                                }
                            }).await;
                            let mon = get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
                            let w = 500;
                            let hgt = 264;
                            let px = mon.x + mon.width as i32 - w - 12;
                            let py = mon.y + 44;
                            let mon_name = mon.name.clone();
                            let _ = std::process::Command::new("hyprctl").args(["eval", &format!(r#"hl.window_rule({{ name = "finick-screenshot-live-{mon_name}", match = {{ class = "^(overlay-screenshot)$" }}, monitor = "{mon_name}", float = true, pin = true, move = {{ {px}, {py} }}, size = {{ {w}, {hgt} }}, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 }})"#, mon_name=mon_name, px=px, py=py, w=w, hgt=hgt)]).output();
                            tokio::time::sleep(std::time::Duration::from_millis(35)).await;
                            let _ = proxy.post_callback(move |ctx| {
                                let cfg = crate::screenshot::screenshot_window_config();
                                let wid = ctx.launch_window(cfg);
                                if let Some(sw) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                    if let Ok(mut g) = sw.0.lock() { *g = Some(wid); }
                                }
                            }).await;
                            let _ = tx2.send(ipsea::modals::ModalResponse::Success { data: None });
                            tokio::spawn(async move {
                                for delay in [30, 80] {
                                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                                    let lua = format!(
                                        r#"
                                        for _, w in ipairs(hl.get_windows()) do
                                            if w.class == 'overlay-screenshot' or w.title == 'screenshot' then
                                                local addr = 'address:' .. tostring(w.address)
                                                hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                                hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                                hl.dispatch(hl.dsp.window.move({{ window = addr, x = {px}, y = {py} }}))
                                                hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {w}, y = {hgt}, relative = false }}))
                                                break
                                            end
                                        end
                                        "#,
                                        px = px, py = py, w = w, hgt = hgt,
                                    );
                                    let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
                                }
                            });
                        });
                    } else {
                        let req_clone = req.clone();
                        let proxy = proxy_clone.clone();
                        let h = handle.clone();
                        let mon = get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
                        let mw = mon.width;
                        let mh = mon.height;
                        let mx = mon.x;
                        let my = mon.y;
                        let mon_name = mon.name.clone();
                        h.spawn(async move {
                            let _ = std::process::Command::new("hyprctl").args(["eval", &format!(r#"hl.window_rule({{ name = "finick-modal-live-{mon_name}", match = {{ class = "^(overlay-modal)$" }}, monitor = "{mon_name}", float = true, pin = true, move = {{ {mx}, {my} }}, size = {{ {mw}, {mh} }}, border_size = 0, no_shadow = true, no_anim = true }})"#, mon_name=mon_name, mx=mx, my=my, mw=mw as i32, mh=mh as i32)]).output();
                            tokio::time::sleep(std::time::Duration::from_millis(35)).await;
                            let _ = proxy.post_callback(move |ctx| {
                                let window_id = ctx.launch_window(
                                    WindowConfig::new(modal::fullscreen_modal_app)
                                        .with_title("overlay-modal")
                                        .with_app_id("overlay-modal")
                                        .with_size(mw, mh)
                                        .with_decorations(false)
                                        .with_transparency(true)
                                        .with_background(Color::TRANSPARENT)
                                );
                                if let Some(modal_ctx) = ctx.global_contexts.try_get_context::<modal::ModalState>() {
                                    if let Ok(mut g) = modal_ctx.req.lock() { *g = Some((req_clone.clone(), tx.clone(), window_id)); }
                                }
                            }).await;
                            tokio::spawn(async move {
                                for delay in [30, 80, 180, 350] {
                                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                                    let lua = format!(
                                        r#"
                                        for _, w in ipairs(hl.get_windows()) do
                                            if w.class == 'overlay-modal' or w.title == 'overlay-modal' then
                                                local addr = 'address:' .. tostring(w.address)
                                                hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                                hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                                hl.dispatch(hl.dsp.window.move({{ window = addr, x = {mx}, y = {my} }}))
                                                hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {mw}, y = {mh}, relative = false }}))
                                                hl.dispatch(hl.dsp.window.focus({{ window = addr }}))
                                                return true
                                            end
                                        end
                                        return false
                                        "#,
                                        mx = mx,
                                        my = my,
                                        mw = mw as i32,
                                        mh = mh as i32,
                                    );
                                    let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
                                    let _ = std::process::Command::new("hyprctl").args(["dispatch", "bringactivetotop"]).output();
                                }
                            });
                        });
                    }
                }
            );
        });
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
        let _ = fetch_notifications_blocking();
    }

    #[test]
    fn test_panel_config_sizing() {
        let cfg = panel_window_config();
        let _ = cfg;
    }

    #[test]
    fn test_compute_panel_height_empty_vs_full() {
        use crate::panel::compute_panel_height;
        let empty_h = compute_panel_height(
            true,  // wifi_on
            false, // has_connected_wifi
            0,     // available_wifi_count
            false, // has_wired
            true,  // bt_on
            0,     // connected_bt_count
            0,     // other_bt_count
            false, // has_battery
        );
        assert_eq!(empty_h, 700);
        let full_h = compute_panel_height(true, true, 5, false, true, 1, 2, false);
        assert_eq!(full_h, 700);
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
mod modal;
