use freya::prelude::*;
use ui::*;
use system::{HyprlandBackend, SystemBackend};

/// 2-column grid container helper matching .grid2 in ui_demo.html
pub fn grid2(children: impl IntoIterator<Item = impl IntoElement>) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(GAP)
        .children(children)
        .content(Content::Flex)
}

/// Appearance detail page matching ui_demo.html
pub fn appearance_detail_page(
    theme_state: State<AppTheme>,
    wallpaper_idx: State<usize>,
    scrollbar_pref: State<usize>,
    icon_size_pref: State<usize>,
) -> impl IntoElement {
    let t = use_app_theme();

    responsive_view(1000.0, move |compact| {
        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(APPEARANCE, "Appearance", "Theme, accent colour and desktop background"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
                    // Wide Tile: Theme & Accent
                    .child(
                        tile()
                            .child(tile_head(
                                None,
                                "Theme & accent",
                                Some({
                                    let mut th = theme_state.clone();
                                    segmented_control(
                                        vec![
                                            ("Light", ThemeMode::Light),
                                            ("Dark", ThemeMode::Dark),
                                            ("Auto", ThemeMode::Auto),
                                        ],
                                        t.mode,
                                        move |mode| {
                                            let new_t = th.read().with_mode(mode);
                                            th.set(new_t);
                                            set_theme(&new_t);
                                        },
                                    )
                                }),
                            ))
                            .child(field_label("Accent colour"))
                            .child({
                                let mut th = theme_state.clone();
                                swatch_picker(
                                    t.accent_name,
                                    true,
                                    move |acc| {
                                        let new_t = th.read().with_accent(acc);
                                        th.set(new_t);
                                        set_theme(&new_t);
                                    },
                                )
                            })
                    )
                    // 2-Column Row: Wallpaper & Interface
                    .child(
                        responsive_stack(compact, [
                            // Left: Wallpaper (8 previews)
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(None, "Wallpaper", None::<String>))
                                        .child({
                                            let mut wp = wallpaper_idx.clone();
                                            wallpaper_picker(
                                                8,
                                                *wallpaper_idx.read(),
                                                true,
                                                move |idx| wp.set(idx),
                                            )
                                        })
                                ),
                            // Right: Interface
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(None, "Interface", None::<String>))
                                        .child(field_label("Show scrollbars"))
                                        .child({
                                            let mut sp = scrollbar_pref.clone();
                                            segmented_control(
                                                vec![("Automatically", 0), ("When scrolling", 1), ("Always", 2)],
                                                *scrollbar_pref.read(),
                                                move |idx| sp.set(idx),
                                            )
                                        })
                                        .child(rect().margin((16., 0., 0., 0.)).child(field_label("Sidebar icon size")))
                                        .child({
                                            let mut ip = icon_size_pref.clone();
                                            segmented_control(
                                                vec![("Small", 0), ("Medium", 1), ("Large", 2)],
                                                *icon_size_pref.read(),
                                                move |idx| ip.set(idx),
                                            )
                                        })
                                ),
                        ])
                    ),
            )
    })
}

/// Wi-Fi detail page matching ui_demo.html
pub fn wifi_detail_page(
    wifi_power: State<bool>,
    ask_to_join: State<bool>,
    limit_tracking: State<bool>,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(WIFI, "Wi-Fi", "Manage saved networks and connection preferences"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Current network
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Current network",
                            Some({
                                let mut wp = wifi_power.clone();
                                pill_switch(*wifi_power.read(), move |v| wp.set(v))
                            }),
                        ))
                        .child(setting_row(
                            if *wifi_power.read() { "Homebase 5G" } else { "Not connected" },
                            Some(if *wifi_power.read() { "Auto-join enabled" } else { "Wi-Fi is off" }),
                            false,
                            label().font_size(12.).color(t.accent).text(if *wifi_power.read() { "Connected" } else { "" }),
                        ))
                        .child(setting_row("IP address", None::<String>, true, label().font_size(12.).color(t.text_dim).text("192.168.1.42")))
                        .child(setting_row("Security", None::<String>, true, label().font_size(12.).color(t.text_dim).text("WPA3 Personal")))
                )
                // 2-Column Row: Known networks & Preferences
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Known networks", None::<String>))
                                    .child(setting_row("Homebase 5G", None::<String>, false, status_chip("Connected", true, None)))
                                    .child(setting_row("Homebase 2.4G", None::<String>, true, pill_switch(true, |_| {})))
                                    .child(setting_row("The Coffee House", None::<String>, true, pill_switch(false, |_| {})))
                                    .child(setting_row("Studio Guest", None::<String>, true, pill_switch(true, |_| {})))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Preferences", None::<String>))
                                    .child(setting_row("Ask to join networks", None::<String>, false, {
                                        let mut aj = ask_to_join.clone();
                                        pill_switch(*ask_to_join.read(), move |v| aj.set(v))
                                    }))
                                    .child(setting_row("Limit IP address tracking", None::<String>, true, {
                                        let mut lt = limit_tracking.clone();
                                        pill_switch(*limit_tracking.read(), move |v| lt.set(v))
                                    }))
                            ),
                    ])
                ),
        )
}

/// Bluetooth detail page matching ui_demo.html
pub fn bluetooth_detail_page(
    bt_power: State<bool>,
    discoverable: State<bool>,
    kb_connected: State<bool>,
    hp_connected: State<bool>,
    tp_connected: State<bool>,
) -> impl IntoElement {

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(BLUETOOTH, "Bluetooth", "Connect and manage nearby devices"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Bluetooth & Discoverable
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Bluetooth",
                            Some({
                                let mut bp = bt_power.clone();
                                pill_switch(*bt_power.read(), move |v| bp.set(v))
                            }),
                        ))
                        .child(setting_row("Discoverable", None::<String>, false, {
                            let mut d = discoverable.clone();
                            pill_switch(*discoverable.read(), move |v| d.set(v))
                        }))
                )
                // 2-Column Row: My devices & Nearby
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "My devices", None::<String>))
                                    .child(setting_row(
                                        "Wireless Keyboard",
                                        Some("Battery 64%"),
                                        false,
                                        {
                                            let mut kb = kb_connected.clone();
                                            status_chip(
                                                if *kb_connected.read() { "Connected" } else { "Connect" },
                                                *kb_connected.read(),
                                                Some((move |_| kb.set(!*kb.read())).into()),
                                            )
                                        },
                                    ))
                                    .child(setting_row(
                                        "Headphones",
                                        Some(if *hp_connected.read() { "Connected" } else { "Not connected" }),
                                        true,
                                        {
                                            let mut hp = hp_connected.clone();
                                            status_chip(
                                                if *hp_connected.read() { "Connected" } else { "Connect" },
                                                *hp_connected.read(),
                                                Some((move |_| hp.set(!*hp.read())).into()),
                                            )
                                        },
                                    ))
                                    .child(setting_row(
                                        "Trackpad",
                                        Some("Battery 81%"),
                                        true,
                                        {
                                            let mut tp = tp_connected.clone();
                                            status_chip(
                                                if *tp_connected.read() { "Connected" } else { "Connect" },
                                                *tp_connected.read(),
                                                Some((move |_| tp.set(!*tp.read())).into()),
                                            )
                                        },
                                    ))

                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Nearby", None::<String>))
                                    .child(setting_row("Unknown Speaker", None::<String>, false, ghost_button("Connect", || {})))
                                    .child(setting_row("Flora's Watch", None::<String>, true, ghost_button("Connect", || {})))
                            ),
                    ])
                ),
        )
}

/// Display detail page matching ui_demo.html
pub fn display_detail_page(
    brightness: State<f64>,
    auto_brightness: State<bool>,
    true_tone: State<bool>,
    res_choice: State<ResolutionChoice>,
    night_shift: State<bool>,
    night_shift_mode: State<usize>,
    color_temp: State<f64>,
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(DISPLAY, "Display", "Brightness, resolution and colour"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Brightness
                .child(
                    tile()
                        .child(tile_head(None, "Brightness", None::<String>))
                        .child({
                            let mut br = brightness.clone();
                            slider_row(Some(SUN), *brightness.read(), move |v| br.set(v))
                        })
                        .child(setting_row("Auto-brightness", None::<String>, false, {
                            let mut ab = auto_brightness.clone();
                            pill_switch(*auto_brightness.read(), move |v| ab.set(v))
                        }))
                        .child(setting_row("True Tone", None::<String>, true, {
                            let mut tt = true_tone.clone();
                            pill_switch(*true_tone.read(), move |v| tt.set(v))
                        }))
                )
                // 2-Column: Resolution & Night Shift
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Resolution", None::<String>))
                                    .child({
                                        let mut rc = res_choice.clone();
                                        resolution_picker(*res_choice.read(), move |c| rc.set(c))
                                    })
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(
                                        None,
                                        "Night Shift",
                                        Some({
                                            let mut ns = night_shift.clone();
                                            pill_switch(*night_shift.read(), move |v| ns.set(v))
                                        }),
                                    ))
                                    .child({
                                        let mut nm = night_shift_mode.clone();
                                        segmented_control(
                                            vec![("Off", 0), ("Sunset to sunrise", 1), ("Custom", 2)],
                                            *night_shift_mode.read(),
                                            move |idx| nm.set(idx),
                                        )
                                    })
                                    .child(rect().margin((14., 0., 0., 0.)).child(field_label("Colour temperature")))
                                    .child({
                                        let mut ct = color_temp.clone();
                                        slider_row(None, *color_temp.read(), move |v| ct.set(v))
                                    })
                            ),
                    ])
                )
                // Wide: Arrangement
                .child(
                    tile()
                        .child(tile_head(None, "Arrangement", None::<String>))
                        .child(displays_mock(vec![
                            DisplayMockItem { name: "Studio Display".into(), height_px: 60. },
                            DisplayMockItem { name: "Built-in Display".into(), height_px: 44. },
                        ]))
                ),
        )
}

/// Sound detail page matching ui_demo.html
pub fn sound_detail_page(
    volume: State<f64>,
    is_mute: State<bool>,
    feedback_on_change: State<bool>,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(SOUND, "Sound", "Output, input and alert sounds"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Output
                .child(
                    tile()
                        .child(tile_head(None, "Output", Some(tile_sub("Studio Speakers"))))
                        .child({
                            let mut vl = volume.clone();
                            slider_row(Some(SOUND), *volume.read(), move |v| {
                                vl.set(v);
                                std::thread::spawn(move || {
                                    let _ = HyprlandBackend.set_volume(v as i32);
                                });
                            })
                        })
                        .child(select_row("Studio Speakers", None))
                        .child(setting_row("Mute", None::<String>, false, {
                            let mut im = is_mute.clone();
                            pill_switch(*is_mute.read(), move |v| {
                                im.set(v);
                                std::thread::spawn(move || {
                                    let _ = HyprlandBackend.toggle_mute();
                                });
                            })
                        }))
                )
                // 2-Column: Input & Sound effects
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Input", None::<String>))
                                    .child(multi_segment_bar(vec![(35., t.accent), (65., t.track)], false))
                                    .child(select_row("Studio Microphone", None))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Sound effects", None::<String>))
                                    .child(select_row("Alert sound · Tri-tone", None))
                                    .child(setting_row("Feedback on volume change", None::<String>, false, {
                                        let mut fc = feedback_on_change.clone();
                                        pill_switch(*feedback_on_change.read(), move |v| fc.set(v))
                                    }))
                            ),
                    ])
                ),
        )
}

/// Focus detail page matching ui_demo.html
pub fn focus_detail_page(
    focus_mode: State<&'static str>,
    work_sched: State<bool>,
    sleep_sched: State<bool>,
    share_devices: State<bool>,
) -> impl IntoElement {

    let focus_desc = match *focus_mode.read() {
        "work" => "Notifications are silenced except from your team.",
        "personal" => "Only calls and messages from favourites come through.",
        "sleep" => "Everything is silenced until morning.",
        _ => "Notifications are delivered as usual.",
    };

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(FOCUS, "Focus", "Silence notifications on your schedule"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Mode
                .child(
                    tile()
                        .child(tile_head(None, "Mode", None::<String>))
                        .child(
                            rect()
                                .horizontal()
                                .spacing(8.)
                                .child({
                                    let mut fm = focus_mode.clone();
                                    focus_pill("Off", *focus_mode.read() == "off", move || fm.set("off"))
                                })
                                .child({
                                    let mut fm = focus_mode.clone();
                                    focus_pill("Work", *focus_mode.read() == "work", move || fm.set("work"))
                                })
                                .child({
                                    let mut fm = focus_mode.clone();
                                    focus_pill("Personal", *focus_mode.read() == "personal", move || fm.set("personal"))
                                })
                                .child({
                                    let mut fm = focus_mode.clone();
                                    focus_pill("Sleep", *focus_mode.read() == "sleep", move || fm.set("sleep"))
                                })
                        )
                        .child(
                            rect()
                                .margin((12., 0., 0., 0.))
                                .child(label().font_size(12.).color(use_app_theme().text_dim).text(focus_desc))
                        )
                )
                // 2-Column: Schedule & Allowed
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Schedule", None::<String>))
                                    .child(setting_row("Work", Some("Weekdays · 9:00–17:00"), false, {
                                        let mut ws = work_sched.clone();
                                        pill_switch(*work_sched.read(), move |v| ws.set(v))
                                    }))
                                    .child(setting_row("Sleep", Some("Daily · 23:00–07:00"), false, {
                                        let mut ss = sleep_sched.clone();
                                        pill_switch(*sleep_sched.read(), move |v| ss.set(v))
                                    }))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Allowed", None::<String>))
                                    .child(setting_row("Share across devices", None::<String>, false, {
                                        let mut sd = share_devices.clone();
                                        pill_switch(*share_devices.read(), move |v| sd.set(v))
                                    }))
                                    .child(setting_row("Allow notifications from", None::<String>, true, status_chip("Favourites", false, None)))
                            ),
                    ])
                ),
        )
}

/// Notifications detail page matching ui_demo.html
pub fn notifications_detail_page(
    allow_notif: State<bool>,
    notif_style: State<usize>,
    notif_messages: State<bool>,
    notif_calendar: State<bool>,
    notif_mail: State<bool>,
    notif_photos: State<bool>,
    notif_weather: State<bool>,
    silence_sleep: State<bool>,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(NOTIFICATIONS, "Notifications", "Choose how apps notify you"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Allow notifications
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Allow notifications",
                            Some({
                                let mut an = allow_notif.clone();
                                pill_switch(*allow_notif.read(), move |v| an.set(v))
                            }),
                        ))
                        .child({
                            let mut ns = notif_style.clone();
                            segmented_control(
                                vec![("Banners", 0), ("Alerts", 1)],
                                *notif_style.read(),
                                move |idx| ns.set(idx),
                            )
                        })
                )
                // 2-Column: Apps & DND
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Apps", None::<String>))
                                    .child(setting_row("Messages", None::<String>, false, {
                                        let mut nm = notif_messages.clone();
                                        pill_switch(*notif_messages.read(), move |v| nm.set(v))
                                    }))
                                    .child(setting_row("Calendar", None::<String>, true, {
                                        let mut nc = notif_calendar.clone();
                                        pill_switch(*notif_calendar.read(), move |v| nc.set(v))
                                    }))
                                    .child(setting_row("Mail", None::<String>, true, {
                                        let mut nm = notif_mail.clone();
                                        pill_switch(*notif_mail.read(), move |v| nm.set(v))
                                    }))
                                    .child(setting_row("Photos", None::<String>, true, {
                                        let mut np = notif_photos.clone();
                                        pill_switch(*notif_photos.read(), move |v| np.set(v))
                                    }))
                                    .child(setting_row("Weather", None::<String>, true, {
                                        let mut nw = notif_weather.clone();
                                        pill_switch(*notif_weather.read(), move |v| nw.set(v))
                                    }))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Do Not Disturb", None::<String>))
                                    .child(setting_row("Silence during sleep", None::<String>, false, {
                                        let mut ss = silence_sleep.clone();
                                        pill_switch(*silence_sleep.read(), move |v| ss.set(v))
                                    }))
                                    .child(setting_row("Schedule", None::<String>, true, label().font_size(12.).color(t.text_dim).text("22:00–07:00")))
                            ),
                    ])
                ),
        )
}

/// General detail page matching ui_demo.html
pub fn general_detail_page(
    time_24h: State<bool>,
    auto_updates: State<bool>,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(GENERAL, "General", "Language, region and system-wide preferences"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Language & Region
                .child(
                    tile()
                        .child(tile_head(None, "Language & region", None::<String>))
                        .child(select_row("Language · English (Australia)", None))
                        .child(select_row("Region · Australia", None))
                )
                // 2-Column: Date & Time & Software Update
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Date & time", None::<String>))
                                    .child(setting_row("24-hour time", None::<String>, false, {
                                        let mut t24 = time_24h.clone();
                                        pill_switch(*time_24h.read(), move |v| t24.set(v))
                                    }))
                                    .child(setting_row("Time zone", None::<String>, true, label().font_size(12.).color(t.text_dim).text("Melbourne (GMT+10)")))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Software update", Some(status_chip("Up to date", false, None))))
                                    .child(setting_row("Automatic updates", None::<String>, false, {
                                        let mut au = auto_updates.clone();
                                        pill_switch(*auto_updates.read(), move |v| au.set(v))
                                    }))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Check for updates", || {})))
                            ),
                    ])
                ),
        )
}

/// Storage detail page matching ui_demo.html
pub fn storage_detail_page(
    empty_trash_auto: State<bool>,
    save_cloud: State<bool>,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(STORAGE, "Storage", "See what's using space and free some up"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: 1 TB drive
                .child(
                    tile()
                        .child(tile_head(None, "1 TB drive", Some(tile_sub("612 GB used · 412 GB available"))))
                        .child(multi_segment_bar(
                            vec![
                                (38., t.accent),
                                (21., t.bg_active),
                                (14., t.text_dim),
                                (27., t.track),
                            ],
                            true,
                        ))
                        .child(storage_legend(vec![
                            StorageLegendItem { label: "Applications", value: "231 GB".into(), color: t.accent },
                            StorageLegendItem { label: "Photos", value: "128 GB".into(), color: t.bg_active },
                            StorageLegendItem { label: "Documents", value: "84 GB".into(), color: t.text_dim },
                            StorageLegendItem { label: "System", value: "169 GB".into(), color: t.track },
                        ]))
                )
                // 2-Column: Recommendations & Categories
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Recommendations", None::<String>))
                                    .child(setting_row("Empty Trash automatically", None::<String>, false, {
                                        let mut et = empty_trash_auto.clone();
                                        pill_switch(*empty_trash_auto.read(), move |v| et.set(v))
                                    }))
                                    .child(setting_row("Save new files to Cloud", None::<String>, true, {
                                        let mut sc = save_cloud.clone();
                                        pill_switch(*save_cloud.read(), move |v| sc.set(v))
                                    }))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Review large files", || {})))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Categories", None::<String>))
                                    .child(setting_row("Applications", None::<String>, false, label().font_size(12.).color(t.text_dim).text("231 GB")))
                                    .child(setting_row("Photos", None::<String>, true, label().font_size(12.).color(t.text_dim).text("128 GB")))
                                    .child(setting_row("Documents", None::<String>, true, label().font_size(12.).color(t.text_dim).text("84 GB")))
                                    .child(setting_row("System", None::<String>, true, label().font_size(12.).color(t.text_dim).text("169 GB")))
                            ),
                    ])
                ),
        )
}

/// Battery detail page matching ui_demo.html
pub fn battery_detail_page(
    battery_pct: State<u8>,
    battery_mode: State<&'static str>,
    opt_charging: State<bool>,
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(BATTERY, "Battery", "Usage, health and power mode"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Power mode
                .child(
                    tile()
                        .child(tile_head(None, "Power mode", Some(tile_sub("78% · Charging"))))
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(16.)
                                .child(battery_ring(*battery_pct.read(), true))
                                .child({
                                    let mut bm = battery_mode.clone();
                                    battery_modes(
                                        vec![
                                            ("Low Power", "low"),
                                            ("Balanced", "balanced"),
                                            ("High Performance", "high"),
                                        ],
                                        *battery_mode.read(),
                                        true,
                                        move |m| bm.set(m),
                                    )
                                })
                        )
                )
                // 2-Column: Past 7 days & Battery health
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Past 7 days", None::<String>))
                                    .child(mini_bar_chart([52., 70., 38., 85., 60., 44., 78.], ["M", "T", "W", "T", "F", "S", "S"]))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Battery health", Some(status_chip("Normal", false, None))))
                                    .child(setting_row("Maximum capacity", None::<String>, false, label().font_size(12.).color(use_app_theme().text_dim).text("91%")))
                                    .child(setting_row("Optimised charging", None::<String>, true, {
                                        let mut oc = opt_charging.clone();
                                        pill_switch(*opt_charging.read(), move |v| oc.set(v))
                                    }))
                            ),
                    ])
                ),
        )
}

/// Accessibility detail page matching ui_demo.html
pub fn accessibility_detail_page(
    text_size: State<f64>,
    reduce_motion: State<bool>,
    increase_contrast: State<bool>,
    reduce_transparency: State<bool>,
    screen_reader: State<bool>,
) -> impl IntoElement {

    let t = use_app_theme();
    let preview_font_size = 13.0 + (*text_size.read() as f32 / 100.0) * 11.0;

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(ACCESSIBILITY, "Accessibility", "Make your device easier to see, hear and use"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Text size
                .child(
                    tile()
                        .child(tile_head(None, "Text size", None::<String>))
                        .child({
                            let mut ts = text_size.clone();
                            slider_row(None, *text_size.read(), move |v| ts.set(v))
                        })
                        .child(
                            rect()
                                .margin((14., 0., 0., 0.))
                                .child(
                                    label()
                                        .font_size(preview_font_size)
                                        .color(t.text_dim)
                                        .text("The quick brown fox jumps over the lazy dog."),
                                )
                        )
                )
                // 2-Column: Display & Hearing & speech
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Display", None::<String>))
                                    .child(setting_row("Reduce motion", None::<String>, false, {
                                        let mut rm = reduce_motion.clone();
                                        pill_switch(*reduce_motion.read(), move |v| rm.set(v))
                                    }))
                                    .child(setting_row("Increase contrast", None::<String>, true, {
                                        let mut ic = increase_contrast.clone();
                                        pill_switch(*increase_contrast.read(), move |v| ic.set(v))
                                    }))
                                    .child(setting_row("Reduce transparency", None::<String>, true, {
                                        let mut rt = reduce_transparency.clone();
                                        pill_switch(*reduce_transparency.read(), move |v| rt.set(v))
                                    }))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Hearing & speech", None::<String>))
                                    .child(setting_row("Screen reader", None::<String>, false, {
                                        let mut sr = screen_reader.clone();
                                        pill_switch(*screen_reader.read(), move |v| sr.set(v))
                                    }))
                                    .child(select_row("Spoken content voice · Aria", None))
                            ),
                    ])
                ),
        )
}

/// About detail page matching ui_demo.html
pub fn about_detail_page() -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(ABOUT, "About", "Device information and legal"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Device info
                .child(
                    tile()
                        .child(setting_row("Name", None::<String>, false, label().font_size(12.).color(t.text_dim).text("Flora's Studio")))
                        .child(setting_row("Version", None::<String>, true, label().font_size(12.).color(t.text_dim).text("14.6.1 (Finick OS)")))
                        .child(setting_row("Processor", None::<String>, true, label().font_size(12.).color(t.text_dim).text("8-core")))
                        .child(setting_row("Memory", None::<String>, true, label().font_size(12.).color(t.text_dim).text("32 GB")))
                        .child(setting_row("Storage", None::<String>, true, label().font_size(12.).color(t.text_dim).text("1 TB")))
                )
                // 2-Column: Software & Support
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Software", Some(status_chip("Up to date", false, None))))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Check for updates", || {})))
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Support", None::<String>))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Get help", || {})))
                            ),
                    ])
                ),
        )
}
