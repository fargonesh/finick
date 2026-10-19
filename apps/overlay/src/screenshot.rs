use {
    freya::prelude::*,
    system::{CaptureOptions, CaptureTarget, SystemBackend, capture::take_screenshot},
    ui::*,
};

#[derive(PartialEq)]
pub struct ScreenshotUtility {
    pub dismiss: State<bool>,
}

impl Component for ScreenshotUtility {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let status = use_state(|| Option::<String>::None);
        let monitors = use_hook(|| system::HyprlandBackend.get_displays());
        let mon_names: Vec<String> = monitors.iter().map(|m| m.name.clone()).collect();
        let selected_mon = use_state(|| mon_names.first().cloned().unwrap_or_default());
        let copying = use_state(|| false);
        let mut dismiss = self.dismiss;
        let wid = Platform::window_id();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(6.)
            .background(Color::TRANSPARENT)
            .content(Content::Flex)
            .child(
                PanelShell::new(dismiss.into_readable())
                    .duration(300)
                    .scale_from(1.0)
                    .on_dismiss(EventHandler::new(move |()| Platform::get().close_window(wid)))
                    .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::auto())
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
                                .child(small_close_button(move || dismiss.set(true))),
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
                                    action_tile_flex(FULLSCREEN, "Fullscreen", false, move || {
                                        if *cp.read() {
                                            return;
                                        }
                                        cp.set(true);
                                        st.set(Some("Capturing fullscreen…".to_string()));
                                        let mut st2 = st;
                                        let mut cp2 = cp;
                                        spawn(async move {
                                            let accent = get_theme().accent_name.to_string();
                                            let r = tokio::task::spawn_blocking(move || {
                                                let opts = CaptureOptions { accent_hex: Some(accent), ..Default::default() };
                                                take_screenshot(&CaptureTarget::FullScreen, &opts)
                                            })
                                            .await
                                            .unwrap_or(Err("task failed".to_string()));
                                            let msg = match r {
                                                Ok(v) => {
                                                    if v.success {
                                                        format!(
                                                            "Saved {}",
                                                            v.file_path.map(|p| p.display().to_string()).unwrap_or_default()
                                                        )
                                                    } else {
                                                        v.message
                                                    }
                                                }
                                                Err(e) => e,
                                            };
                                            let msg2 = msg.clone();
                                            st2.set(Some(msg));
                                            cp2.set(false);
                                            system::capture::send_notification("Screenshot", &msg2);
                                        });
                                    })
                                })
                                .child({
                                    let mut st = status;
                                    let mut cp = copying;
                                    action_tile_flex(CROP_ICON, "Region", false, move || {
                                        if *cp.read() {
                                            return;
                                        }
                                        cp.set(true);
                                        st.set(Some("Select region…".to_string()));
                                        let mut st2 = st;
                                        let mut cp2 = cp;
                                        spawn(async move {
                                            let accent = get_theme().accent_name.to_string();
                                            let r = tokio::task::spawn_blocking(move || {
                                                let opts = CaptureOptions { accent_hex: Some(accent), ..Default::default() };
                                                take_screenshot(&CaptureTarget::Region, &opts)
                                            })
                                            .await
                                            .unwrap_or(Err("task failed".to_string()));
                                            let msg = match r {
                                                Ok(v) => {
                                                    if v.success {
                                                        format!(
                                                            "Saved {}",
                                                            v.file_path.map(|p| p.display().to_string()).unwrap_or_default()
                                                        )
                                                    } else {
                                                        v.message
                                                    }
                                                }
                                                Err(e) => e,
                                            };
                                            let msg2 = msg.clone();
                                            st2.set(Some(msg));
                                            cp2.set(false);
                                            system::capture::send_notification("Screenshot", &msg2);
                                        });
                                    })
                                })
                                .child({
                                    let mut st = status;
                                    let mut cp = copying;
                                    let mon = selected_mon.read().clone();
                                    action_tile_flex(MONITOR_ICON, "Monitor", false, move || {
                                        if *cp.read() || mon.is_empty() {
                                            return;
                                        }
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
                                            })
                                            .await
                                            .unwrap_or(Err("task failed".to_string()));
                                            let msg = match r {
                                                Ok(v) => {
                                                    if v.success {
                                                        format!(
                                                            "Saved {}",
                                                            v.file_path.map(|p| p.display().to_string()).unwrap_or_default()
                                                        )
                                                    } else {
                                                        v.message
                                                    }
                                                }
                                                Err(e) => e,
                                            };
                                            let msg2 = msg.clone();
                                            st2.set(Some(msg));
                                            cp2.set(false);
                                            system::capture::send_notification("Screenshot", &msg2);
                                        });
                                    })
                                }),
                        )
                        .maybe(mon_names.len() > 1, |el| {
                            let opts: Vec<DropdownOption> =
                                mon_names.iter().map(|n| DropdownOption { label: n.clone(), value: n.clone() }).collect();
                            let cur = selected_mon.read().clone();
                            el.child(dropdown_select(
                                cur,
                                opts,
                                EventHandler::new({
                                    let mut sm = selected_mon;
                                    move |v: String| sm.set(v)
                                }),
                            ))
                        })
                        .maybe_child(status.read().clone().map(|m| {
                            let short = if m.chars().count() > 60 {
                                format!("{}…", m.chars().take(59).collect::<String>())
                            } else {
                                m
                            };
                            rect()
                                .width(Size::fill())
                                .padding((6., 8.))
                                .corner_radius(8.)
                                .background(t.bg)
                                .border(Border::new().width(1.).fill(t.border))
                                .child(label().font_size(11.).color(t.text_dim).text(short))
                        }))
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .main_align(Alignment::End)
                                .content(Content::Flex)
                                .child(ghost_button("Close", || Platform::get().close_window(Platform::window_id()))),
                        ),
                ),
            )
            .into_element()
    }
}

pub fn screenshot_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let dismiss = use_state(|| false);
    use_hook(move || {
        crate::state::spawn_panel_focus_watcher(&["overlay-screenshot", "overlay", "topbar"], "screenshot", dismiss);
    });
    ScreenshotUtility { dismiss }.into_element()
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
        .with_on_close(|ctx, _wid| {
            if let Some(sw) = ctx.global_contexts.try_get_context::<crate::ScreenshotWindowId>()
                && let Ok(mut g) = sw.0.lock() {
                    *g = None;
                }
            CloseDecision::Close
        })
}


