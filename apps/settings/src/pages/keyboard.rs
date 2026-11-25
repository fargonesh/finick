use {crate::state::SettingsStore, freya::prelude::*, ipsea::settings::SettingKey, std::process::Command, system::HyprlandBackend, ui::*};

fn kb_layout_key() -> SettingKey {
    SettingKey::Custom("keyboard.layout".to_string())
}
fn repeat_key() -> SettingKey {
    SettingKey::Custom("keyboard.repeat_rate".to_string())
}
fn delay_key() -> SettingKey {
    SettingKey::Custom("keyboard.repeat_delay".to_string())
}

fn current_layout() -> String {
    if let Ok(o) = Command::new("hyprctl").args(["getoption", "input:kb_layout"]).output() {
        let s = String::from_utf8_lossy(&o.stdout).to_string();
        for line in s.lines() {
            let t = line.trim();
            if let Some(v) = t.strip_prefix("str:") {
                let clean = v.trim().trim_matches('"');
                if !clean.is_empty() {
                    return clean.to_string();
                }
            }
        }
    }
    if let Ok(o) = Command::new("localectl").arg("status").output() {
        let s = String::from_utf8_lossy(&o.stdout).to_string();
        for line in s.lines() {
            if let Some(rest) = line.trim().strip_prefix("X11 Layout:") {
                let l = rest.trim();
                if !l.is_empty() {
                    return l.to_string();
                }
            }
        }
    }
    "us".to_string()
}

fn apply_layout(layout: &str) -> Result<(), String> {
    HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ input = {{ kb_layout = \"{layout}\" }} }})"),
        "input:kb_layout",
        layout,
    )
}

fn apply_repeat(rate: i64, delay: i64) -> Result<(), String> {
    HyprlandBackend::hyprctl_set_config(
        &format!("hl.config({{ input = {{ repeat_rate = {rate}, repeat_delay = {delay} }} }})"),
        "input:repeat_rate",
        &rate.to_string(),
    )
    .and_then(|_| {
        HyprlandBackend::hyprctl_set_config(
            &format!("hl.config({{ input = {{ repeat_delay = {delay} }} }})"),
            "input:repeat_delay",
            &delay.to_string(),
        )
    })
}

const LAYOUTS: &[(&str, &str)] = &[
    ("us", "English (US)"),
    ("gb", "English (UK)"),
    ("de", "German"),
    ("fr", "French"),
    ("es", "Spanish"),
    ("it", "Italian"),
    ("ru", "Russian"),
    ("jp", "Japanese"),
];

#[derive(PartialEq)]
pub struct Keyboard {
    pub store: SettingsStore,
}

impl Component for Keyboard {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let layouts: State<Vec<String>> = use_state(|| vec!["us".to_string()]);
        let active: State<String> = use_state(|| "us".to_string());
        let rate: State<i64> = use_state(|| 30);
        let delay: State<i64> = use_state(|| 250);
        let tab: State<usize> = use_state(|| 0usize);
        let err: State<Option<String>> = use_state(|| None);
        let armed_remove: State<Option<String>> = use_state(|| None);

        use_hook(move || {
            let mut ac = active;
            spawn(async move {
                let cur = tokio::task::spawn_blocking(current_layout).await.unwrap_or("us".to_string());
                ac.set(cur);
            });
        });

        let cur_tab = *tab.read();
        let cur_rate = *rate.read();
        let cur_delay = *delay.read();
        let cur_active = active.read().clone();
        let err_opt = err.read().clone();
        let layout_lock = store.lock_label(&kb_layout_key());

        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(KEYBOARD, "Keyboard", "Input sources, typing repeat and shortcuts."))
            .maybe_child(err_opt.map(|m| {
                rect()
                    .width(Size::fill())
                    .child(notice_pill(m, NoticeKind::Error))
                    .child(rect().margin((6., 0., 0., 0.)).child(ghost_button("Dismiss", {
                        let mut e = err;
                        move || e.set(None)
                    })))
            }))
            .child(tile().child(tile_head(
                None,
                "Section",
                Some(segmented_control(vec![("Layouts", 0usize), ("Typing", 1usize), ("Shortcuts", 2usize)], cur_tab, {
                    let mut t = tab;
                    move |v| t.set(v)
                })),
            )));

        if cur_tab == 0 {
            let mut t = tile().child(tile_head(
                None,
                format!("Input sources ({})", layouts.read().len()),
                layout_lock.clone().map(lock_badge),
            ));
            t = t.child(tile_sub("Active layout applies via hyprctl keyword input:kb_layout and persists via the daemon."));
            for code in layouts.read().clone() {
                let friendly = LAYOUTS.iter().find(|(c, _)| *c == code).map(|(_, n)| *n).unwrap_or("Custom layout");
                let is_active = code == cur_active;
                let is_armed = armed_remove.read().clone().as_deref() == Some(code.as_str());
                let control: Element = if is_armed {
                    let mut armed = armed_remove;
                    let mut ls = layouts;
                    let c = code.clone();
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .content(Content::Flex)
                        .child(danger_button("Sure? Remove", move || {
                            let mut v = ls.read().clone();
                            v.retain(|x| x != &c);
                            ls.set(v);
                            armed.set(None);
                        }))
                        .child(ghost_button("Cancel", move || armed.set(None)))
                        .into_element()
                } else if is_active {
                    status_chip("Active", true, None).into_element()
                } else {
                    let ac = active;
                    let e = err;
                    let s = store;
                    let managed = layout_lock.is_some();
                    let c = code.clone();
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .content(Content::Flex)
                        .child(secondary_button("Use", move || {
                            if managed {
                                return;
                            }
                            let target = c.clone();
                            let applied = c.clone();
                            let mut ac = ac;
                            let mut e = e;
                            let s = s;
                            spawn(async move {
                                let res = tokio::task::spawn_blocking(move || apply_layout(&target)).await.unwrap_or(Err("Task failed".to_string()));
                                match res {
                                    Ok(()) => {
                                        ac.set(applied.clone());
                                        s.set(kb_layout_key(), applied);
                                    }
                                    Err(msg) => e.set(Some(msg)),
                                }
                            });
                        }))
                        .child({
                            let mut armed = armed_remove;
                            let c2 = code.clone();
                            ghost_button("Remove", move || {
                                armed.set(Some(c2.clone()));
                                let id = c2.clone();
                                spawn(async move {
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                    if armed.read().clone().as_deref() == Some(id.as_str()) {
                                        armed.set(None);
                                    }
                                });
                            })
                        })
                        .into_element()
                };
                t = t.child(setting_row(format!("{code} · {friendly}"), Some(if is_active { "Preview: AaBbCc 123" } else { "Preview on activation" }), false, control));
            }
            let opts: Vec<DropdownOption> = LAYOUTS
                .iter()
                .filter(|(c, _)| !layouts.read().contains(&c.to_string()))
                .map(|(c, n)| DropdownOption { label: format!("{c} · {n}"), value: c.to_string() })
                .collect();
            if !opts.is_empty() {
                t = t.child(rect().margin((10., 0., 0., 0.)).child(field_label("Add layout")));
                t = t.child(dropdown_select(
                    "Select layout…".to_string(),
                    opts,
                    EventHandler::new(move |v: String| {
                        let mut ls = layouts;
                        let mut cur = ls.read().clone();
                        if !cur.contains(&v) {
                            cur.push(v);
                            ls.set(cur);
                        }
                    }),
                ));
            }
            base = base.child(t);
        } else if cur_tab == 1 {
            base = base.child(
                tile()
                    .child(tile_head(None, "Typing", None::<String>))
                    .child(field_label(format!("Repeat rate · {cur_rate} keys/sec")))
                    .child({
                        let mut r = rate;
                        let d = delay;
                        let e = err;
                        let s = store;
                        slider_row(None, cur_rate as f64, move |v| {
                            let next = v.clamp(5.0, 60.0) as i64;
                            r.set(next);
                            let delay_v = *d.read();
                            let mut e = e;
                            spawn(async move {
                                let res = tokio::task::spawn_blocking(move || apply_repeat(next, delay_v))
                                    .await
                                    .unwrap_or(Err("Task failed".to_string()));
                                if let Err(msg) = res {
                                    e.set(Some(msg));
                                } else {
                                    s.set(repeat_key(), next);
                                }
                            });
                        })
                    })
                    .child(rect().margin((10., 0., 0., 0.)).child(field_label(format!("Repeat delay · {cur_delay} ms"))))
                    .child({
                        let mut d = delay;
                        let r = rate;
                        let e = err;
                        let s = store;
                        slider_row(None, (cur_delay as f64).clamp(0.0, 100.0), move |v| {
                            let next = (v * 10.0).clamp(100.0, 1000.0) as i64;
                            d.set(next);
                            let rate_v = *r.read();
                            let mut e = e;
                            spawn(async move {
                                let res = tokio::task::spawn_blocking(move || apply_repeat(rate_v, next))
                                    .await
                                    .unwrap_or(Err("Task failed".to_string()));
                                if let Err(msg) = res {
                                    e.set(Some(msg));
                                } else {
                                    s.set(delay_key(), next);
                                }
                            });
                        })
                    })
                    .child(tile_sub("Repeat applies live via input:repeat_rate / input:repeat_delay.")),
            );
        } else {
            base = base.child(
                tile()
                    .child(tile_head(None, "Shortcuts", Some(status_chip("Unchanged", false, None))))
                    .child(tile_sub("Custom keybindings live under System → Shortcuts. Layout changes do not alter shortcuts.")),
            );
        }
        FadeSlideIn::new().child(base)
    }
}
