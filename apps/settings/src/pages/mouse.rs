use {freya::prelude::*, system::SystemBackend, ui::*};

#[derive(PartialEq)]
pub struct Mouse;

impl Component for Mouse {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let speed: State<f64> = use_state(|| 0.0);
        let accel: State<String> = use_state(|| "adaptive".to_string());
        let natural: State<bool> = use_state(|| false);
        let tap: State<bool> = use_state(|| true);
        let dwt: State<bool> = use_state(|| true);
        let left_handed: State<bool> = use_state(|| false);
        let mice: State<Vec<String>> = use_state(Vec::new);
        let keyboards: State<Vec<String>> = use_state(Vec::new);
        let loading: State<bool> = use_state(|| true);
        let op_error: State<Option<String>> = use_state(|| None);
        let applying: State<bool> = use_state(|| false);
        let speed_gen: State<std::sync::Arc<std::sync::atomic::AtomicU64>> =
            use_state(|| std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)));

        use_hook(move || {
            let mut sp = speed;
            let mut ac = accel;
            let mut nat = natural;
            let mut tp = tap;
            let mut dw = dwt;
            let mut lh = left_handed;
            let mut mc = mice;
            let mut kb = keyboards;
            let mut ld = loading;
            spawn(async move {
                let ps = tokio::task::spawn_blocking(system::load_pointer_settings)
                    .await
                    .unwrap_or_default();
                sp.set_if_modified(ps.speed);
                ac.set_if_modified(ps.accel_profile);
                nat.set_if_modified(ps.natural_scroll);
                tp.set_if_modified(ps.tap_to_click);
                dw.set_if_modified(ps.disable_while_typing);
                lh.set_if_modified(ps.left_handed);
                let devs = tokio::task::spawn_blocking(|| {
                    system::HyprlandBackend.get_input_devices()
                })
                .await
                .unwrap_or_default();
                mc.set(devs.mice.into_iter().map(|d| d.name).collect());
                kb.set(devs.keyboards.into_iter().map(|d| d.name).collect());
                ld.set(false);
            });
        });

        let err_opt = op_error.read().clone();
        let is_loading = *loading.read();
        let mice_list = mice.read().clone();
        let kb_list = keyboards.read().clone();
        let speed_val = *speed.read();
        let slider_val = ((speed_val + 1.0) * 50.0).clamp(0.0, 100.0);
        let accel_val = accel.read().clone();
        let no_devices = !is_loading && mice_list.is_empty() && kb_list.is_empty();

        let apply_bool = move |key: &'static str, v: bool, mut flag: State<bool>| {
            let mut err = op_error;
            let mut nat_s = natural;
            let mut tap_s = tap;
            let mut dwt_s = dwt;
            let mut lh_s = left_handed;
            flag.set(true);
            err.set(None);
            spawn(async move {
                let res =
                    tokio::task::spawn_blocking(move || system::apply_input_bool(key, v))
                        .await
                        .unwrap_or(Err("Task failed".to_string()));
                match res {
                    Ok(()) => {
                        match key {
                            "natural_scroll" => nat_s.set(v),
                            "tap_to_click" => tap_s.set(v),
                            "disable_while_typing" => dwt_s.set(v),
                            "left_handed" => lh_s.set(v),
                            _ => {}
                        }
                        flag.set(false);
                    }
                    Err(e) => {
                        flag.set(false);
                        err.set(Some(e));
                    }
                }
            });
        };

        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(
                GENERAL,
                "Mouse & Touchpad",
                "Pointer speed, acceleration and touchpad gestures",
            ))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
                    .maybe_child(err_opt.map(|e| {
                        tile()
                            .child(tile_head(None, "Could not apply change", None::<String>))
                            .child(tile_sub(e))
                            .child(rect().margin((8., 0., 0., 0.)).child(secondary_button(
                                "Dismiss",
                                {
                                    let mut er = op_error;
                                    move || er.set(None)
                                },
                            )))
                    }))
                    .child(
                        tile()
                            .child(tile_head(
                                None,
                                "Pointer",
                                if *applying.read() {
                                    Some(status_chip("Applying…", false, None))
                                } else {
                                    None
                                },
                            ))
                            .child(field_label(format!(
                                "Pointer speed · {:.2}",
                                speed_val
                            )))
                            .child({
                                let mut sp = speed;
                                let gen_arc = speed_gen.read().clone();
                                let mut flag = applying;
                                let mut err = op_error;
                                slider_row(None, slider_val, move |v| {
                                    let next = ((v / 50.0) - 1.0).clamp(-1.0, 1.0);
                                    sp.set(next);
                                    let ctr = gen_arc.clone();
                                    let seq =
                                        ctr.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                                    flag.set(true);
                                    spawn(async move {
                                        tokio::time::sleep(
                                            std::time::Duration::from_millis(150),
                                        )
                                        .await;
                                        if ctr.load(std::sync::atomic::Ordering::SeqCst)
                                            != seq
                                        {
                                            return;
                                        }
                                        let res = tokio::task::spawn_blocking(move || {
                                            system::apply_pointer_speed(next)
                                        })
                                        .await
                                        .unwrap_or(Err("Task failed".to_string()));
                                        flag.set(false);
                                        match res {
                                            Ok(()) => err.set(None),
                                            Err(e) => err.set(Some(e)),
                                        }
                                    });
                                })
                            })
                            .child(
                                rect()
                                    .margin((14., 0., 0., 0.))
                                    .child(field_label("Acceleration profile")),
                            )
                            .child({
                                let mut ac = accel;
                                let mut flag = applying;
                                let mut err = op_error;
                                let cur = accel_val.clone();
                                segmented_control(
                                    vec![("Adaptive", "adaptive".to_string()), ("Flat", "flat".to_string())],
                                    cur,
                                    move |v: String| {
                                        ac.set(v.clone());
                                        flag.set(true);
                                        err.set(None);
                                        spawn(async move {
                                            let res = tokio::task::spawn_blocking(move || {
                                                system::apply_pointer_accel(&v)
                                            })
                                            .await
                                            .unwrap_or(Err("Task failed".to_string()));
                                            flag.set(false);
                                            if let Err(e) = res {
                                                err.set(Some(e));
                                            }
                                        });
                                    },
                                )
                            })
                            .child(setting_row(
                                "Natural scroll",
                                Some("Content moves with your fingers".to_string()),
                                true,
                                {
                                    let cur = *natural.read();
                                    let flag = applying;
                                    pill_switch(cur, move |v| {
                                        apply_bool("natural_scroll", v, flag)
                                    })
                                },
                            ))
                            .child(setting_row(
                                "Left-handed pointers",
                                Some("Swap primary and secondary buttons".to_string()),
                                true,
                                {
                                    let cur = *left_handed.read();
                                    let flag = applying;
                                    pill_switch(cur, move |v| {
                                        apply_bool("left_handed", v, flag)
                                    })
                                },
                            )),
                    )
                    .child(
                        tile()
                            .child(tile_head(None, "Touchpad", None::<String>))
                            .child(setting_row(
                                "Tap to click",
                                Some("Tap the touchpad to click".to_string()),
                                false,
                                {
                                    let cur = *tap.read();
                                    let flag = applying;
                                    pill_switch(cur, move |v| {
                                        apply_bool("tap_to_click", v, flag)
                                    })
                                },
                            ))
                            .child(setting_row(
                                "Disable while typing",
                                Some("Ignore accidental touches".to_string()),
                                true,
                                {
                                    let cur = *dwt.read();
                                    let flag = applying;
                                    pill_switch(cur, move |v| {
                                        apply_bool("disable_while_typing", v, flag)
                                    })
                                },
                            )),
                    )
                    .child({
                        let mut base = tile().child(tile_head(
                            None,
                            "Devices",
                            (!is_loading).then(|| {
                                status_chip(
                                    format!(
                                        "{} pointers · {} keyboards",
                                        mice_list.len(),
                                        kb_list.len()
                                    ),
                                    false,
                                    None,
                                )
                            }),
                        ));
                        if is_loading {
                            base = base.child(tile_sub("Reading input devices…"));
                        } else if no_devices {
                            base = base.child(empty_state(
                                GENERAL,
                                "No input devices found",
                                "Connect a mouse or keyboard, or start Hyprland with libinput enabled.",
                            ));
                        } else {
                            for name in mice_list.iter().take(8) {
                                base = base.child(setting_row(
                                    name.clone(),
                                    Some("Pointer device".to_string()),
                                    false,
                                    status_chip("Connected", true, None),
                                ));
                            }
                            for name in kb_list.iter().take(8) {
                                base = base.child(setting_row(
                                    name.clone(),
                                    Some("Keyboard".to_string()),
                                    false,
                                    status_chip("Connected", false, None),
                                ));
                            }
                            base = base.child(tile_sub(
                                "Per-device sensitivity follows the pointer speed above via hyprctl input.",
                            ));
                        }
                        let _ = t;
                        base
                    }),
            )
    }
}
