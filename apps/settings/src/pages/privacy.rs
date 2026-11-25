use {
    crate::state::*,
    freya::prelude::*,
    ipsea::settings::SettingKey,
    system::SystemBackend,
    ui::*,
};

fn lock_delay_key() -> SettingKey {
    SettingKey::Custom("privacy.lock_delay".to_string())
}

fn usb_key() -> SettingKey {
    SettingKey::Custom("privacy.usb_protection".to_string())
}

fn diagnostics_key() -> SettingKey {
    SettingKey::Custom("privacy.diagnostics".to_string())
}

fn lock_delay_label(secs: u32) -> String {
    match secs {
        0 => "Never".to_string(),
        60 => "1 minute".to_string(),
        s if s < 60 => format!("{s} seconds"),
        s if s % 60 == 0 => format!("{} minutes", s / 60),
        s => format!("{s} seconds"),
    }
}

#[derive(PartialEq)]
pub struct Privacy;

impl Component for Privacy {
    fn render(&self) -> impl IntoElement {
        let store = use_settings();
        let t = use_app_theme();

        // Live hardware probes (system truth). Daemon truth lives in the store.
        let fw: State<Option<system::FirewallStatus>> = use_state(|| None);
        let loc: State<Option<system::LocationStatus>> = use_state(|| None);
        let cam: State<Option<system::CameraStatus>> = use_state(|| None);
        let mic: State<Option<system::MicStatus>> = use_state(|| None);
        let usb: State<Option<system::UsbStatus>> = use_state(|| None);
        let loading: State<bool> = use_state(|| true);
        let hist_armed: State<bool> = use_state(|| false);
        let hist_busy: State<bool> = use_state(|| false);
        let hist_msg: State<Option<(String, bool)>> = use_state(|| None);

        use_hook(move || {
            let mut fw = fw;
            let mut loc = loc;
            let mut cam = cam;
            let mut mic = mic;
            let mut usb = usb;
            let mut loading = loading;
            spawn(async move {
                let probed = tokio::task::spawn_blocking(|| {
                    (
                        system::query_firewall(),
                        system::query_location(),
                        system::query_camera(),
                        system::query_mic(),
                        system::query_usb_protection(),
                    )
                })
                .await
                .ok();
                if let Some((f, l, c, m, u)) = probed {
                    fw.set(Some(f));
                    loc.set(Some(l));
                    cam.set(Some(c));
                    mic.set(Some(m));
                    usb.set(Some(u));
                }
                loading.set(false);
            });
        });

        let is_loading = *loading.read();
        let fw_val = fw.read().clone();
        let loc_val = loc.read().clone();
        let cam_val = cam.read().clone();
        let mic_val = mic.read().clone();
        let usb_val = usb.read().clone();
        let err_opt = store.last_error.read().clone();
        let hist_state = hist_msg.read().clone();
        let armed = *hist_armed.read();
        let busy = *hist_busy.read();

        // Managed (policy) labels per key.
        let fw_lock = store.lock_label(&SettingKey::FirewallEnabled);
        let cam_lock = store.lock_label(&SettingKey::CameraAccess);
        let mic_lock = store.lock_label(&SettingKey::MicAccess);
        let loc_lock = store.lock_label(&SettingKey::LocationAccess);
        let delay_lock = store.lock_label(&lock_delay_key());
        let usb_lock = store.lock_label(&usb_key());
        let diag_lock = store.lock_label(&diagnostics_key());

        let fw_on = *store.firewall_enabled.read();
        let cam_on = *store.camera_access.read();
        let mic_on = *store.mic_access.read();
        let loc_on = *store.location_access.read();
        let delay_secs = *store.lock_delay_secs.read();
        let usb_on = *store.usb_protection.read();
        let diag_on = *store.diagnostics_enabled.read();

        // Row states: Managed wins, then Unavailable, else Live.
        let fw_chip = fw_lock.clone().map(|_| ("Managed", false)).unwrap_or_else(|| {
            if is_loading || fw_val.is_none() {
                ("Checking", false)
            } else if !fw_val.as_ref().map(|f| f.manageable).unwrap_or(false) {
                ("Unavailable", false)
            } else {
                ("Live", fw_on)
            }
        });
        let loc_chip = loc_lock.clone().map(|_| ("Managed", false)).unwrap_or_else(|| {
            if is_loading || loc_val.is_none() {
                ("Checking", false)
            } else if !loc_val.as_ref().map(|l| l.available).unwrap_or(false) {
                ("Unavailable", false)
            } else {
                ("Live", loc_on)
            }
        });
        let mic_chip = mic_lock.clone().map(|_| ("Managed", false)).unwrap_or_else(|| {
            if is_loading || mic_val.is_none() {
                ("Checking", false)
            } else if !mic_val.as_ref().map(|m| m.available).unwrap_or(false) {
                ("Unavailable", false)
            } else {
                ("Live", mic_on)
            }
        });
        let usb_chip = usb_lock.clone().map(|_| ("Managed", false)).unwrap_or_else(|| {
            if is_loading || usb_val.is_none() {
                ("Checking", false)
            } else if !usb_val.as_ref().map(|u| u.available).unwrap_or(false) {
                ("Unavailable", false)
            } else {
                ("Live", usb_on)
            }
        });
        let delay_chip: (&str, bool) = delay_lock.clone().map(|_| ("Managed", false)).unwrap_or(("Live", true));
        let diag_chip: (&str, bool) = diag_lock.clone().map(|_| ("Managed", false)).unwrap_or(("Live", diag_on));

        let fw_detail = fw_val
            .map(|f| format!("{} · {}", f.backend, f.detail))
            .unwrap_or_else(|| "Probing firewall…".to_string());
        let loc_detail = loc_val
            .map(|l| {
                if l.available {
                    format!("{} · {}", l.reason, if l.enabled { "portal on" } else { "portal off" })
                } else {
                    l.reason
                }
            })
            .unwrap_or_else(|| "Probing location portal…".to_string());
        let cam_sub = cam_val
            .map(|c| {
                if c.count == 0 {
                    "No camera device found.".to_string()
                } else {
                    format!(
                        "{} camera device{} found. {}",
                        c.count,
                        if c.count == 1 { "" } else { "s" },
                        c.reason
                    )
                }
            })
            .unwrap_or_else(|| "Probing camera devices…".to_string());
        let mic_detail = mic_val
            .map(|m| {
                if m.available {
                    format!("{} · {}", m.reason, if m.muted { "muted" } else { "open" })
                } else {
                    m.reason
                }
            })
            .unwrap_or_else(|| "Probing microphone…".to_string());
        let usb_detail = usb_val
            .map(|u| {
                if u.available {
                    format!("{} · {}", u.reason, if u.enforced { "enforced" } else { "not enforced" })
                } else {
                    u.reason
                }
            })
            .unwrap_or_else(|| "Probing USBGuard…".to_string());

        let fw_managed = fw_lock.is_some();
        let fw_live = !fw_managed && fw_chip.0 == "Live";
        let loc_managed = loc_lock.is_some();
        let loc_live = !loc_managed && loc_chip.0 == "Live";
        let mic_managed = mic_lock.is_some();
        let mic_live = !mic_managed && mic_chip.0 == "Live";
        let usb_managed = usb_lock.is_some();
        let usb_live = !usb_managed && usb_chip.0 == "Live";
        let delay_managed = delay_lock.is_some();
        let diag_managed = diag_lock.is_some();

        FadeSlideIn::new().child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(page_head(
                    LOCK,
                    "Privacy and security",
                    "Each row shows Live, Managed or Unavailable. No toggle claims enforcement it lacks.",
                ))
                .maybe_child(err_opt.map(|e| {
                    rect()
                        .width(Size::fill())
                        .child(notice_pill(format!("Change failed: {e}"), NoticeKind::Error))
                        .child(rect().margin((6., 0., 0., 0.)).child(ghost_button("Dismiss", {
                            let s = store;
                            move || s.clear_error()
                        })))
                }))
                // Screen lock delay: real enforcement via gsettings + finick fallback.
                .child(
                    tile()
                        .child(tile_head(
                            Some(LOCK),
                            "Screen lock",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(delay_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(delay_chip.0, delay_chip.1, None)),
                            ),
                        ))
                        .child(tile_sub(
                            "Delay before the screen locks. Stored via GNOME screensaver and Finick config.",
                        ))
                        .child(field_label(format!("Lock after · {}", lock_delay_label(delay_secs))))
                        .child(
                            rect().opacity(if delay_managed { 0.45 } else { 1.0 }).child({
                                let s = store;
                                segmented_control(
                                    vec![
                                        ("Never", 0u32),
                                        ("1 min", 60u32),
                                        ("5 min", 300u32),
                                        ("15 min", 900u32),
                                    ],
                                    delay_secs,
                                    move |v: u32| {
                                        if !delay_managed {
                                            s.set(lock_delay_key(), v as i64);
                                        }
                                    },
                                )
                            }),
                        )
                        .child(setting_row(
                            "Lock now",
                            Some("Start the screen locker immediately.".to_string()),
                            true,
                            secondary_button("Lock", || system::trigger_lock()),
                        )),
                )
                // Firewall: live toggle when a manageable backend exists.
                .child(
                    tile()
                        .child(tile_head(
                            Some(LOCK),
                            "Firewall",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(fw_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(fw_chip.0, fw_chip.1, None)),
                            ),
                        ))
                        .child(tile_sub(fw_detail))
                        .child(setting_row(
                            "Block unauthorized network access",
                            Some(if fw_live {
                                "Enforced with ufw. Needs root; denial rolls back.".to_string()
                            } else if fw_managed {
                                "Managed by policy.".to_string()
                            } else {
                                "Unavailable: no manageable firewall backend.".to_string()
                            }),
                            false,
                            {
                                let s = store;
                                rect().opacity(if fw_live { 1.0 } else { 0.45 }).child(pill_switch(
                                    fw_on,
                                    move |v| {
                                        if fw_live {
                                            s.set(SettingKey::FirewallEnabled, v);
                                        }
                                    },
                                ))
                            },
                        ))
                        .child(setting_row(
                            "Re-probe firewall",
                            Some("Re-run the same detection as on open.".to_string()),
                            true,
                            secondary_button("Refresh", {
                                let mut l = loading;
                                let mut f = fw;
                                move || {
                                    l.set(true);
                                    f.set(None);
                                    spawn(async move {
                                        let next = tokio::task::spawn_blocking(system::query_firewall)
                                            .await
                                            .ok();
                                        if let Some(v) = next {
                                            f.set(Some(v));
                                        }
                                        l.set(false);
                                    });
                                }
                            }),
                        )),
                )
                // Location portal.
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Location",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(loc_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(loc_chip.0, loc_chip.1, None)),
                            ),
                        ))
                        .child(tile_sub(loc_detail))
                        .child(setting_row(
                            "Location services",
                            Some(if loc_live {
                                "Enforced with the GNOME location portal.".to_string()
                            } else if loc_managed {
                                "Managed by policy.".to_string()
                            } else {
                                "Unavailable: location portal not found.".to_string()
                            }),
                            false,
                            {
                                let s = store;
                                rect().opacity(if loc_live { 1.0 } else { 0.45 }).child(pill_switch(
                                    loc_on,
                                    move |v| {
                                        if loc_live {
                                            s.set(SettingKey::LocationAccess, v);
                                        }
                                    },
                                ))
                            },
                        )),
                )
                // Camera: honest Unavailable, no enforcing toggle.
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Camera",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(cam_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(
                                        cam_lock.as_ref().map(|_| "Managed").unwrap_or("Unavailable"),
                                        false,
                                        None,
                                    )),
                            ),
                        ))
                        .child(tile_sub(cam_sub))
                        .child(setting_row(
                            format!("Camera access ({})", if cam_on { "allowed" } else { "denied" }),
                            Some(
                                "Unavailable: no camera inhibit backend ships with Finick. Cover the lens or unplug the device to enforce."
                                    .to_string(),
                            ),
                            false,
                            {
                                rect().opacity(0.45).child(pill_switch(cam_on, move |_| {
                                    // Disabled: no enforceable backend, so the toggle never claims control.
                                }))
                            },
                        )),
                )
                // Microphone: live mute via PipeWire.
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Microphone",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(mic_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(mic_chip.0, mic_chip.1, None)),
                            ),
                        ))
                        .child(tile_sub(mic_detail))
                        .child(setting_row(
                            "Microphone access",
                            Some(if mic_live {
                                "Enforced by muting the PipeWire source.".to_string()
                            } else if mic_managed {
                                "Managed by policy.".to_string()
                            } else {
                                "Unavailable: PipeWire controls not found.".to_string()
                            }),
                            false,
                            {
                                let s = store;
                                rect().opacity(if mic_live { 1.0 } else { 0.45 }).child(pill_switch(
                                    mic_on,
                                    move |v| {
                                        if mic_live {
                                            s.set(SettingKey::MicAccess, v);
                                        }
                                    },
                                ))
                            },
                        )),
                )
                // USB protection.
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "USB devices",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(usb_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(usb_chip.0, usb_chip.1, None)),
                            ),
                        ))
                        .child(tile_sub(usb_detail))
                        .child(setting_row(
                            "Block new USB devices",
                            Some(if usb_live {
                                "Detected with USBGuard. Policy edits need root.".to_string()
                            } else if usb_managed {
                                "Managed by policy.".to_string()
                            } else {
                                "Unavailable: USBGuard not installed.".to_string()
                            }),
                            false,
                            {
                                let s = store;
                                rect().opacity(if usb_live { 1.0 } else { 0.45 }).child(pill_switch(
                                    usb_on,
                                    move |v| {
                                        if usb_live {
                                            s.set(usb_key(), v);
                                        }
                                    },
                                ))
                            },
                        )),
                )
                // File history with destructive confirm.
                .child(
                    tile()
                        .child(tile_head(None, "File history", Some(status_chip("Live", true, None))))
                        .child(tile_sub(
                            "Clear recent files, trash contents and shell history on this machine.",
                        ))
                        .maybe(!armed, |el| {
                            el.child(setting_row(
                                "Clear history",
                                Some("Recent files, trash and shell history. Asks first.".to_string()),
                                false,
                                {
                                    let mut a = hist_armed;
                                    secondary_button("Clear…", move || a.set(true))
                                },
                            ))
                        })
                        .maybe(armed, |el| {
                            el.child(tile_sub(
                                "Removes recently-used entries, empties trash and truncates shell history. This cannot be undone.",
                            ))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .content(Content::Flex)
                                    .child({
                                        let mut a = hist_armed;
                                        let mut b = hist_busy;
                                        let mut m = hist_msg;
                                        danger_button("Clear now", move || {
                                            a.set(false);
                                            b.set(true);
                                            m.set(None);
                                            spawn(async move {
                                                let res = tokio::task::spawn_blocking(|| {
                                                    system::HyprlandBackend.clear_history()
                                                })
                                                .await
                                                .unwrap_or(Err("Task failed".to_string()));
                                                b.set(false);
                                                match res {
                                                    Ok(r) => {
                                                        let shells = if r.shells_truncated.is_empty() {
                                                            "shell history untouched".to_string()
                                                        } else {
                                                            format!("truncated {}", r.shells_truncated.join(", "))
                                                        };
                                                        m.set(Some((
                                                            format!(
                                                                "Cleared recent files, emptied {} trashed item{}. {}",
                                                                r.trash_emptied,
                                                                if r.trash_emptied == 1 { "" } else { "s" },
                                                                shells
                                                            ),
                                                            true,
                                                        )));
                                                    }
                                                    Err(e) => m.set(Some((e, false))),
                                                }
                                            });
                                        })
                                    })
                                    .child({
                                        let mut a = hist_armed;
                                        ghost_button("Cancel", move || a.set(false))
                                    }),
                            )
                        })
                        .maybe(busy, |el| el.child(tile_sub("Clearing history…")))
                        .maybe_child(hist_state.map(|(msg, ok)| {
                            rect().margin((8., 0., 0., 0.)).child(notice_pill(
                                msg,
                                if ok { NoticeKind::Success } else { NoticeKind::Error },
                            ))
                        })),
                )
                // Diagnostics: honest local-only toggle.
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Diagnostics",
                            Some(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .maybe_child(diag_lock.clone().map(|l| lock_badge(l)))
                                    .child(status_chip(diag_chip.0, diag_chip.1, None)),
                            ),
                        ))
                        .child(tile_sub("Local only. Finick never uploads diagnostics."))
                        .child(setting_row(
                            "Share diagnostics with Finick",
                            Some("Stores a local preference. No reports leave this machine.".to_string()),
                            false,
                            {
                                let s = store;
                                rect().opacity(if diag_managed { 0.45 } else { 1.0 }).child(pill_switch(
                                    diag_on,
                                    move |v| {
                                        if !diag_managed {
                                            s.set(diagnostics_key(), v);
                                        }
                                    },
                                ))
                            },
                        )),
                )
                .child({
                    let _ = t;
                    rect().margin((2., 0., 0., 0.)).child(tile_sub(
                        "Live rows enforce through the settings daemon. Managed rows are locked by policy. Unavailable rows name the missing backend.",
                    ))
                }),
        )
    }
}
