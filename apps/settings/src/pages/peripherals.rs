use {crate::state::SettingsStore, freya::prelude::*, ipsea::settings::SettingKey, std::process::Command, ui::*};

fn tablet_present() -> Option<String> {
    if let Ok(o) = Command::new("libwacom-list-local-devices").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if o.status.success() && !s.is_empty() && !s.contains("0 devices") {
            return Some(s.lines().next().unwrap_or("Wacom tablet").to_string());
        }
    }
    let mut found: Option<String> = None;
    if let Ok(entries) = std::fs::read_dir("/dev/input") {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower.contains("wacom") || lower.contains("tablet") {
                found = Some(name);
                break;
            }
        }
    }
    if found.is_some() {
        return found;
    }
    if let Ok(o) = Command::new("lsusb").output() {
        let s = String::from_utf8_lossy(&o.stdout).to_lowercase();
        if s.contains("wacom") || s.contains("tablet") {
            return Some("USB tablet".to_string());
        }
    }
    None
}

fn colord_present() -> bool {
    Command::new("colormgr").arg("get-devices").output().map(|o| o.status.success()).unwrap_or(false)
}

fn media_key() -> SettingKey {
    SettingKey::Custom("media.autorun".to_string())
}
fn pressure_key() -> SettingKey {
    SettingKey::Custom("tablet.pressure".to_string())
}

#[derive(PartialEq)]
pub struct Peripherals {
    pub store: SettingsStore,
}

impl Component for Peripherals {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let tablet: State<Option<String>> = use_state(|| None);
        let checked: State<bool> = use_state(|| false);
        let pressure: State<f64> = use_state(|| 50.0);
        let autorun: State<bool> = use_state(|| false);
        let colord: State<Option<bool>> = use_state(|| None);

        use_hook(move || {
            let mut tb = tablet;
            let mut ck = checked;
            let mut cd = colord;
            spawn(async move {
                let res = tokio::task::spawn_blocking(|| (tablet_present(), colord_present())).await.unwrap_or((None, false));
                tb.set(res.0);
                cd.set(Some(res.1));
                ck.set(true);
            });
        });

        let tablet_opt = tablet.read().clone();
        let done = *checked.read();
        let colord_state = *colord.read();
        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(MOUSE, "Color and devices", "Tablet, color profiles and removable media."));

        // Tablet: only when HW present
        if !done {
            base = base.child(tile().child(tile_head(None, "Graphics tablet", None::<String>)).child(tile_sub("Probing for tablets via libwacom…")));
        } else if let Some(name) = tablet_opt {
            base = base.child(
                tile()
                    .child(tile_head(None, "Graphics tablet", Some(status_chip("Connected", true, None))))
                    .child(setting_row(name, Some("Detected via libwacom / input devices".to_string()), false, {
                        let mut p = pressure;
                        let s = store;
                        pill_switch(*pressure.read() > 1.0, move |v| {
                            p.set(if v { 50.0 } else { 0.0 });
                            s.set(pressure_key(), if v { 50.0 } else { 0.0 });
                        })
                    }))
                    .child(field_label(format!("Pressure curve · {}%", *pressure.read() as i32)))
                    .child({
                        let mut p = pressure;
                        let s = store;
                        slider_row(None, *pressure.read(), move |v| {
                            p.set(v);
                            s.set(pressure_key(), v);
                        })
                    })
                    .child(tile_sub("Pressure persists locally. Calibration runs in the vendor tool.")),
            );
        } else {
            base = base.child(
                tile()
                    .child(tile_head(None, "Graphics tablet", Some(status_chip("No tablet", false, None))))
                    .child(empty_state(MOUSE, "No tablet detected", "Connect a Wacom-compatible tablet. This section appears only when hardware is present.")),
            );
        }

        // Color: honest colord stub
        base = base.child(
            tile()
                .child(tile_head(
                    None,
                    "Color profiles",
                    Some(status_chip(
                        match colord_state {
                            Some(true) => "colord",
                            Some(false) => "Unavailable",
                            None => "Checking",
                        },
                        colord_state == Some(true),
                        None,
                    )),
                ))
                .child(tile_sub(match colord_state {
                    Some(true) => "colord is installed. Profile assignment runs via colormgr; Finick stores the preference.",
                    _ => "Unavailable: colord/colormgr not found. No profile can be applied from here.",
                }))
                .child(setting_row(
                    "Night light follows display",
                    Some("Honest stub: no colord calibration is performed.".to_string()),
                    false,
                    rect().opacity(0.45).child(pill_switch(false, move |_| {})),
                )),
        );

        // Removable media
        {
            let on = *autorun.read();
            let media_lock = store.lock_label(&media_key());
            let managed = media_lock.is_some();
            base = base.child(
                tile()
                    .child(tile_head(
                        None,
                        "Removable media",
                        Some(
                            rect()
                                .horizontal()
                                .spacing(8.)
                                .cross_align(Alignment::Center)
                                .content(Content::Flex)
                                .maybe_child(media_lock.clone().map(lock_badge))
                                .child(status_chip(if on { "Autorun" } else { "Ask" }, on, None)),
                        ),
                    ))
                    .child(tile_sub("Autorun preference persists via the settings daemon."))
                    .child(setting_row(
                        "Open media automatically",
                        Some("Stored preference; the file manager applies it.".to_string()),
                        false,
                        {
                            let s = store;
                            let mut a = autorun;
                            rect().opacity(if managed { 0.45 } else { 1.0 }).child(pill_switch(on, move |v| {
                                if !managed {
                                    a.set(v);
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
