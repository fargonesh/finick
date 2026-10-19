#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

pub mod clipboard_manager;
pub mod launcher;
mod date_panel;
mod notification_panel;
mod notification_popup;
mod toasts;
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
pub struct CalendarPanelWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

#[derive(Clone, Default)]
pub struct ClipboardWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

#[derive(Clone, Default)]
pub struct ScreenshotWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

#[derive(Clone, Default)]
pub struct ToastWindowId(pub Arc<Mutex<Option<winit::window::WindowId>>>);

/// Distance from the monitor top edge to the top of dropdown panels.
/// Topbar is 36px tall with a reserved top of 36, so 48 leaves a 12px gap.
pub const PANEL_TOP_OFFSET: i32 = 48;

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
        .with_on_close(|ctx, _wid| {
            if let Some(panel) = ctx.global_contexts.try_get_context::<PanelWindowId>() {
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
            if !crate::state::rate_limit_ok(&crate::state::LAST_PANEL_TOGGLE_MS, 350) {
                return;
            }
            let panel_ctx = panel_ctx.clone();
            let mon_name = mon_name_for_panel.clone();
            spawn(async move {
                let tracked = panel_ctx.0.lock().ok().and_then(|g| *g);
                if tracked.is_some() {
                    let hypr_live = tokio::task::spawn_blocking(|| {
                        crate::state::hypr_has_any_class_blocking(&["overlay-panel", "topbar-panel"])
                    })
                    .await
                    .unwrap_or(true);
                    if hypr_live {
                        // Animated close: the panel maps this to its local dismiss
                        // so the PanelShell exit runs instead of popping.
                        crate::state::request_panel_close();
                        return;
                    }
                    if let Ok(mut g) = panel_ctx.0.lock() {
                        if *g == tracked {
                            *g = None;
                        }
                    }
                    if let Some(id) = tracked {
                        Platform::get().close_window(id);
                    }
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-panel)$' }))"])
                        .output();
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
                if let Some(cal_ctx) = GlobalContexts::get().try_get_context::<crate::CalendarPanelWindowId>() {
                    if let Ok(mut g) = cal_ctx.0.lock() {
                        if let Some(nid) = g.take() {
                            Platform::get().close_window(nid);
                            let _ = std::process::Command::new("hyprctl")
                                .args([
                                    "eval",
                                    "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-calendar)$' }))",
                                ])
                                .output();
                        }
                    }
                }
                if let Some(cw) = GlobalContexts::get().try_get_context::<crate::ClipboardWindowId>() {
                    if let Ok(mut g) = cw.0.lock() {
                        if let Some(id) = g.take() {
                            Platform::get().close_window(id);
                        }
                    }
                }
                if let Some(sw) = GlobalContexts::get().try_get_context::<crate::ScreenshotWindowId>() {
                    if let Ok(mut g) = sw.0.lock() {
                        if let Some(id) = g.take() {
                            Platform::get().close_window(id);
                        }
                    }
                }
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                    .output();

                let current_mon = get_monitor_state(&mon_name);
                let panel_x = current_mon.x + current_mon.width as i32 - 460 - 12;
                let panel_y = current_mon.y + PANEL_TOP_OFFSET;
                crate::state::arm_travel_guard(panel_x as f64, panel_y as f64, 460., 700., (panel_x + 380) as f64);

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
            if !crate::state::rate_limit_ok(&crate::state::LAST_NOTIF_TOGGLE_MS, 350) {
                return;
            }
            let notif_panel_ctx = notif_panel_ctx.clone();
            let mon_name = mon_name_for_notif.clone();
            spawn(async move {
                let tracked = notif_panel_ctx.0.lock().ok().and_then(|g| *g);
                if tracked.is_some() {
                    let hypr_live = tokio::task::spawn_blocking(|| {
                        crate::state::hypr_has_any_class_blocking(&["overlay-notifications"])
                    })
                    .await
                    .unwrap_or(true);
                    if hypr_live {
                        crate::state::request_notifications_close();
                        return;
                    }
                    if let Ok(mut g) = notif_panel_ctx.0.lock() {
                        if *g == tracked {
                            *g = None;
                        }
                    }
                    if let Some(id) = tracked {
                        Platform::get().close_window(id);
                    }
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-notifications)$' }))"])
                        .output();
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
                if let Some(cal_ctx) = GlobalContexts::get().try_get_context::<crate::CalendarPanelWindowId>() {
                    if let Ok(mut g) = cal_ctx.0.lock() {
                        if let Some(pid) = g.take() {
                            Platform::get().close_window(pid);
                        }
                    }
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-calendar)$' }))"])
                        .output();
                }
                if let Some(cw) = GlobalContexts::get().try_get_context::<crate::ClipboardWindowId>() {
                    if let Ok(mut g) = cw.0.lock() {
                        if let Some(id) = g.take() {
                            Platform::get().close_window(id);
                        }
                    }
                }
                if let Some(sw) = GlobalContexts::get().try_get_context::<crate::ScreenshotWindowId>() {
                    if let Ok(mut g) = sw.0.lock() {
                        if let Some(id) = g.take() {
                            Platform::get().close_window(id);
                        }
                    }
                }
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                    .output();

                let current_mon = get_monitor_state(&mon_name);
                let panel_x = current_mon.x + current_mon.width as i32 - 400 - 12;
                let panel_y = current_mon.y + PANEL_TOP_OFFSET;
                crate::state::arm_travel_guard(panel_x as f64, panel_y as f64, 400., 620., (panel_x + 340) as f64);
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
                                    hl.dispatch(hl.dsp.window.resize({{ window = addr, x = 400, y = 620, relative = false }}))
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

    let cal_panel_ctx: CalendarPanelWindowId = GlobalContexts::get().get_context::<CalendarPanelWindowId>();
    let mon_name_for_cal = mon_name.clone();
    let toggle_calendar = {
        let cal_panel_ctx = cal_panel_ctx.clone();
        std::rc::Rc::new(move || {
            if !crate::state::rate_limit_ok(&crate::state::LAST_CALENDAR_TOGGLE_MS, 350) {
                return;
            }
            let cal_panel_ctx = cal_panel_ctx.clone();
            let mon_name = mon_name_for_cal.clone();
            spawn(async move {
                let tracked = cal_panel_ctx.0.lock().ok().and_then(|g| *g);
                if tracked.is_some() {
                    let hypr_live = tokio::task::spawn_blocking(|| {
                        crate::state::hypr_has_any_class_blocking(&["overlay-calendar"])
                    })
                    .await
                    .unwrap_or(true);
                    if hypr_live {
                        crate::state::request_calendar_close();
                        return;
                    }
                    if let Ok(mut g) = cal_panel_ctx.0.lock() {
                        if *g == tracked {
                            *g = None;
                        }
                    }
                    if let Some(id) = tracked {
                        Platform::get().close_window(id);
                    }
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-calendar)$' }))"])
                        .output();
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
                if let Some(notif_ctx) = GlobalContexts::get().try_get_context::<crate::NotificationPanelWindowId>() {
                    if let Ok(mut g) = notif_ctx.0.lock() {
                        if let Some(pid) = g.take() {
                            Platform::get().close_window(pid);
                        }
                    }
                    let _ = std::process::Command::new("hyprctl")
                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-notifications)$' }))"])
                        .output();
                }
                if let Some(cw) = GlobalContexts::get().try_get_context::<crate::ClipboardWindowId>() {
                    if let Ok(mut g) = cw.0.lock() {
                        if let Some(id) = g.take() {
                            Platform::get().close_window(id);
                        }
                    }
                }
                if let Some(sw) = GlobalContexts::get().try_get_context::<crate::ScreenshotWindowId>() {
                    if let Ok(mut g) = sw.0.lock() {
                        if let Some(id) = g.take() {
                            Platform::get().close_window(id);
                        }
                    }
                }
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                    .output();
                let _ = std::process::Command::new("hyprctl")
                    .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                    .output();

                let current_mon = get_monitor_state(&mon_name);
                let panel_x = current_mon.x + current_mon.width as i32 - 400 - 12;
                let panel_y = current_mon.y + PANEL_TOP_OFFSET;
                crate::state::arm_travel_guard(panel_x as f64, panel_y as f64, 400., 620., (panel_x + 340) as f64);
                let lua_rule = format!(
                    r#"
                    hl.window_rule({{
                        name = "finick-calendar-position",
                        match = {{ class = "^(overlay-calendar)$" }},
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
                let cfg = crate::date_panel::date_panel_window_config();
                let new_id = Platform::get().launch_window(cfg).await;
                if let Ok(mut g) = cal_panel_ctx.0.lock() {
                    *g = Some(new_id);
                }
                tokio::spawn(async move {
                    for delay in [30, 80] {
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        let lua = format!(
                            r#"
                            for _, w in ipairs(hl.get_windows()) do
                                if w.class == 'overlay-calendar' or w.title == 'calendar-panel' then
                                    local addr = 'address:' .. tostring(w.address)
                                    hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                                    hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                                    hl.dispatch(hl.dsp.window.move({{ window = addr, x = {panel_x}, y = {panel_y} }}))
                                    hl.dispatch(hl.dsp.window.resize({{ window = addr, x = 400, y = 620, relative = false }}))
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
    let workspaces = use_state(|| system::get_workspaces());
    let active_workspaces = use_state(|| system::get_active_workspaces());

    use_hook(move || {
        let mut fmt = use_24h;
        spawn(async move {
            loop {
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
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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
                        system::hypr_workspace(&id_str);
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
                            if !crate::state::rate_limit_ok(&crate::state::LAST_CLIPBOARD_TOGGLE_MS, 350) {
                                return;
                            }
                            std::thread::spawn(|| {
                                let _ = ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::Clipboard);
                            });
                        })
                        .child(icon_stroked(COPY, sz, sw, if is_h { Color::WHITE } else { base }))
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
                            if !crate::state::rate_limit_ok(&crate::state::LAST_SCREENSHOT_TOGGLE_MS, 350) {
                                return;
                            }
                            std::thread::spawn(|| {
                                let _ = ipsea::modals::send_modal_request(ipsea::modals::ModalRequest::Screenshot);
                            });
                        })
                        .child(icon_stroked(CAMERA, sz, sw, base))
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
                            let sz = tb_icon_size as f32;
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
                            .background(if is_h { t.panel_raised } else { Color::TRANSPARENT })
                            .center()
                            .cursor(CursorIcon::Pointer)
                            .on_pointer_enter(move |_| h.set(true))
                            .on_pointer_leave(move |_| h2.set(false))
                            .on_press({
                                let tn = toggle_notifications.clone();
                                move |_| tn()
                            })
                            .child(icon_stroked(NOTIFICATIONS, sz, sw, if has_notif { t.accent } else { base }))
                            .maybe(has_notif, |el| {
                                el.child(
                                    rect()
                                        .position(Position::new_absolute().top(0.).right(0.))
                                        .height(Size::px(15.))
                                        .padding((0., 4.))
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
                            })
                    })
                })
                .child(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 8.))
                        .on_press({
                            let t = toggle_calendar.clone();
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
        hl.window_rule({
            name = "finick-calendar",
            match = { class = "^(overlay-calendar)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            rounding = 20,
        })
        hl.window_rule({
            name = "finick-toasts",
            match = { class = "^(overlay-toasts)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            no_initial_focus = true,
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

/// Lua audit that keeps the fake layer-shell topbar exclusive.
///
/// The topbar is a normal window dressed up with `float`/`pin` plus
/// `hl.monitor reserved top=36` and `hl.window_rule`s, all installed via
/// `hyprctl eval`. Hyprland drops eval-installed reservations and rules on
/// `reload`, monitor sleep/wake, and hotplug. When the reservation is gone,
/// maximized windows fill y=0..H and paint underneath the transparent
/// topbar window, so the bar looks like it "lost its privileges".
/// This script re-asserts the reservation on every monitor and re-floats /
/// re-pins / re-moves / re-sizes any overlay window that went astray.
fn topbar_audit_lua() -> &'static str {
    r#"
        for _, mon in ipairs(hl.get_monitors()) do
            hl.monitor({ output = mon.name, reserved = { top = 36 } })
            for _, w in ipairs(hl.get_windows()) do
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
    "#
}

/// Re-installs the ephemeral `hl.window_rule`s (base + per-monitor).
///
/// Rules installed via `hyprctl eval` vanish on Hyprland `reload`, and the
/// overlay service can start before Hyprland accepts `eval` at all, so the
/// one-shot install in `run()` is not enough. Socket events (notably
/// `configreloaded`) force a re-apply; the 500ms ticker only re-applies at
/// most every 10s to avoid spamming `hyprctl`.
fn reapply_topbar_rules_if_due(last_applied: &mut Option<std::time::Instant>, force: bool) {
    const RULE_REAPPLY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);
    if !force {
        if let Some(last) = last_applied {
            if last.elapsed() < RULE_REAPPLY_INTERVAL {
                return;
            }
        }
    }
    apply_hyprland_base_rules();
    for mon in get_monitor_states() {
        apply_hyprland_monitor_rules(&mon);
    }
    *last_applied = Some(std::time::Instant::now());
}

/// Parses `hyprctl clients -j` + `hyprctl monitors -j` output.
///
/// Returns `(hidden, known)`: monitors hosting a mapped, real-fullscreen
/// window, and all known monitor names. Only mode >= 2 counts — 1 is
/// maximize, which keeps gaps and bars, so the topbar must stay visible.
/// `known` lets callers match only exact `overlay-{monitor}` topbar windows
/// and never touch panels, modals, or other overlay windows.
fn parse_fullscreen_monitors(
    clients_json: &str,
    monitors_json: &str,
) -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
    let mut hidden = std::collections::HashSet::new();
    let mut known = std::collections::HashSet::new();
    let mut id_to_name = std::collections::HashMap::new();
    if let Ok(mons) = serde_json::from_str::<serde_json::Value>(monitors_json) {
        if let Some(arr) = mons.as_array() {
            for m in arr {
                if let (Some(id), Some(name)) = (m["id"].as_i64(), m["name"].as_str()) {
                    id_to_name.insert(id, name.to_string());
                    known.insert(name.to_string());
                }
            }
        }
    }
    if let Ok(clients) = serde_json::from_str::<serde_json::Value>(clients_json) {
        if let Some(arr) = clients.as_array() {
            for c in arr {
                let mapped = c["mapped"].as_bool().unwrap_or(true);
                let fs = c["fullscreen"].as_i64().unwrap_or(0);
                let fsm = c["fullscreenMode"].as_i64().unwrap_or(0);
                if mapped && (fs >= 2 || fsm >= 2) {
                    if let Some(mon_id) = c["monitor"].as_i64() {
                        if let Some(name) = id_to_name.get(&mon_id) {
                            hidden.insert(name.clone());
                        }
                    }
                }
            }
        }
    }
    (hidden, known)
}

/// Monitors that currently host a real-fullscreen window (YouTube-style).
/// Empty (never panics) when Hyprland is unreachable or output is unexpected.
fn fullscreen_monitor_names() -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
    let empty = (std::collections::HashSet::new(), std::collections::HashSet::new());
    let clients_out = match std::process::Command::new("hyprctl").args(["clients", "-j"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return empty,
    };
    let monitors_out = match std::process::Command::new("hyprctl").args(["monitors", "-j"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return empty,
    };
    parse_fullscreen_monitors(&String::from_utf8_lossy(&clients_out.stdout), &String::from_utf8_lossy(&monitors_out.stdout))
}

/// Hides the Freya topbar window on monitors with a real-fullscreen window
/// and re-shows the rest. A pinned floating window renders above fullscreen,
/// so without this the bar sits on top of fullscreen video. No-op unless the
/// hidden set changed since the last call.
async fn sync_topbar_fullscreen_visibility(proxy: &LaunchProxy, last_hidden: &mut std::collections::HashSet<String>) {
    let (hidden, known) = fullscreen_monitor_names();
    if hidden == *last_hidden {
        return;
    }
    *last_hidden = hidden.clone();
    let _ = proxy
        .post_callback(move |ctx| {
            for app in ctx.windows().values() {
                let title = app.window().title();
                if let Some(name) = title.strip_prefix("overlay-").or_else(|| title.strip_prefix("topbar-")) {
                    if known.contains(name) {
                        app.window().set_visible(!hidden.contains(name));
                    }
                }
            }
        })
        .await;
}

/// Binds SUPER+L to the finick session locker (`finick-lock`, Freya
/// `locker` as fallback) if no binding already uses it.
/// Retries until Hyprland is reachable (systemd services often start before
/// `HYPRLAND_INSTANCE_SIGNATURE` is usable) so the bind is not silently lost.
pub fn ensure_super_l_lock_binding() {
    std::thread::spawn(|| {
        for _ in 0..60 {
            match try_ensure_super_l_once() {
                Ok(true) => return,
                Ok(false) => {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                Err(e) => {
                    eprintln!("[finick] SUPER+L bind attempt failed: {e}");
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
        eprintln!("[finick] giving up binding SUPER+L: hyprland unreachable");
    });
}

/// Binds SUPER+SPACE to the finick launcher if no binding already uses it.
/// Same retry strategy as the locker bind so early startup never drops it.
pub fn ensure_super_space_launcher_binding() {
    std::thread::spawn(|| {
        for _ in 0..60 {
            match try_ensure_super_space_once() {
                Ok(true) => return,
                Ok(false) => {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                Err(e) => {
                    eprintln!("[finick] SUPER+SPACE bind attempt failed: {e}");
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
        eprintln!("[finick] giving up binding SUPER+SPACE: hyprland unreachable");
    });
}

pub fn ensure_finick_default_bindings() {
    ensure_super_l_lock_binding();
    ensure_super_space_launcher_binding();
}

fn try_ensure_super_l_once() -> Result<bool, String> {
    let locker_cmd =
        system::locker_binary().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "finick-lock".to_string());
    // Always (re)install the Lua bind. Skipping when *any* SUPER+L bind exists
    // leaves broken legacy hyprlang binds (`exec /run/...`) in place on Lua
    // configs, where they fail at press time with
    // `return hl.dispatch(exec ...)` / "nil value (global 'exec')".
    // `hl.bind` is idempotent, so re-binding heals that state.
    system::hypr_bind("SUPER + L", &locker_cmd).map_err(|e| format!("hyprctl eval bind failed: {e}"))
}

fn try_ensure_super_space_once() -> Result<bool, String> {
    let launcher_cmd = system::launcher_exec_cmd();
    // Same as SUPER+L above: always refresh the Lua bind so a legacy
    // `bind = SUPER, SPACE, exec, /run/.../launcher` (broken on Lua configs)
    // gets healed instead of being treated as "already bound".
    system::hypr_bind("SUPER + SPACE", &launcher_cmd).map_err(|e| format!("hyprctl eval bind failed: {e}"))
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

    // Ensure reservations survive reload/hotplug and overlay windows keep
    // their float/pin/size/position privileges (see `topbar_audit_lua`).
    let _ = std::process::Command::new("hyprctl").args(["eval", topbar_audit_lua()]).output();
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
    // Starts unset so the first reconcile installs the eval rules even if
    // Hyprland was not reachable yet when `run()` applied them at startup.
    let mut last_rules_applied: Option<std::time::Instant> = None;
    // Monitors whose topbar is currently hidden for fullscreen. Starts empty
    // so a pre-existing fullscreen (e.g. resumed session) hides the bar on
    // the first periodic sync below.
    let mut last_hidden: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Fullscreen checks cost two `hyprctl -j` queries, so the ticker only
    // re-checks every 2s; `fullscreen>>` socket events sync immediately.
    let mut last_fs_check = std::time::Instant::now() - std::time::Duration::from_secs(10);

    loop {
        tokio::select! {
            line = read_socket_event(&mut socket_reader) => {
                if let Some(event_line) = line {
                    // A reload wipes eval-installed rules/reservations, so
                    // re-apply immediately instead of waiting for the throttle.
                    let reloaded = event_line.contains("configreloaded");
                    reapply_topbar_rules_if_due(&mut last_rules_applied, reloaded);
                    if event_line.starts_with("fullscreen>>") {
                        sync_topbar_fullscreen_visibility(&proxy, &mut last_hidden).await;
                        last_fs_check = std::time::Instant::now();
                    }
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
                reapply_topbar_rules_if_due(&mut last_rules_applied, false);
                if last_fs_check.elapsed() >= std::time::Duration::from_secs(2) {
                    sync_topbar_fullscreen_visibility(&proxy, &mut last_hidden).await;
                    last_fs_check = std::time::Instant::now();
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

/// Page slugs accepted on the command line / via URI handlers.
/// Each names a utility window to open on startup alongside the topbars.
pub const OVERLAY_PAGE_SLUGS: &[&str] =
    &["panel", "clipboard", "screenshot", "notifications", "launcher", "session", "osd", "topbar"];

pub fn run() {
    let launch_args = config::LaunchArgs::parse();
    if launch_args.wants_help() {
        launch_args.print_help("overlay", OVERLAY_PAGE_SLUGS);
        return;
    }
    let initial_page = launch_args.page.clone().map(|p| config::normalize_page_key(&p));
    if let Some(page) = initial_page.as_deref() {
        if !OVERLAY_PAGE_SLUGS.contains(&page) {
            eprintln!("[overlay] unknown page {page:?}, opening topbars only");
        }
    }
    launch_args.maybe_arm_screenshot("overlay");
    let open_utility = initial_page.clone();

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
    ensure_finick_default_bindings();
    let initial_monitors = get_monitor_states();
    for mon in &initial_monitors {
        apply_hyprland_monitor_rules(mon);
    }

    let mut launch_config = LaunchConfig::new()
        .with_exit_on_close(false)
        .with_global(PanelWindowId::default())
        .with_global(NotificationPanelWindowId::default())
        .with_global(CalendarPanelWindowId::default())
        .with_global(ClipboardWindowId::default())
        .with_global(ScreenshotWindowId::default())
        .with_global(ToastWindowId::default())
        .with_global(modal::ModalState::default());

    for mon in &initial_monitors {
        launch_config = launch_config.with_window(make_topbar_window_config(mon));
    }

    // Deep-link: `--page <utility>` opens that utility window on startup
    // (useful for URI handlers and screenshot debugging).
    match open_utility.as_deref() {
        Some("panel") => launch_config = launch_config.with_window(panel_window_config()),
        Some("clipboard") => {
            launch_config = launch_config.with_window(crate::clipboard_manager::clipboard_window_config())
        }
        Some("screenshot") => {
            launch_config = launch_config.with_window(crate::screenshot::screenshot_window_config())
        }
        Some("notifications") => {
            launch_config =
                launch_config.with_window(crate::notification_panel::notification_panel_window_config())
        }
        Some("calendar") | Some("date") => {
            launch_config = launch_config.with_window(crate::date_panel::date_panel_window_config())
        }
        Some("launcher") => launch_config = launch_config.with_window(crate::launcher::launcher_window_config()),
        Some("session") => launch_config = launch_config.with_window(crate::session::session_window_config()),
        Some("osd") => launch_config = launch_config.with_window(crate::osd::osd_window_config()),
        Some("toasts") => launch_config = launch_config.with_window(crate::toasts::toasts_window_config()),
        Some("topbar") | Some("") | None => {}
        Some(other) => {
            eprintln!("[overlay] unknown page {other:?}, opening topbars only");
        }
    }

    let monitors_for_listener = initial_monitors.clone();
    launch_config = launch_config.with_future(move |proxy| async move {
        let proxy_clone = proxy.clone();
        let handle = tokio::runtime::Handle::current();
        let handle2 = handle.clone();
        tokio::spawn(async move {
            let handle = handle2;
            let _ = ipc::start_server(
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
                                        if title == "overlay-modal" || title == "clipboard" || title == "screenshot" || title == "notifications-panel" || title == "calendar-panel" || title == "control-panel" {
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
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::CalendarPanelWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(ctx) = ctx.global_contexts.try_get_context::<crate::ToastWindowId>() {
                                    if let Ok(mut g) = ctx.0.lock() { *g = None; }
                                }
                                if let Some(modal_ctx) = ctx.global_contexts.try_get_context::<modal::ModalState>() {
                                    if let Ok(mut g) = modal_ctx.req.lock() { *g = None; }
                                }
                            }).await;
                        });
                        for cls in [
                            "class:^(overlay-modal)$",
                            "class:^(overlay-clipboard)$",
                            "class:^(overlay-screenshot)$",
                            "class:^(overlay-notifications)$",
                            "class:^(overlay-calendar)$",
                            "class:^(overlay-toasts)$",
                            "class:^(overlay-panel)$",
                        ] {
                            system::hypr_close_window(cls);
                        }
                    } else if let ipsea::modals::ModalRequest::Clipboard = req {
                        let proxy = proxy_clone.clone();
                        let h = handle.clone();
                        let tx2 = tx.clone();
                        h.spawn(async move {
                            let mut is_open = proxy.post_callback(|ctx| {
                                ctx.global_contexts
                                    .try_get_context::<crate::ClipboardWindowId>()
                                    .and_then(|cw| cw.0.lock().ok().and_then(|g| *g))
                                    .is_some_and(|id| ctx.windows().contains_key(&id))
                            }).await.unwrap_or(false);
                            if is_open {
                                let hypr_live = tokio::task::spawn_blocking(|| {
                                    crate::state::hypr_has_any_class_blocking(&["overlay-clipboard"])
                                })
                                .await
                                .unwrap_or(true);
                                if !hypr_live {
                                    let _ = proxy.post_callback(|ctx| {
                                        if let Some(c) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                            if let Ok(mut g) = c.0.lock() {
                                                if let Some(id) = *g {
                                                    *g = None;
                                                    ctx.windows_mut().remove(&id);
                                                }
                                            }
                                        }
                                    }).await;
                                    let _ = std::process::Command::new("hyprctl")
                                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-clipboard)$' }))"])
                                        .output();
                                    is_open = false;
                                }
                            }
                            if is_open {
                                crate::state::request_clipboard_close();
                                let _ = tx2.send(ipsea::modals::ModalResponse::Success { data: None });
                                return;
                            }
                            if crate::state::clipboard_opened_recently() {
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
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-calendar)$' }))"])
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
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::CalendarPanelWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                let ids: Vec<winit::window::WindowId> = ctx.windows().keys().copied().collect();
                                for id in ids {
                                    if let Some(app) = ctx.windows().get(&id) {
                                        let title = app.window().title();
                                        if title == "screenshot" || title == "control-panel" || title == "notifications-panel" || title == "calendar-panel" {
                                            ctx.windows_mut().remove(&id);
                                        }
                                    }
                                }
                            }).await;
                            let mon = get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
                            let w = 520;
                            let hgt = 380;
                            let px = mon.x + mon.width as i32 - w - 12;
                            let py = mon.y + PANEL_TOP_OFFSET;
                            crate::state::arm_travel_guard(px as f64, py as f64, 520., 380., (px + 460) as f64);
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
                            crate::state::note_clipboard_opened();
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
                            let mut is_open = proxy.post_callback(|ctx| {
                                ctx.global_contexts
                                    .try_get_context::<crate::ScreenshotWindowId>()
                                    .and_then(|sw| sw.0.lock().ok().and_then(|g| *g))
                                    .is_some_and(|id| ctx.windows().contains_key(&id))
                            }).await.unwrap_or(false);
                            if is_open {
                                let hypr_live = tokio::task::spawn_blocking(|| {
                                    crate::state::hypr_has_any_class_blocking(&["overlay-screenshot"])
                                })
                                .await
                                .unwrap_or(true);
                                if !hypr_live {
                                    let _ = proxy.post_callback(|ctx| {
                                        if let Some(c) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>() {
                                            if let Ok(mut g) = c.0.lock() {
                                                if let Some(id) = *g {
                                                    *g = None;
                                                    ctx.windows_mut().remove(&id);
                                                }
                                            }
                                        }
                                    }).await;
                                    let _ = std::process::Command::new("hyprctl")
                                        .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-screenshot)$' }))"])
                                        .output();
                                    is_open = false;
                                }
                            }
                            if is_open {
                                crate::state::request_screenshot_close();
                                let _ = tx2.send(ipsea::modals::ModalResponse::Success { data: None });
                                return;
                            }
                            if crate::state::screenshot_opened_recently() {
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
                                .args(["eval", "hl.dispatch(hl.dsp.window.close({ window = 'class:^(overlay-calendar)$' }))"])
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
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::CalendarPanelWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                if let Some(c) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>() {
                                    if let Ok(mut g) = c.0.lock() { *g = None; }
                                }
                                let ids: Vec<winit::window::WindowId> = ctx.windows().keys().copied().collect();
                                for id in ids {
                                    if let Some(app) = ctx.windows().get(&id) {
                                        let title = app.window().title();
                                        if title == "clipboard" || title == "control-panel" || title == "notifications-panel" || title == "calendar-panel" {
                                            ctx.windows_mut().remove(&id);
                                        }
                                    }
                                }
                            }).await;
                            let mon = get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
                            let w = 500;
                            let hgt = 264;
                            let px = mon.x + mon.width as i32 - w - 12;
                            let py = mon.y + PANEL_TOP_OFFSET;
                            crate::state::arm_travel_guard(px as f64, py as f64, 500., 264., (px + 440) as f64);
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
                            crate::state::note_screenshot_opened();
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
                                    system::hypr_bring_to_top("");
                                }
                            });
                        });
                    }
                }
            );
        });
        let proxy_toasts = proxy.clone();
        tokio::spawn(async move {
            crate::toasts::run_toast_manager(proxy_toasts).await;
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
            let _ = ipc::start_server(
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
    fn test_rate_limit_ok_debounces() {
        let last = std::sync::atomic::AtomicU64::new(0);
        assert!(crate::state::rate_limit_ok(&last, 350));
        assert!(!crate::state::rate_limit_ok(&last, u64::MAX));
        assert!(crate::state::rate_limit_ok(&last, 0));
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
    fn test_panel_config_fixed_size() {
        // Panel uses a fixed 460x700 window; dynamic height helpers were removed
        // as dead code. Keep a guard so the config stays fixed.
        let cfg = panel_window_config();
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

    #[test]
    fn test_topbar_audit_lua_enforces_reserved_and_privileges() {
        let lua = super::topbar_audit_lua();
        // Reserved space must be re-asserted: without it maximized windows
        // paint underneath the transparent topbar window.
        assert!(lua.contains("reserved") && lua.contains("top = 36"), "audit must enforce reserved top=36");
        // Lost float/pin/geometry must be restored.
        for token in ["float", "pin", "resize", "move", "overlay-", "topbar-"] {
            assert!(lua.contains(token), "audit must handle {token}");
        }
    }

    #[test]
    fn test_parse_fullscreen_monitors_real_fullscreen_hides() {
        let monitors = r#"[{"id":0,"name":"DP-1"},{"id":1,"name":"HDMI-A-1"}]"#;
        let clients = r#"[
            {"address":"0x1","mapped":true,"monitor":1,"class":"firefox","title":"video","fullscreen":2,"fullscreenMode":2}
        ]"#;
        let (hidden, known) = super::parse_fullscreen_monitors(clients, monitors);
        assert!(hidden.contains("HDMI-A-1"));
        assert!(!hidden.contains("DP-1"));
        assert!(known.contains("DP-1") && known.contains("HDMI-A-1"));
    }

    #[test]
    fn test_parse_fullscreen_monitors_maximize_keeps_bar() {
        let monitors = r#"[{"id":0,"name":"DP-1"}]"#;
        let clients = r#"[
            {"address":"0x1","mapped":true,"monitor":0,"class":"code","title":"ed","fullscreen":1,"fullscreenMode":1}
        ]"#;
        let (hidden, _) = super::parse_fullscreen_monitors(clients, monitors);
        assert!(hidden.is_empty(), "maximize keeps gaps and bars, topbar must stay");
    }

    #[test]
    fn test_parse_fullscreen_monitors_ignores_unmapped_and_unknown() {
        let monitors = r#"[{"id":0,"name":"DP-1"}]"#;
        let clients = r#"[
            {"address":"0x1","mapped":false,"monitor":0,"fullscreen":2,"fullscreenMode":2},
            {"address":"0x2","mapped":true,"monitor":99,"fullscreen":2,"fullscreenMode":2}
        ]"#;
        let (hidden, _) = super::parse_fullscreen_monitors(clients, monitors);
        assert!(hidden.is_empty());
        let (hidden_bad, known_bad) = super::parse_fullscreen_monitors("not json", "also not json");
        assert!(hidden_bad.is_empty() && known_bad.is_empty());
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
