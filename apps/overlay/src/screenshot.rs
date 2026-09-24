use freya::prelude::*;
use system::{CaptureOptions, CaptureTarget, SystemBackend, capture::take_screenshot};
use ui::*;

#[derive(PartialEq)]
pub struct ScreenshotUtility;

impl Component for ScreenshotUtility {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let status = use_state(|| Option::<String>::None);
        let monitors = use_hook(|| system::HyprlandBackend.get_displays());
        let mon_names: Vec<String> = monitors.iter().map(|m| m.name.clone()).collect();
        let selected_mon = use_state(|| mon_names.first().cloned().unwrap_or_default());
        let copying = use_state(|| false);
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(6.)
            .background(Color::TRANSPARENT)
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .padding(12.)
                    .corner_radius(20.)
                    .background(t.panel)
                    .border(Border::new().width(1.).fill(t.border))
                    .vertical()
                    .spacing(8.)
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .content(Content::Flex)
                            .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text).text("Screenshot"))
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
                                    .child(label().font_size(12.).color(t.text_dim).text("✕")),
                            ),
                    )
                    .child(label().font_size(12.).color(t.text_dim).text("Capture and copy to clipboard"))
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .spacing(8.)
                            .content(Content::Flex)
                            .child({
                                let mut st = status;
                                let mut cp = copying;
                                rect()
                                    .width(Size::flex(1.))
                                    .height(Size::px(56.))
                                    .corner_radius(12.)
                                    .background(t.panel_raised)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .center()
                                    .cursor(CursorIcon::Pointer)
                                    .on_press(move |_| {
                                        if *cp.read() { return; }
                                        cp.set(true);
                                        st.set(Some("Capturing fullscreen…".to_string()));
                                        let mut st2 = st;
                                        let mut cp2 = cp;
                                        spawn(async move {
                                            let accent = get_theme().accent_name.to_string();
                                            let r = tokio::task::spawn_blocking(move || {
                                                let opts = CaptureOptions { accent_hex: Some(accent), ..Default::default() };
                                                take_screenshot(&CaptureTarget::FullScreen, &opts)
                                            }).await.unwrap_or(Err("task failed".to_string()));
                                            let msg = match r {
                                                Ok(v) => if v.success { format!("Saved {}", v.file_path.map(|p| p.display().to_string()).unwrap_or_default()) } else { v.message },
                                                Err(e) => e,
                                            };
                                            let msg2 = msg.clone();
                                            st2.set(Some(msg));
                                            cp2.set(false);
                                            system::capture::send_notification("Screenshot", &msg2);
                                        });
                                    })
                                    .child(rect().vertical().center().spacing(4.).child(icon(CAMERA, 18., t.text)).child(label().font_size(11.).color(t.text).text("Fullscreen")))
                            })
                            .child({
                                let mut st = status;
                                let mut cp = copying;
                                rect()
                                    .width(Size::flex(1.))
                                    .height(Size::px(56.))
                                    .corner_radius(12.)
                                    .background(t.panel_raised)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .center()
                                    .cursor(CursorIcon::Pointer)
                                    .on_press(move |_| {
                                        if *cp.read() { return; }
                                        cp.set(true);
                                        st.set(Some("Select region…".to_string()));
                                        let mut st2 = st;
                                        let mut cp2 = cp;
                                        spawn(async move {
                                            let accent = get_theme().accent_name.to_string();
                                            let r = tokio::task::spawn_blocking(move || {
                                                let opts = CaptureOptions { accent_hex: Some(accent), ..Default::default() };
                                                take_screenshot(&CaptureTarget::Region, &opts)
                                            }).await.unwrap_or(Err("task failed".to_string()));
                                            let msg = match r {
                                                Ok(v) => if v.success { format!("Saved {}", v.file_path.map(|p| p.display().to_string()).unwrap_or_default()) } else { v.message },
                                                Err(e) => e,
                                            };
                                            let msg2 = msg.clone();
                                            st2.set(Some(msg));
                                            cp2.set(false);
                                            system::capture::send_notification("Screenshot", &msg2);
                                        });
                                    })
                                    .child(rect().vertical().center().spacing(4.).child(icon(CROP_ICON, 18., t.text)).child(label().font_size(11.).color(t.text).text("Region")))
                            })
                            .child({
                                let mut st = status;
                                let mut cp = copying;
                                let mon = selected_mon.read().clone();
                                rect()
                                    .width(Size::flex(1.))
                                    .height(Size::px(56.))
                                    .corner_radius(12.)
                                    .background(t.panel_raised)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .center()
                                    .cursor(CursorIcon::Pointer)
                                    .on_press(move |_| {
                                        if *cp.read() || mon.is_empty() { return; }
                                        let m = mon.clone();
                                        cp.set(true);
                                        st.set(Some(format!("Capturing {m}…")));
                                        let mut st2 = st;
                                        let mut cp2 = cp;
                                        spawn(async move {
                                            let accent = get_theme().accent_name.to_string();
                                            let r = tokio::task::spawn_blocking(move || {
                                                let opts = CaptureOptions { accent_hex: Some(accent), ..Default::default() };
                                                take_screenshot(&CaptureTarget::Monitor(m), &opts)
                                            }).await.unwrap_or(Err("task failed".to_string()));
                                            let msg = match r {
                                                Ok(v) => if v.success { format!("Saved {}", v.file_path.map(|p| p.display().to_string()).unwrap_or_default()) } else { v.message },
                                                Err(e) => e,
                                            };
                                            let msg2 = msg.clone();
                                            st2.set(Some(msg));
                                            cp2.set(false);
                                            system::capture::send_notification("Screenshot", &msg2);
                                        });
                                    })
                                    .child(rect().vertical().center().spacing(4.).child(icon(MONITOR_ICON, 18., t.text)).child(label().font_size(11.).color(t.text).text("Monitor")))
                            }),
                    )
                    .maybe(mon_names.len() > 1, |el| {
                        let opts: Vec<DropdownOption> = mon_names.iter().map(|n| DropdownOption { label: n.clone(), value: n.clone() }).collect();
                        let cur = selected_mon.read().clone();
                        el.child(dropdown_select(cur, opts, EventHandler::new({
                            let mut sm = selected_mon;
                            move |v: String| sm.set(v)
                        })))
                    })
                    .maybe_child(status.read().clone().map(|m| {
                        let short = if m.chars().count() > 60 { format!("{}…", m.chars().take(59).collect::<String>()) } else { m };
                        rect().width(Size::fill()).padding((6.,8.)).corner_radius(8.).background(t.bg).border(Border::new().width(1.).fill(t.border)).child(label().font_size(11.).color(t.text_dim).text(short))
                    }))
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(ghost_button("Close", || Platform::get().close_window(Platform::window_id()))),
                    ),
            )
            .into_element()
    }
}

pub fn screenshot_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    use_hook(|| {
        let wid = Platform::window_id();
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
                        if s.contains("\"class\": \"overlay-screenshot\"") {
                            return false;
                        }
                        return !s.contains("\"title\": \"screenshot\"");
                    }
                    false
                })
                .await
                .unwrap_or(false);
                if should_close {
                    Platform::get().close_window(wid);
                    break;
                }
            }
        });
    });
    ScreenshotUtility.into_element()
}

pub fn screenshot_window_config() -> WindowConfig {
    WindowConfig::new(screenshot_app)
        .with_title("screenshot")
        .with_app_id("overlay-screenshot")
        .with_size(500., 264.)
        .with_min_size(500., 240.)
        .with_max_size(500., 400.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|_ctx, _wid| {
            if let Some(sw) = GlobalContexts::get().try_get_context::<crate::ScreenshotWindowId>() {
                if let Ok(mut g) = sw.0.lock() {
                    *g = None;
                }
            }
            CloseDecision::Close
        })
}

pub fn open_screenshot_utility() {
    let mon = crate::get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(crate::MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
    let w = 500;
    let h = 264;
    let px = mon.x + mon.width as i32 - w - 12;
    let py = mon.y + 44;
    let lua_rule = format!(
        r#"hl.window_rule({{ name = "finick-screenshot-position", match = {{ class = "^(overlay-screenshot)$" }}, monitor = "{mon}", float = true, pin = true, move = {{ {px}, {py} }}, size = {{ {w}, {h} }}, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 }})"#,
        mon = mon.name, px = px, py = py, w = w, h = h
    );
    let _ = std::process::Command::new("hyprctl").args(["eval", &lua_rule]).output();
    let cfg = screenshot_window_config();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(35)).await;
        let wid = Platform::get().launch_window(cfg).await;
        let _ = wid;
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
                            hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {w}, y = {h}, relative = false }}))
                            break
                        end
                    end
                    "#,
                    px = px, py = py, w = w, h = h
                );
                let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
            }
        });
    });
}
