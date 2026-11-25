use {crate::state::SettingsStore, freya::prelude::*, ipsea::settings::SettingKey, std::process::Command, ui::*};

fn svc_active(unit: &str) -> Option<bool> {
    Command::new("systemctl")
        .args(["is-active", unit])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
}

fn bin_present(bin: &str) -> bool {
    Command::new("which").arg(bin).output().map(|o| o.status.success()).unwrap_or(false)
}

fn local_addresses() -> String {
    if let Ok(o) = Command::new("hostname").arg("-I").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() {
            return s.split_whitespace().take(2).collect::<Vec<_>>().join(", ");
        }
    }
    String::from_utf8_lossy(
        &Command::new("hostname").output().map(|o| o.stdout).unwrap_or_default(),
    )
    .trim()
    .to_string()
}

fn set_service(unit: &str, enable: bool) -> Result<(), String> {
    let action = if enable { "start" } else { "stop" };
    let o = Command::new("systemctl").args([action, unit]).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

fn rdp_key() -> SettingKey {
    SettingKey::Custom("sharing.rdp".to_string())
}
fn ssh_key() -> SettingKey {
    SettingKey::Custom("sharing.ssh".to_string())
}
fn media_key() -> SettingKey {
    SettingKey::Custom("sharing.media".to_string())
}

#[derive(PartialEq)]
pub struct Sharing {
    pub store: SettingsStore,
}

impl Component for Sharing {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let rdp_on: State<bool> = use_state(|| false);
        let ssh_on: State<bool> = use_state(|| false);
        let media_on: State<bool> = use_state(|| false);
        let rdp_avail: State<Option<bool>> = use_state(|| None);
        let ssh_avail: State<Option<bool>> = use_state(|| None);
        let addrs: State<String> = use_state(String::new);
        let busy: State<bool> = use_state(|| false);
        let err: State<Option<String>> = use_state(|| None);

        use_hook(move || {
            let mut r = rdp_on;
            let mut s = ssh_on;
            let mut ra = rdp_avail;
            let mut sa = ssh_avail;
            let mut ad = addrs;
            spawn(async move {
                let res = tokio::task::spawn_blocking(|| {
                    let rdp = if bin_present("grdctl") || bin_present("gnome-remote-desktop-daemon") {
                        svc_active("gnome-remote-desktop.service")
                    } else {
                        None
                    };
                    let ssh = if bin_present("sshd") || std::path::Path::new("/run/current-system/sw/bin/sshd").exists() {
                        svc_active("sshd.service")
                    } else {
                        svc_active("sshd.service")
                    };
                    let addresses = local_addresses();
                    (rdp, ssh, addresses)
                })
                .await
                .unwrap_or((None, None, String::new()));
                ra.set(res.0);
                sa.set(res.1);
                r.set(res.0.unwrap_or(false));
                s.set(res.1.unwrap_or(false));
                ad.set(res.2);
            });
        });

        let rdp_state = *rdp_avail.read();
        let ssh_state = *ssh_avail.read();
        let addr_str = addrs.read().clone();
        let is_busy = *busy.read();
        let err_opt = err.read().clone();
        let rdp_lock = store.lock_label(&rdp_key());
        let ssh_lock = store.lock_label(&ssh_key());
        let media_lock = store.lock_label(&media_key());
        let rygel_present = bin_present("rygel") || bin_present("minidlnad");

        let apply_toggle = move |unit: &'static str, v: bool, mut flag: State<bool>, mut e: State<Option<String>>, mut st: State<bool>, key: SettingKey, s: SettingsStore| {
            flag.set(true);
            e.set(None);
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || set_service(unit, v)).await.unwrap_or(Err("Task failed".to_string()));
                flag.set(false);
                match res {
                    Ok(()) => {
                        st.set(v);
                        s.set(key, v);
                    }
                    Err(msg) => e.set(Some(if msg.is_empty() {
                        format!("Could not change {unit} (needs root or service missing)")
                    } else {
                        msg
                    })),
                }
            });
        };

        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(
                WIRED,
                "Sharing",
                "Screen share, remote login and media. Unavailable backends stay unavailable.",
            ))
            .maybe_child(err_opt.map(|m| {
                rect()
                    .width(Size::fill())
                    .child(notice_pill(m, NoticeKind::Error))
                    .child(rect().margin((6., 0., 0., 0.)).child(ghost_button("Dismiss", {
                        let mut e = err;
                        move || e.set(None)
                    })))
            }));

        // RDP card
        {
            let managed = rdp_lock.is_some();
            let live = !managed && rdp_state.is_some();
            let chip = rdp_lock.clone().map(|_| "Managed").unwrap_or_else(|| {
                if rdp_state.is_none() {
                    "Unavailable"
                } else if *rdp_on.read() {
                    "Live"
                } else {
                    "Off"
                }
            });
            let on = *rdp_on.read();
            let mut card = tile()
                .child(tile_head(
                    None,
                    "Remote desktop (RDP)",
                    Some(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .cross_align(Alignment::Center)
                            .content(Content::Flex)
                            .maybe_child(rdp_lock.clone().map(lock_badge))
                            .child(status_chip(chip, on && live, None)),
                    ),
                ))
                .child(tile_sub(if rdp_state.is_none() {
                    "gnome-remote-desktop not found (grdctl missing). No fake RDP switch is offered.".to_string()
                } else {
                    format!("Address: {} · port 3389", if addr_str.is_empty() { "unknown" } else { &addr_str })
                }));
            card = card.child(setting_row(
                "Allow remote desktop connections",
                Some(if live {
                    "Controls gnome-remote-desktop.service. Needs root; denial rolls back.".to_string()
                } else if managed {
                    "Locked by policy.".to_string()
                } else {
                    "Unavailable: install gnome-remote-desktop to enable.".to_string()
                }),
                false,
                {
                    let flag = busy;
                    let e = err;
                    let st = rdp_on;
                    let s = store;
                    rect().opacity(if live && !is_busy { 1.0 } else { 0.45 }).child(pill_switch(on, move |v| {
                        if live {
                            apply_toggle("gnome-remote-desktop.service", v, flag, e, st, rdp_key(), s);
                        }
                    }))
                },
            ));
            base = base.child(card);
        }

        // SSH card
        {
            let managed = ssh_lock.is_some();
            let live = !managed && ssh_state.is_some();
            let chip = ssh_lock.clone().map(|_| "Managed").unwrap_or_else(|| {
                if ssh_state.is_none() {
                    "Unavailable"
                } else if *ssh_on.read() {
                    "Live"
                } else {
                    "Off"
                }
            });
            let on = *ssh_on.read();
            let mut card = tile()
                .child(tile_head(
                    None,
                    "Remote login (SSH)",
                    Some(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .cross_align(Alignment::Center)
                            .content(Content::Flex)
                            .maybe_child(ssh_lock.clone().map(lock_badge))
                            .child(status_chip(chip, on && live, None)),
                    ),
                ))
                .child(tile_sub(format!(
                    "Address: {} · port 22",
                    if addr_str.is_empty() { "unknown" } else { &addr_str }
                )));
            card = card.child(setting_row(
                "Allow SSH connections",
                Some(if live {
                    "Controls sshd.service. Needs root; denial rolls back.".to_string()
                } else if managed {
                    "Locked by policy.".to_string()
                } else {
                    "Unavailable: sshd.service not found.".to_string()
                }),
                false,
                {
                    let flag = busy;
                    let e = err;
                    let st = ssh_on;
                    let s = store;
                    rect().opacity(if live && !is_busy { 1.0 } else { 0.45 }).child(pill_switch(on, move |v| {
                        if live {
                            apply_toggle("sshd.service", v, flag, e, st, ssh_key(), s);
                        }
                    }))
                },
            ));
            base = base.child(card);
        }

        // Media card (honest local pref, no fake rygel control)
        {
            let managed = media_lock.is_some();
            let on = *media_on.read();
            let chip = media_lock.clone().map(|_| "Managed").unwrap_or_else(|| if rygel_present { "Live" } else { "Unavailable" });
            base = base.child(
                tile()
                    .child(tile_head(
                        None,
                        "Media sharing",
                        Some(
                            rect()
                                .horizontal()
                                .spacing(8.)
                                .cross_align(Alignment::Center)
                                .content(Content::Flex)
                                .maybe_child(media_lock.clone().map(lock_badge))
                                .child(status_chip(chip, on && rygel_present, None)),
                        ),
                    ))
                    .child(tile_sub(if rygel_present {
                        "Shares music and video on the local network via Rygel when running.".to_string()
                    } else {
                        "Unavailable: rygel/minidlna not installed. Preference is stored but nothing is shared.".to_string()
                    }))
                    .child(setting_row(
                        "Share media on this network",
                        Some("Stored locally. Starts nothing when the backend is missing.".to_string()),
                        false,
                        {
                            let s = store;
                            let mut m = media_on;
                            rect().opacity(if managed { 0.45 } else { 1.0 }).child(pill_switch(on, move |v| {
                                if !managed {
                                    m.set(v);
                                    s.set(media_key(), v);
                                }
                            }))
                        },
                    )),
            );
        }

        FadeSlideIn::new().child(base)
    }
}
