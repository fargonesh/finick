use {crate::state::SettingsStore, freya::prelude::*, ipsea::settings::SettingKey, std::process::Command, ui::*};

#[derive(Clone, Debug, PartialEq)]
struct SysUser {
    name: String,
    uid: String,
    shell: String,
    admin: bool,
}

fn list_users() -> Vec<SysUser> {
    let mut out = Vec::new();
    if let Ok(text) = std::fs::read_to_string("/etc/passwd") {
        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 7 {
                continue;
            }
            let uid: u32 = parts[2].parse().unwrap_or(0);
            if uid >= 1000 && uid < 60000 {
                out.push(SysUser {
                    name: parts[0].to_string(),
                    uid: parts[2].to_string(),
                    shell: parts[6].to_string(),
                    admin: std::fs::read_to_string("/etc/group")
                        .map(|g| g.lines().any(|l| l.starts_with("wheel:") && l.contains(parts[0])))
                        .unwrap_or(false),
                });
            }
        }
    }
    if out.is_empty() {
        if let Ok(user) = std::env::var("USER") {
            out.push(SysUser { name: user, uid: "1000".to_string(), shell: "/bin/bash".to_string(), admin: true });
        }
    }
    out
}

fn list_timezones() -> Vec<String> {
    if let Ok(o) = Command::new("timedatectl").arg("list-timezones").output() {
        let s = String::from_utf8_lossy(&o.stdout).to_string();
        let v: Vec<String> = s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).take(60).collect();
        if !v.is_empty() {
            return v;
        }
    }
    vec![
        "UTC".to_string(),
        "Australia/Sydney".to_string(),
        "Australia/Melbourne".to_string(),
        "Europe/London".to_string(),
        "Europe/Berlin".to_string(),
        "America/New_York".to_string(),
        "America/Los_Angeles".to_string(),
        "Asia/Tokyo".to_string(),
    ]
}

fn current_tz() -> String {
    if let Ok(o) = Command::new("timedatectl").arg("show").arg("--property=Timezone").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if let Some(v) = s.strip_prefix("Timezone=") {
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    "UTC".to_string()
}

fn nixos_present() -> bool {
    Command::new("which").arg("nixos-rebuild").output().map(|o| o.status.success()).unwrap_or(false)
}

fn tz_key() -> SettingKey {
    SettingKey::Custom("system.timezone".to_string())
}
fn locale_key() -> SettingKey {
    SettingKey::Custom("system.locale".to_string())
}

#[derive(PartialEq)]
pub struct System {
    pub store: SettingsStore,
}

impl Component for System {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let users: State<Vec<SysUser>> = use_state(Vec::new);
        let loading: State<bool> = use_state(|| true);
        let tz: State<String> = use_state(|| "UTC".to_string());
        let tz_opts: State<Vec<String>> = use_state(Vec::new);
        let locale: State<String> = use_state(|| "en_US.UTF-8".to_string());
        let new_user: State<String> = use_state(String::new);
        let busy: State<bool> = use_state(|| false);
        let msg: State<Option<(String, bool)>> = use_state(|| None);
        let armed_rm: State<Option<String>> = use_state(|| None);

        use_hook(move || {
            let mut u = users;
            let mut ld = loading;
            let mut t = tz;
            let mut to = tz_opts;
            let mut lc = locale;
            spawn(async move {
                let res = tokio::task::spawn_blocking(|| {
                    let us = list_users();
                    let cur_tz = current_tz();
                    let zones = list_timezones();
                    let cur_locale = std::process::Command::new("localectl")
                        .arg("status")
                        .output()
                        .ok()
                        .and_then(|o| {
                            let s = String::from_utf8_lossy(&o.stdout).to_string();
                            s.lines().find_map(|l| {
                                let t = l.trim();
                                t.strip_prefix("System Locale:").and_then(|rest| {
                                    rest.split_whitespace().find_map(|p| p.strip_prefix("LANG="))
                                }).map(|v| v.trim_matches('"').to_string())
                            })
                        })
                        .unwrap_or_else(|| "en_US.UTF-8".to_string());
                    (us, cur_tz, zones, cur_locale)
                })
                .await
                .unwrap_or_default();
                u.set(res.0);
                t.set(res.1);
                to.set(res.2);
                lc.set(res.3);
                ld.set(false);
            });
        });

        let user_list = users.read().clone();
        let is_loading = *loading.read();
        let tz_val = tz.read().clone();
        let zones = tz_opts.read().clone();
        let locale_val = locale.read().clone();
        let msg_opt = msg.read().clone();
        let is_busy = *busy.read();

        let run_pkexec = move |args: Vec<String>, mut b: State<bool>, mut m: State<Option<(String, bool)>>, ok_msg: String| {
            b.set(true);
            spawn(async move {
                let res = tokio::task::spawn_blocking(move || {
                    Command::new("pkexec").args(&args).output().map_err(|e| e.to_string()).and_then(|o| {
                        if o.status.success() {
                            Ok(())
                        } else {
                            Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                        }
                    })
                })
                .await
                .unwrap_or(Err("Task failed".to_string()));
                b.set(false);
                match res {
                    Ok(()) => m.set(Some((ok_msg, true))),
                    Err(e) => m.set(Some((
                        if e.is_empty() {
                            "Polkit denied or command failed. Nothing was changed.".to_string()
                        } else {
                            e
                        },
                        false,
                    ))),
                }
            });
        };

        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(GENERAL, "System", "Users, updates, region, time zone and remote access."))
            .maybe_child(msg_opt.map(|(m, ok)| notice_pill(m, if ok { NoticeKind::Success } else { NoticeKind::Error })));

        // Users
        let mut users_tile = tile().child(tile_head(
            None,
            format!("Users ({})", user_list.len()),
            if is_loading { Some(status_chip("Loading", false, None)) } else { None },
        ));
        if is_loading && user_list.is_empty() {
            users_tile = users_tile.child(tile_sub("Reading /etc/passwd…"));
        } else {
            for u in user_list.iter().take(10) {
                let name = u.name.clone();
                let is_armed = armed_rm.read().clone().as_deref() == Some(name.as_str());
                let control: Element = if !is_armed {
                    let mut armed = armed_rm;
                    let n = name.clone();
                    secondary_button("Remove…", move || {
                        armed.set(Some(n.clone()));
                        let id = n.clone();
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            if armed.read().clone().as_deref() == Some(id.as_str()) {
                                armed.set(None);
                            }
                        });
                    })
                    .into_element()
                } else {
                    let mut armed = armed_rm;
                    let b = busy;
                    let m = msg;
                    let n = name.clone();
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .content(Content::Flex)
                        .child(danger_button("Sure? Delete", move || {
                            armed.set(None);
                            run_pkexec(vec!["userdel".to_string(), "-r".to_string(), n.clone()], b, m, format!("Removed {n}"));
                        }))
                        .child(ghost_button("Cancel", move || armed.set(None)))
                        .into_element()
                };
                users_tile = users_tile.child(setting_row(
                    format!("{} (UID {})", u.name, u.uid),
                    Some(u.shell.clone()),
                    false,
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .content(Content::Flex)
                        .child(status_chip(if u.admin { "Admin" } else { "User" }, u.admin, None))
                        .child(control),
                ));
            }
        }
        users_tile = users_tile.child(rect().margin((10., 0., 0., 0.)).child(field_label("Add user (needs polkit)")));
        users_tile = users_tile.child(search_field(new_user.into(), "username", None));
        users_tile = users_tile.child(rect().margin((8., 0., 0., 0.)).child({
            let mut b = busy;
            let mut m = msg;
            let mut nu = new_user;
            let us = users;
            secondary_button(if is_busy { "Working…" } else { "Add user" }, move || {
                let name = nu.read().trim().to_string();
                if name.is_empty() {
                    return;
                }
                nu.set(String::new());
                b.set(true);
                let mut us = us;
                let name_for_cmd = name.clone();
                let name_for_msg = name.clone();
                spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        Command::new("pkexec").args(["useradd", "-m", &name_for_cmd]).output()
                    })
                    .await;
                    b.set(false);
                    match res {
                        Ok(Ok(o)) if o.status.success() => {
                            m.set(Some((format!("Created {name_for_msg}. Set a password with passwd."), true)));
                            let refreshed = tokio::task::spawn_blocking(list_users).await.unwrap_or_default();
                            us.set(refreshed);
                        }
                        _ => m.set(Some(("Could not add user. Polkit denied or useradd failed.".to_string(), false))),
                    }
                });
            })
        }));
        users_tile = users_tile.child(tile_sub("Password changes run pkexec passwd <user> in a terminal; Finick never stores passwords."));
        base = base.child(users_tile);

        // Region + timezone
        {
            let tz_options: Vec<DropdownOption> = zones.iter().map(|z| DropdownOption { label: z.clone(), value: z.clone() }).collect();
            base = base.child(
                tile()
                    .child(tile_head(None, "Region and time zone", None::<String>))
                    .child(setting_row(
                        "System locale",
                        Some("Writes via localectl set-locale (needs root)".to_string()),
                        false,
                        {
                            let lc = locale;
                            let s = store;
                            let mut b = busy;
                            let mut m = msg;
                            secondary_button(format!("Apply {}", locale_val.clone()), move || {
                                let v = lc.read().clone();
                                let v_cmd = v.clone();
                                let v_msg = v.clone();
                                b.set(true);
                                spawn(async move {
                                    let res = tokio::task::spawn_blocking(move || {
                                        Command::new("pkexec").args(["localectl", "set-locale", &format!("LANG={v_cmd}")]).output()
                                    })
                                    .await;
                                    b.set(false);
                                    match res {
                                        Ok(Ok(o)) if o.status.success() => {
                                            s.set(locale_key(), v_msg.clone());
                                            m.set(Some((format!("Locale set to {v_msg}"), true)));
                                        }
                                        _ => m.set(Some(("localectl failed or was denied. Locale unchanged.".to_string(), false))),
                                    }
                                });
                            })
                        },
                    ))
                    .child(field_label(format!("Time zone · {tz_val}")))
                    .child(dropdown_select(
                        tz_val.clone(),
                        tz_options,
                        EventHandler::new(move |v: String| {
                            let mut t = tz;
                            let s = store;
                            let mut m = msg;
                            let mut b = busy;
                            t.set(v.clone());
                            b.set(true);
                            let v_cmd = v.clone();
                            let v_msg = v.clone();
                            spawn(async move {
                                let res = tokio::task::spawn_blocking(move || {
                                    Command::new("pkexec").args(["timedatectl", "set-timezone", &v_cmd]).output()
                                })
                                .await;
                                b.set(false);
                                match res {
                                    Ok(Ok(o)) if o.status.success() => {
                                        s.set(tz_key(), v_msg.clone());
                                        m.set(Some((format!("Time zone set to {v_msg}"), true)));
                                    }
                                    _ => m.set(Some(("Could not set time zone. Polkit denied or timedatectl failed.".to_string(), false))),
                                }
                            });
                        }),
                    )),
            );
        }

        // Updates
        {
            let has_nixos = nixos_present();
            base = base.child(
                tile()
                    .child(tile_head(
                        None,
                        "Software update",
                        Some(status_chip(if has_nixos { "nixos-rebuild" } else { "Unavailable" }, has_nixos, None)),
                    ))
                    .child(tile_sub(if has_nixos {
                        "Runs a real nixos-rebuild dry-build check. Apply happens in a terminal."
                    } else {
                        "Unavailable: nixos-rebuild not found on this machine."
                    }))
                    .child(setting_row(
                        "Check for updates",
                        None::<String>,
                        false,
                        {
                            let mut b = busy;
                            let mut m = msg;
                            secondary_button(if is_busy { "Checking…" } else { "Check now" }, move || {
                                if !nixos_present() {
                                    m.set(Some(("nixos-rebuild not found. Nothing to check.".to_string(), false)));
                                    return;
                                }
                                b.set(true);
                                m.set(None);
                                spawn(async move {
                                    let res = tokio::task::spawn_blocking(|| {
                                        Command::new("nixos-rebuild").args(["dry-build", "--show-trace"]).output().map_err(|e| e.to_string())
                                    })
                                    .await;
                                    b.set(false);
                                    match res {
                                        Ok(Ok(o)) if o.status.success() => {
                                            m.set(Some(("System is up to date (dry-build succeeded).".to_string(), true)))
                                        }
                                        Ok(Ok(o)) => {
                                            let err = String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("dry-build reported changes").to_string();
                                            m.set(Some((format!("Updates available: {err}"), true)));
                                        }
                                        _ => m.set(Some(("Update check failed to run.".to_string(), false))),
                                    }
                                });
                            })
                        },
                    )),
            );
        }
        FadeSlideIn::new().child(base)
    }
}
