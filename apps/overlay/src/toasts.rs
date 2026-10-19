use {
    crate::{
        ToastWindowId, get_monitor_states, notification_popup::NotificationPopup, state::cursor_pos_blocking,
    },
    freya::prelude::*,
    ipsea::notifications::{
        NOTIFICATIONS_SOCKET_NAME, Notification, NotificationEvent, get_active_notifications, subscribe_channel,
    },
    ui::*,
};

const TOAST_WIDTH: i32 = 400;
const TOAST_HEIGHT: i32 = 400;
const TOAST_MAX: usize = 4;
const TOAST_DEFAULT_TIMEOUT_MS: u64 = 5000;

#[derive(PartialEq, Clone)]
pub struct ToastItem {
    notif: Notification,
    on_remove: EventHandler<u32>,
}

impl Component for ToastItem {
    fn render(&self) -> impl IntoElement {
        let dismissed = use_state(|| false);
        let timeout = self.notif.timeout;
        let id = self.notif.id;
        let notif = self.notif.clone();
        let on_remove = self.on_remove.clone();
        use_hook(move || {
            let mut d = dismissed;
            spawn(async move {
                let ms = if timeout > 0 { timeout as u64 } else { TOAST_DEFAULT_TIMEOUT_MS };
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                d.set(true);
            });
        });
        let mut dx = dismissed;
        let popup = NotificationPopup::from_notification(&notif)
            .with_auto_dismiss(false)
            .with_width(Size::fill())
            .on_close(move |_: u32| dx.set(true));
        let app_name = notif.app_name.clone();
        let summary = notif.summary.clone();
        rect()
            .width(Size::fill())
            .cursor(CursorIcon::Pointer)
            .content(Content::Flex)
            .on_press(move |_| {
                let (a, s) = (app_name.clone(), summary.clone());
                std::thread::spawn(move || focus_notification_source(&a, &s));
            })
            .child(
                PanelShell::new(dismissed.into_readable())
                    .duration(240)
                    .scale_from(1.0)
                    .on_dismiss(EventHandler::new(move |()| on_remove.call(id)))
                    .child(popup),
            )
    }
}

fn norm_text(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect()
}

fn focus_notification_source(app_name: &str, summary: &str) {
    let app = norm_text(app_name);
    let sum = norm_text(summary);
    if app.len() < 2 && sum.len() < 2 {
        return;
    }
    let out = std::process::Command::new("hyprctl").args(["clients", "-j"]).output();
    let Ok(o) = out else { return };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) else { return };
    let Some(clients) = v.as_array() else { return };
    let mut best: Option<(i32, String)> = None;
    for c in clients {
        let class = c.get("class").and_then(|v| v.as_str()).unwrap_or("");
        if class.starts_with("overlay") || class.starts_with("topbar") {
            continue;
        }
        let addr = c.get("address").and_then(|v| v.as_str()).unwrap_or("");
        if addr.is_empty() {
            continue;
        }
        let lc = norm_text(class);
        let lt = norm_text(c.get("title").and_then(|v| v.as_str()).unwrap_or(""));
        let mut score = 0;
        if app.len() > 2 && !lc.is_empty() && (lc.contains(&app) || app.contains(&lc)) {
            score += 2;
        }
        if sum.len() > 2 && !lt.is_empty() && (lt.contains(&sum) || sum.contains(&lt)) {
            score += 1;
        }
        if score > 0 && best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, addr.to_string()));
        }
    }
    if let Some((_, addr)) = best {
        let _ = std::process::Command::new("hyprctl")
            .args(["dispatch", "focuswindow", &format!("address:{addr}")])
            .output();
    }
}

fn focused_monitor_name() -> Option<String> {
    let o = std::process::Command::new("hyprctl").args(["monitors", "-j"]).output().ok()?;
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).ok()?;
    v.as_array()?
        .iter()
        .find(|m| m.get("focused").and_then(|f| f.as_bool()).unwrap_or(false))
        .and_then(|m| m.get("name").and_then(|n| n.as_str()))
        .map(|s| s.to_string())
}

pub fn toasts_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let toasts = use_state(Vec::<Notification>::new);
    let wid = Platform::window_id();
    use_hook(move || {
        let mut ts = toasts;
        let w = wid;
        spawn(async move {
            let initial = tokio::task::spawn_blocking(|| {
                get_active_notifications(NOTIFICATIONS_SOCKET_NAME).unwrap_or_default()
            })
            .await
            .unwrap_or_default();
            ts.set(initial);
            loop {
                if let Ok(mut rx) = subscribe_channel(NOTIFICATIONS_SOCKET_NAME) {
                    let fresh = tokio::task::spawn_blocking(|| {
                        get_active_notifications(NOTIFICATIONS_SOCKET_NAME).unwrap_or_default()
                    })
                    .await
                    .unwrap_or_default();
                    ts.set(fresh);
                    while let Some(evt) = rx.recv().await {
                        match evt {
                            NotificationEvent::Show(notif) => {
                                let mut current = ts.read().clone();
                                if let Some(idx) = current.iter().position(|n| n.id == notif.id) {
                                    current[idx] = notif;
                                } else {
                                    current.push(notif);
                                }
                                ts.set(current);
                            }
                            NotificationEvent::Close(id) => {
                                let mut current = ts.read().clone();
                                if let Some(pos) = current.iter().position(|n| n.id == id) {
                                    current.remove(pos);
                                }
                                let empty = current.is_empty();
                                ts.set(current);
                                if empty {
                                    Platform::get().close_window(w);
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        });
    });
    let list = toasts.read().clone();
    let mut remover = toasts;
    let start = list.len().saturating_sub(TOAST_MAX);
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(12.)
        .background(Color::TRANSPARENT)
        .vertical()
        .content(Content::Flex)
        .main_align(Alignment::Start)
        .child(
            rect()
                .width(Size::fill())
                .height(Size::auto())
                .vertical()
                .spacing(10.)
                .content(Content::Flex)
                .children(list[start..].iter().cloned().map(|n| ToastItem {
                    notif: n,
                    on_remove: EventHandler::new(move |rid: u32| {
                        let mut current = remover.read().clone();
                        if let Some(pos) = current.iter().position(|x| x.id == rid) {
                            current.remove(pos);
                        }
                        let empty = current.is_empty();
                        remover.set(current);
                        if empty {
                            Platform::get().close_window(wid);
                        }
                    }),
                })),
        )
        .into_element()
}

pub fn toasts_window_config() -> WindowConfig {
    WindowConfig::new(toasts_app)
        .with_title("toasts")
        .with_app_id("overlay-toasts")
        .with_size(TOAST_WIDTH as f64, TOAST_HEIGHT as f64)
        .with_min_size(380., 120.)
        .with_max_size(400., 600.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|ctx, _wid| {
            if let Some(tw) = ctx.global_contexts.try_get_context::<ToastWindowId>()
                && let Ok(mut g) = tw.0.lock() {
                    *g = None;
                }
            CloseDecision::Close
        })
}

pub async fn run_toast_manager(proxy: LaunchProxy) {
    eprintln!("[toasts] manager started");
    loop {
        if let Ok(mut rx) = subscribe_channel(NOTIFICATIONS_SOCKET_NAME) {
            eprintln!("[toasts] subscribed");
            while let Some(evt) = rx.recv().await {
                if matches!(evt, NotificationEvent::Show(_)) {
                    eprintln!("[toasts] show event, ensuring window");
                    ensure_toasts_window(&proxy).await;
                }
            }
            eprintln!("[toasts] subscription ended, retrying");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub async fn ensure_toasts_window(proxy: &LaunchProxy) {
    let exists = proxy
        .post_callback(|ctx| {
            ctx.global_contexts
                .try_get_context::<ToastWindowId>()
                .and_then(|c| c.0.lock().ok().and_then(|g| *g))
                .is_some()
        })
        .await
        .unwrap_or(true);
    if exists {
        return;
    }
    let mons = get_monitor_states();
    let focused = focused_monitor_name().and_then(|n| mons.iter().find(|m| m.name == n).cloned());
    let cursor = cursor_pos_blocking();
    let mon = focused
        .or_else(|| {
            cursor.and_then(|(cx, cy)| {
                mons.iter()
                    .find(|m| {
                        cx >= m.x as f64
                            && cx < (m.x as f64 + m.width)
                            && cy >= m.y as f64
                            && cy < (m.y as f64 + m.height)
                    })
                    .cloned()
            })
        })
        .or_else(|| mons.iter().find(|m| m.x == 0).cloned())
        .unwrap_or(crate::MonitorState {
            name: "default".to_string(),
            x: 0,
            y: 0,
            width: 1920.0,
            height: 1080.0,
        });
    let w = TOAST_WIDTH;
    let h = TOAST_HEIGHT;
    let px = mon.x + mon.width as i32 - w - 12;
    let py = mon.y + 44;
    let mon_name = mon.name.clone();
    let _ = std::process::Command::new("hyprctl").args(["eval", &format!(r#"hl.window_rule({{ name = "finick-toasts-live-{mon_name}", match = {{ class = "^(overlay-toasts)$" }}, monitor = "{mon_name}", float = true, pin = true, move = {{ {px}, {py} }}, size = {{ {w}, {h} }}, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, no_initial_focus = true, rounding = 20 }})"#, mon_name=mon_name, px=px, py=py, w=w, h=h)]).output();
    tokio::time::sleep(std::time::Duration::from_millis(35)).await;
    let _ = proxy
        .post_callback(move |ctx| {
            let wid = ctx.launch_window(crate::toasts::toasts_window_config());
            if let Some(tw) = ctx.global_contexts.try_get_context::<ToastWindowId>()
                && let Ok(mut g) = tw.0.lock() {
                    *g = Some(wid);
                }
        })
        .await;
    tokio::spawn(async move {
        for delay in [30, 80] {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            let lua = format!(
                r#"
                for _, w in ipairs(hl.get_windows()) do
                    if w.class == 'overlay-toasts' or w.title == 'toasts' then
                        local addr = 'address:' .. tostring(w.address)
                        hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                        hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                        hl.dispatch(hl.dsp.window.move({{ window = addr, x = {px}, y = {py} }}))
                        hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {w}, y = {h}, relative = false }}))
                        break
                    end
                end
                "#,
                px = px,
                py = py,
                w = w,
                h = h,
            );
            let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
        }
    });
}
