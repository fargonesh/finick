use {
    freya::prelude::*,
    std::path::PathBuf,
    system::{CaptureOptions, CaptureTarget, CaptureToolsAvailability, SystemBackend, capture::take_screenshot},
    ui::*,
};

fn run_capture(
    target: CaptureTarget,
    opts: CaptureOptions,
    mut status: State<Option<String>>,
    mut busy: State<bool>,
    mut last_file: State<Option<PathBuf>>,
) {
    if *busy.read() {
        return;
    }
    busy.set(true);
    let label = match &target {
        CaptureTarget::FullScreen => "Capturing fullscreen…".to_string(),
        CaptureTarget::Region => "Select region…".to_string(),
        CaptureTarget::Window => "Capturing window…".to_string(),
        CaptureTarget::Monitor(m) => format!("Capturing {m}…"),
    };
    status.set(Some(label));
    spawn(async move {
        let accent = get_theme().accent_name.to_string();
        let mut opts = opts;
        if opts.accent_hex.is_none() {
            opts.accent_hex = Some(accent);
        }
        let r = tokio::task::spawn_blocking(move || take_screenshot(&target, &opts))
            .await
            .unwrap_or(Err("task failed".to_string()));
        let msg = match r {
            Ok(v) => {
                if v.success {
                    if let Some(p) = v.file_path.clone() {
                        last_file.set(Some(p.clone()));
                    }
                    format!("Saved {}", v.file_path.map(|p| p.display().to_string()).unwrap_or_default())
                } else {
                    v.message
                }
            }
            Err(e) => e,
        };
        let msg2 = msg.clone();
        status.set(Some(msg));
        busy.set(false);
        system::capture::send_notification("Screenshot", &msg2);
    });
}

fn run_recording_toggle(
    target: CaptureTarget,
    opts: CaptureOptions,
    mut status: State<Option<String>>,
    mut busy: State<bool>,
    mut rec: State<Option<system::capture::RecordingStatus>>,
) {
    if *busy.read() {
        return;
    }
    if system::capture::get_active_screencast().is_some() {
        busy.set(true);
        status.set(Some("Stopping recording…".to_string()));
        spawn(async move {
            let r = tokio::task::spawn_blocking(system::capture::stop_screencast).await.unwrap_or(Err("task failed".to_string()));
            let msg = match r {
                Ok(v) => format!("Saved {}", v.file_path.map(|p| p.display().to_string()).unwrap_or_default()),
                Err(e) => e,
            };
            status.set(Some(msg.clone()));
            busy.set(false);
            rec.set(system::capture::get_active_screencast());
            system::capture::send_notification("Recording", &msg);
        });
        return;
    }
    busy.set(true);
    status.set(Some("Starting recording…".to_string()));
    spawn(async move {
        let accent = get_theme().accent_name.to_string();
        let mut opts = opts;
        if opts.accent_hex.is_none() {
            opts.accent_hex = Some(accent);
        }
        let r = tokio::task::spawn_blocking(move || system::capture::start_screencast(&target, &opts))
            .await
            .unwrap_or(Err("task failed".to_string()));
        match r {
            Ok(st) => {
                status.set(Some(format!("Recording {} — {}", st.tool, st.file_path.display())));
                rec.set(Some(st));
            }
            Err(e) => status.set(Some(e)),
        }
        busy.set(false);
    });
}

fn open_path(path: &std::path::Path) -> bool {
    std::process::Command::new("xdg-open").arg(path).spawn().is_ok()
}

#[derive(PartialEq)]
pub struct ScreenshotUtility {
    pub dismiss: State<bool>,
}

impl Component for ScreenshotUtility {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let status = use_state(|| Option::<String>::None);
        let busy = use_state(|| false);
        let last_file = use_state(|| Option::<PathBuf>::None);
        let rec = use_state(|| Option::<system::capture::RecordingStatus>::None);
        let tools = use_state(|| CaptureToolsAvailability::default());
        let delay_idx = use_state(|| 0usize);
        let include_cursor = use_state(|| false);
        let record_audio = use_state(|| false);
        let monitors = use_hook(|| system::HyprlandBackend.get_displays());
        let mon_names: Vec<String> = monitors.iter().map(|m| m.name.clone()).collect();
        let selected_mon = use_state(|| mon_names.first().cloned().unwrap_or_default());
        let mut dismiss = self.dismiss;
        let wid = Platform::window_id();

        {
            let mut tools = tools;
            let mut rec = rec;
            use_hook(move || {
                spawn(async move {
                    let found = tokio::task::spawn_blocking(system::capture::check_tools).await.unwrap_or_default();
                    tools.set(found);
                    rec.set(system::capture::get_active_screencast());
                });
                // Poll the active recording so the pill + OSD timer stay honest.
                let mut rec = rec;
                spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        let cur = tokio::task::spawn_blocking(system::capture::get_active_screencast)
                            .await
                            .unwrap_or(None);
                        rec.set_if_modified(cur);
                    }
                });
            });
        }

        let toolset = tools.read().clone();
        let is_busy = *busy.read();
        let delay_secs = [0u32, 3, 5][(*delay_idx.read()).min(2)];
        let cursor = *include_cursor.read();
        let audio = *record_audio.read();
        let mon = selected_mon.read().clone();
        let active_rec = rec.read().clone();

        let opts_for = move || CaptureOptions {
            delay_seconds: delay_secs,
            include_cursor: cursor,
            record_audio: audio,
            copy_to_clipboard: true,
            custom_output_dir: None,
            custom_filename: None,
            accent_hex: None,
        };
        let rec_opts_for = move || CaptureOptions {
            delay_seconds: 0,
            include_cursor: cursor,
            record_audio: audio,
            copy_to_clipboard: false,
            custom_output_dir: None,
            custom_filename: None,
            accent_hex: None,
        };

        let mut body = rect()
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
            .child(label().font_size(12.).color(t.text_dim).text("Capture, record and save"));

        if !toolset.has_grim {
            body = body.child(notice_pill("Unavailable: grim not installed, screenshots disabled", NoticeKind::Error));
        }
        if !toolset.has_slurp {
            body = body.child(notice_pill("Unavailable: slurp not installed, region select disabled", NoticeKind::Error));
        }

        body = body.child(
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(8.)
                .content(Content::Flex)
                .child({
                    let (st, cp, lf) = (status, busy, last_file);
                    let opts = opts_for();
                    let enabled = toolset.has_grim && !is_busy;
                    action_tile_flex(FULLSCREEN, "Fullscreen", false, move || {
                        if enabled {
                            run_capture(CaptureTarget::FullScreen, opts.clone(), st, cp, lf);
                        }
                    })
                })
                .child({
                    let (st, cp, lf) = (status, busy, last_file);
                    let opts = opts_for();
                    let enabled = toolset.has_grim && toolset.has_slurp && !is_busy;
                    action_tile_flex(CROP_ICON, "Region", false, move || {
                        if enabled {
                            run_capture(CaptureTarget::Region, opts.clone(), st, cp, lf);
                        }
                    })
                })
                .child({
                    let (st, cp, lf) = (status, busy, last_file);
                    let opts = opts_for();
                    let enabled = toolset.has_grim && !is_busy;
                    action_tile_flex(MONITOR_ICON, "Monitor", false, move || {
                        if enabled && !mon.is_empty() {
                            run_capture(CaptureTarget::Monitor(mon.clone()), opts.clone(), st, cp, lf);
                        }
                    })
                })
                .child({
                    let (st, cp, lf) = (status, busy, last_file);
                    let opts = opts_for();
                    let enabled = toolset.has_grim && !is_busy;
                    action_tile_flex(FULLSCREEN, "Window", false, move || {
                        if enabled {
                            run_capture(CaptureTarget::Window, opts.clone(), st, cp, lf);
                        }
                    })
                }),
        );

        body = body.maybe(mon_names.len() > 1, |el| {
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
        });

        // Recording row: start/stop wl-screenrec with status pill.
        if toolset.has_wl_screenrec || toolset.has_wf_recorder {
            let rec_label = if active_rec.is_some() { "Stop recording" } else { "Record region" };
            body = body.child(
                rect().width(Size::fill()).horizontal().spacing(8.).content(Content::Flex).child({
                    let (st, cp, rc) = (status, busy, rec);
                    let opts = rec_opts_for();
                    action_tile_flex(RECORD_ICON, rec_label, active_rec.is_some(), move || {
                        run_recording_toggle(CaptureTarget::Region, opts.clone(), st, cp, rc);
                    })
                }),
            );
            if let Some(r) = active_rec.clone() {
                let elapsed = r.duration_secs;
                body = body.child(setting_row(
                    format!("Recording {} {:02}:{:02}", r.tool, elapsed / 60, elapsed % 60),
                    Some(r.file_path.display().to_string()),
                    false,
                    status_chip("REC", true, None),
                ));
            }
        } else {
            body = body.child(notice_pill(
                "Unavailable: wl-screenrec and wf-recorder not installed, recording disabled",
                NoticeKind::Error,
            ));
        }

        // Delay / cursor / audio toggles + save-to location.
        {
            let mut di = delay_idx;
            body = body.child(tile().child(tile_head(None, "Capture options", None::<String>)).child(setting_row(
                "Delay".to_string(),
                Some("Wait before capturing".to_string()),
                false,
                segmented_control(vec![("None", 0), ("3s", 1), ("5s", 2)], *delay_idx.read(), move |i| di.set(i)),
            )).child(setting_row(
                "Include cursor".to_string(),
                Some("grim -c, wl-screenrec cursor".to_string()),
                false,
                {
                    let mut c = include_cursor;
                    pill_switch(cursor, move |v| c.set(v))
                },
            )).child(setting_row(
                "Record audio".to_string(),
                Some("wl-screenrec --audio".to_string()),
                true,
                {
                    let mut a = record_audio;
                    pill_switch(audio, move |v| a.set(v))
                },
            )).child(setting_row(
                "Save to".to_string(),
                Some(system::capture::default_screenshot_dir().display().to_string()),
                true,
                label().font_size(11.).color(t.text_dim).text("Pictures/Screenshots"),
            )));
        }

        // Annotation is honestly unavailable: no canvas, deep link to the file.
        {
            let last = last_file.read().clone();
            body = body.child(tile().child(tile_head(None, "Annotate", None::<String>)).child(tile_sub(
                "Annotation with arrows, text and blur is not available in this build",
            )).child(setting_row(
                "Open last capture".to_string(),
                Some(match &last {
                    Some(p) => p.display().to_string(),
                    None => "Capture something first".to_string(),
                }),
                false,
                {
                    let mut st = status;
                    ghost_button("Open", move || {
                        if let Some(p) = last.clone() {
                            if !open_path(&p) {
                                st.set(Some("Could not open the file".to_string()));
                            }
                        } else {
                            st.set(Some("Nothing captured yet".to_string()));
                        }
                    })
                },
            )));
        }

        body = body.maybe_child(status.read().clone().map(|m| {
            let short = if m.chars().count() > 90 { format!("{}…", m.chars().take(89).collect::<String>()) } else { m };
            rect()
                .width(Size::fill())
                .padding((6., 8.))
                .corner_radius(8.)
                .background(t.bg)
                .border(Border::new().width(1.).fill(t.border))
                .child(label().font_size(11.).color(t.text_dim).text(short))
        }));
        body = body.child(
            rect()
                .width(Size::fill())
                .horizontal()
                .main_align(Alignment::End)
                .content(Content::Flex)
                .child(ghost_button("Close", {
                    let mut d = dismiss;
                    move || d.set(true)
                })),
        );

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
                    .child(body),
            )
            .into_element()
    }
}

pub fn screenshot_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let dismiss = use_state(|| false);
    let wid = Platform::window_id();
    use_hook(move || {
        let my_wid = wid;
        crate::state::spawn_panel_focus_watcher_guarded(
            &["overlay-screenshot", "overlay", "topbar"],
            "screenshot",
            dismiss,
            move || crate::state::is_screenshot_window_current(my_wid),
        );
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::clear_close_flag(&crate::state::CLOSE_SCREENSHOT);
        crate::state::spawn_close_flag_watcher_guarded(
            &crate::state::CLOSE_SCREENSHOT,
            dismiss,
            move || crate::state::is_screenshot_window_current(my_wid),
        );
    });
    ScreenshotUtility { dismiss }.into_element()
}

pub fn screenshot_window_config() -> WindowConfig {
    WindowConfig::new(screenshot_app)
        .with_title("screenshot")
        .with_app_id("overlay-screenshot")
        .with_size(500., 640.)
        .with_min_size(500., 400.)
        .with_max_size(500., 760.)
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
