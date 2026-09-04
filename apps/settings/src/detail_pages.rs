use freya::prelude::*;
use ipsea::settings::SettingKey;
use system::{DisplayInfo, SystemBackend};
use ui::*;
use ui::displays_mock_reorderable;
use crate::state::*;

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
    store: SettingsStore,
) -> impl IntoElement {
    let t = use_app_theme();
    let theme_lock = store.lock_label(&SettingKey::ThemeMode);
    let is_theme_locked = theme_lock.is_some();
    let accent_lock = store.lock_label(&SettingKey::AccentColor);
    let is_accent_locked = accent_lock.is_some();
    let wp_lock = store.lock_label(&SettingKey::Wallpaper);
    let is_wp_locked = wp_lock.is_some();
    let scroll_lock = store.lock_label(&SettingKey::Scrollbars);
    let is_scroll_locked = scroll_lock.is_some();
    let icon_lock = store.lock_label(&SettingKey::IconSize);
    let is_icon_locked = icon_lock.is_some();

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
                                Some(
                                    rect()
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .spacing(8.)
                                        .maybe_child(theme_lock.as_ref().map(|l| lock_badge(l)))
                                        .child(
                                            rect()
                                                .opacity(if is_theme_locked { 0.45 } else { 1.0 })
                                                .child({
                                                    let mut th = theme_state;
                                                    let store = store;
                                                    segmented_control(
                                                        vec![
                                                            ("Light", ThemeMode::Light),
                                                            ("Dark", ThemeMode::Dark),
                                                            ("Auto", ThemeMode::Auto),
                                                        ],
                                                        t.mode,
                                                        move |mode| {
                                                            if !is_theme_locked {
                                                                let new_t = th.read().with_mode(mode);
                                                                th.set(new_t);
                                                                set_theme(&new_t);
                                                                let mode_str = match mode {
                                                                    ThemeMode::Light => "light",
                                                                    ThemeMode::Dark => "dark",
                                                                    ThemeMode::Auto => "auto",
                                                                };
                                                                store.set(SettingKey::ThemeMode, mode_str);
                                                            }
                                                        },
                                                    )
                                                }),
                                        ),
                                ),
                            ))
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .child(field_label("Accent colour"))
                                    .maybe_child(accent_lock.as_ref().map(|l| lock_badge(l))),
                            )
                            .child(
                                rect()
                                    .opacity(if is_accent_locked { 0.45 } else { 1.0 })
                                    .child({
                                        let mut th = theme_state;
                                        let store = store;
                                        swatch_picker(
                                            t.accent_name,
                                            true,
                                            move |acc| {
                                                if !is_accent_locked {
                                                    let new_t = th.read().with_accent(acc);
                                                    th.set(new_t);
                                                    set_theme(&new_t);
                                                    store.set(SettingKey::AccentColor, acc.name.to_string());
                                                }
                                            },
                                        )
                                    }),
                            ),
                    )
                    // 2-Column Row: Wallpaper & Interface
                    .child(
                        responsive_stack(compact, [
                            // Left: Wallpaper (8 previews)
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(
                                            None,
                                            "Wallpaper",
                                            wp_lock.as_ref().map(|l| lock_badge(l)),
                                        ))
                                        .child(
                                            rect()
                                                .opacity(if is_wp_locked { 0.45 } else { 1.0 })
                                                .child({
                                                    let store = store;
                                                    wallpaper_picker(
                                                        8,
                                                        *store.wallpaper_idx.read(),
                                                        true,
                                                        move |idx| {
                                                            if !is_wp_locked {
                                                                store.set(SettingKey::Wallpaper, idx as i64);
                                                            }
                                                        },
                                                    )
                                                }),
                                        ),
                                ),
                            // Right: Interface
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(None, "Interface", None::<String>))
                                        .child(
                                            rect()
                                                .horizontal()
                                                .cross_align(Alignment::Center)
                                                .spacing(8.)
                                                .child(field_label("Show scrollbars"))
                                                .maybe_child(scroll_lock.as_ref().map(|l| lock_badge(l))),
                                        )
                                        .child(
                                            rect()
                                                .opacity(if is_scroll_locked { 0.45 } else { 1.0 })
                                                .child({
                                                    let store = store;
                                                    segmented_control(
                                                        vec![("Automatically", 0), ("When scrolling", 1), ("Always", 2)],
                                                        *store.scrollbar_pref.read(),
                                                        move |idx| {
                                                            if !is_scroll_locked {
                                                                store.set(SettingKey::Scrollbars, idx as i64);
                                                            }
                                                        },
                                                    )
                                                }),
                                        )
                                        .child(
                                            rect()
                                                .margin((16., 0., 0., 0.))
                                                .horizontal()
                                                .cross_align(Alignment::Center)
                                                .spacing(8.)
                                                .child(field_label("Sidebar icon size"))
                                                .maybe_child(icon_lock.as_ref().map(|l| lock_badge(l))),
                                        )
                                        .child(
                                            rect()
                                                .opacity(if is_icon_locked { 0.45 } else { 1.0 })
                                                .child({
                                                    let store = store;
                                                    segmented_control(
                                                        vec![("Small", 0), ("Medium", 1), ("Large", 2)],
                                                        *store.icon_size_pref.read(),
                                                        move |idx| {
                                                            if !is_icon_locked {
                                                                store.set(SettingKey::IconSize, idx as i64);
                                                            }
                                                        },
                                                    )
                                                }),
                                        ),
                                ),
                        ]),
                    ),
            )
    })
}

/// Wi-Fi detail page matching ui_demo.html
pub fn wifi_detail_page(store: SettingsStore) -> impl IntoElement {
    let t = use_app_theme();
    let is_wifi_locked = store.is_locked(&SettingKey::WifiEnabled);
    let wifi_lock = store.lock_label(&SettingKey::WifiEnabled);
    let wifi_on = *store.wifi_power.read();

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
                            Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .maybe_child(wifi_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(
                                        rect()
                                            .opacity(if is_wifi_locked { 0.45 } else { 1.0 })
                                            .child({
                                                let store = store;
                                                pill_switch(wifi_on, move |v| {
                                                    if !is_wifi_locked {
                                                        store.set(SettingKey::WifiEnabled, v);
                                                    }
                                                })
                                            }),
                                    ),
                            ),
                        ))
                        .child(setting_row(
                            if wifi_on { "Homebase 5G" } else { "Not connected" },
                            Some(if wifi_on { "Auto-join enabled" } else { "Wi-Fi is off" }),
                            false,
                            label().font_size(12.).color(t.accent).text(if wifi_on { "Connected" } else { "" }),
                        ))
                        .child(setting_row("IP address", None::<String>, true, label().font_size(12.).color(t.text_dim).text("192.168.1.42")))
                        .child(setting_row("Security", None::<String>, true, label().font_size(12.).color(t.text_dim).text("WPA3 Personal"))),
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
                                    .child(setting_row("Studio Guest", None::<String>, true, pill_switch(true, |_| {}))),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Preferences", None::<String>))
                                    .child(setting_row_locked(
                                        "Ask to join networks",
                                        None::<String>,
                                        false,
                                        store.lock_label(&SettingKey::WifiNetwork),
                                        {
                                            let store = store;
                                            let is_locked = store.is_locked(&SettingKey::WifiNetwork);
                                            let val = *store.ask_to_join.read();
                                            pill_switch(val, move |v| {
                                                if !is_locked {
                                                    store.set(SettingKey::WifiNetwork, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(setting_row(
                                        "Limit IP address tracking",
                                        None::<String>,
                                        true,
                                        {
                                            let mut lt = store.limit_tracking;
                                            pill_switch(*store.limit_tracking.read(), move |v| lt.set(v))
                                        },
                                    )),
                            ),
                    ]),
                ),
        )
}

/// Bluetooth detail page matching ui_demo.html
pub fn bluetooth_detail_page(
    store: SettingsStore,
    kb_connected: State<bool>,
    hp_connected: State<bool>,
    tp_connected: State<bool>,
) -> impl IntoElement {
    let bt_lock = store.lock_label(&SettingKey::BluetoothEnabled);
    let is_bt_locked = bt_lock.is_some();
    let bt_on = *store.bt_power.read();
    let disc_on = *store.bt_discoverable.read();

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
                            Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .maybe_child(bt_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(
                                        rect()
                                            .opacity(if is_bt_locked { 0.45 } else { 1.0 })
                                            .child({
                                                let store = store;
                                                pill_switch(bt_on, move |v| {
                                                    if !is_bt_locked {
                                                        store.set(SettingKey::BluetoothEnabled, v);
                                                    }
                                                })
                                            }),
                                    ),
                            ),
                        ))
                        .child(setting_row("Discoverable", None::<String>, false, {
                            let mut d = store.bt_discoverable;
                            pill_switch(disc_on, move |v| d.set(v))
                        })),
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
                                            let mut kb = kb_connected;
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
                                            let mut hp = hp_connected;
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
                                            let mut tp = tp_connected;
                                            status_chip(
                                                if *tp_connected.read() { "Connected" } else { "Connect" },
                                                *tp_connected.read(),
                                                Some((move |_| tp.set(!*tp.read())).into()),
                                            )
                                        },
                                    )),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Nearby", None::<String>))
                                    .child(setting_row("Unknown Speaker", None::<String>, false, ghost_button("Connect", || {})))
                                    .child(setting_row("Flora's Watch", None::<String>, true, ghost_button("Connect", || {}))),
                            ),
                    ]),
                ),
        )
}

/// Display detail page matching ui_demo.html
pub fn display_detail_page(store: SettingsStore) -> impl IntoElement {
    let br_lock = store.lock_label(&SettingKey::DisplayBrightness);
    let is_br_locked = br_lock.is_some();
    let auto_br_lock = store.lock_label(&SettingKey::DisplayAutoBrightness);
    let is_auto_br_locked = auto_br_lock.is_some();
    let res_lock = store.lock_label(&SettingKey::DisplayResolution);
    let is_res_locked = res_lock.is_some();
    let ns_lock = store.lock_label(&SettingKey::DisplayNightShift);
    let is_ns_locked = ns_lock.is_some();
    let ct_lock = store.lock_label(&SettingKey::DisplayColorTemp);
    let is_ct_locked = ct_lock.is_some();
    // Arrangement state — top-level hooks for stable order
    let mut displays: State<Option<Vec<DisplayInfo>>> = use_state(|| None);
    let mut loaded = use_state(|| false);
    let mut order_status: State<Option<Result<(), String>>> = use_state(|| None);
    if !*loaded.read() {
        loaded.set(true);
        let mut ds = displays;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let d = system::HyprlandBackend.get_displays();
            let _ = tx.send(d);
        });
        freya::prelude::spawn(async move {
            if let Some(d) = rx.recv().await {
                ds.set(Some(d));
            }
        });
    }
    let t = use_app_theme();
    let is_arrangement_locked = is_res_locked;

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
                        .child(tile_head(
                            None,
                            "Brightness",
                            br_lock.as_ref().map(|l| lock_badge(l)),
                        ))
                        .child(
                            rect()
                                .opacity(if is_br_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    slider_row(Some(SUN), *store.brightness.read(), move |v| {
                                        if !is_br_locked {
                                            store.set(SettingKey::DisplayBrightness, v);
                                        }
                                    })
                                }),
                        )
                        .child(setting_row_locked(
                            "Auto-brightness",
                            None::<String>,
                            false,
                            auto_br_lock,
                            {
                                let store = store;
                                pill_switch(*store.auto_brightness.read(), move |v| {
                                    if !is_auto_br_locked {
                                        store.set(SettingKey::DisplayAutoBrightness, v);
                                    }
                                })
                            },
                        ))
                        .child(setting_row("True Tone", None::<String>, true, {
                            let mut tt = store.true_tone;
                            pill_switch(*store.true_tone.read(), move |v| tt.set(v))
                        })),
                )
                // 2-Column: Resolution & Night Shift
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(
                                        None,
                                        "Resolution",
                                        res_lock.as_ref().map(|l| lock_badge(l)),
                                    ))
                                    .child(
                                        rect()
                                            .opacity(if is_res_locked { 0.45 } else { 1.0 })
                                            .child({
                                                let store = store;
                                                resolution_picker(*store.res_choice.read(), move |c| {
                                                    if !is_res_locked {
                                                        let choice_str = match c {
                                                            ResolutionChoice::MoreSpace => "MoreSpace",
                                                            ResolutionChoice::LargerText => "LargerText",
                                                            ResolutionChoice::Default => "Default",
                                                        };
                                                        store.set(SettingKey::DisplayResolution, choice_str);
                                                    }
                                                })
                                            }),
                                    ),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(
                                        None,
                                        "Night Shift",
                                        Some(
                                            rect()
                                                .horizontal()
                                                .cross_align(Alignment::Center)
                                                .spacing(8.)
                                                .maybe_child(ns_lock.as_ref().map(|l| lock_badge(l)))
                                                .child(
                                                    rect()
                                                        .opacity(if is_ns_locked { 0.45 } else { 1.0 })
                                                        .child({
                                                            let store = store;
                                                            pill_switch(*store.night_shift.read(), move |v| {
                                                                if !is_ns_locked {
                                                                    store.set(SettingKey::DisplayNightShift, v);
                                                                }
                                                            })
                                                        }),
                                                ),
                                        ),
                                    ))
                                    .child({
                                        let mut nm = store.night_shift_mode;
                                        segmented_control(
                                            vec![("Off", 0), ("Sunset to sunrise", 1), ("Custom", 2)],
                                            *store.night_shift_mode.read(),
                                            move |idx| nm.set(idx),
                                        )
                                    })
                                    .child(
                                        rect()
                                            .margin((14., 0., 0., 0.))
                                            .horizontal()
                                            .cross_align(Alignment::Center)
                                            .spacing(8.)
                                            .child(field_label("Colour temperature"))
                                            .maybe_child(ct_lock.as_ref().map(|l| lock_badge(l))),
                                    )
                                    .child(
                                        rect()
                                            .opacity(if is_ct_locked { 0.45 } else { 1.0 })
                                            .child({
                                                let store = store;
                                                slider_row(None, *store.color_temp.read(), move |v| {
                                                    if !is_ct_locked {
                                                        store.set(SettingKey::DisplayColorTemp, v);
                                                    }
                                                })
                                            }),
                                    ),
                            ),
                    ]),
                )
                // Wide: Arrangement — interactive reorderable
                .child({
                    let displays_len = displays.read().as_ref().map(|v| v.len()).unwrap_or(0);
                    let status_chip_text: Option<Element> = match &*order_status.read() {
                        Some(Ok(())) => Some(status_chip("Order saved", false, None).into_element()),
                        Some(Err(e)) => {
                            let msg = if e.len() > 48 { format!("{}…", &e[..48]) } else { e.clone() };
                            Some(status_chip(format!("Error: {msg}"), true, None).into_element())
                        }
                        None => None,
                    };
                    let header_right: Option<Element> = {
                        let lock = res_lock.clone().map(|l| lock_badge(l).into_element());
                        match (lock, status_chip_text) {
                            (Some(l), Some(s)) => Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .child(l)
                                    .child(s)
                                    .into_element(),
                            ),
                            (Some(l), None) => Some(l),
                            (None, Some(s)) => Some(s),
                            (None, None) => None,
                        }
                    };
                    let arrangement_tile = tile().child(tile_head(None, "Arrangement", header_right))
                        .child({
                            if displays.read().is_none() {
                                tile_sub("Scanning displays…").into_element()
                            } else if displays_len == 0 {
                                rect()
                                    .width(Size::fill())
                                    .center()
                                    .padding((18., 12.))
                                    .child(
                                        rect()
                                            .vertical()
                                            .cross_align(Alignment::Center)
                                            .spacing(6.)
                                            .child(icon(DISPLAY, 20., t.text_dim))
                                            .child(label().font_size(12.).color(t.text_dim).text("No displays detected")),
                                    )
                                    .into_element()
                            } else {
                                let items: Vec<DisplayMockItem> = displays
                                    .read()
                                    .as_ref()
                                    .map(|v| {
                                        v.iter()
                                            .enumerate()
                                            .map(|(i, d)| DisplayMockItem {
                                                name: d.name.clone(),
                                                height_px: 60. - (i as f32 * 8.).min(28.),
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                let on_swap = {
                                    let mut displays_state = displays;
                                    let mut status_state = order_status;
                                    let locked = is_arrangement_locked;
                                    EventHandler::new(move |(from, to): (usize, usize)| {
                                        if locked {
                                            return;
                                        }
                                        let len = displays_state.read().as_ref().map(|v| v.len()).unwrap_or(0);
                                        if from >= len || to >= len {
                                            return;
                                        }
                                        let mut new_order = displays_state.read().as_ref().unwrap().clone();
                                        new_order.swap(from, to);
                                        let prev = displays_state.read().as_ref().unwrap().clone();
                                        displays_state.set(Some(new_order.clone()));
                                        status_state.set(None);
                                        let names: Vec<String> = new_order.iter().map(|d| d.name.clone()).collect();
                                        freya::prelude::spawn(async move {
                                            let res = tokio::task::spawn_blocking(move || {
                                                system::HyprlandBackend.set_display_order(&names)
                                            })
                                            .await;
                                            match res {
                                                Ok(Ok(())) => status_state.set(Some(Ok(()))),
                                                Ok(Err(e)) => {
                                                    displays_state.set(Some(prev));
                                                    status_state.set(Some(Err(e)));
                                                }
                                                Err(e) => {
                                                    displays_state.set(Some(prev));
                                                    status_state.set(Some(Err(format!("join error: {e}"))));
                                                }
                                            }
                                        });
                                    })
                                };
                                rect()
                                    .width(Size::fill())
                                    .opacity(if is_arrangement_locked { 0.45 } else { 1.0 })
                                    .child(displays_mock_reorderable(items, on_swap))
                                    .into_element()
                            }
                        });
                    arrangement_tile.into_element()
                }),
        )
}

/// Sound detail page matching ui_demo.html
pub fn sound_detail_page(store: SettingsStore) -> impl IntoElement {
    let t = use_app_theme();
    let vol_lock = store.lock_label(&SettingKey::AudioVolume);
    let is_vol_locked = vol_lock.is_some();
    let mute_lock = store.lock_label(&SettingKey::AudioMuted);
    let is_mute_locked = mute_lock.is_some();

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
                        .child(tile_head(
                            None,
                            "Output",
                            Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .maybe_child(vol_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(tile_sub("Studio Speakers")),
                            ),
                        ))
                        .child(
                            rect()
                                .opacity(if is_vol_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    slider_row(Some(SOUND), *store.volume.read(), move |v| {
                                        if !is_vol_locked {
                                            store.set(SettingKey::AudioVolume, v as i64);
                                        }
                                    })
                                }),
                        )
                        .child(select_row("Studio Speakers", None))
                        .child(setting_row_locked(
                            "Mute",
                            None::<String>,
                            false,
                            mute_lock,
                            {
                                let store = store;
                                pill_switch(*store.is_mute.read(), move |v| {
                                    if !is_mute_locked {
                                        store.set(SettingKey::AudioMuted, v);
                                    }
                                })
                            },
                        )),
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
                                    .child(select_row("Studio Microphone", None)),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Sound effects", None::<String>))
                                    .child(select_row("Alert sound · Tri-tone", None))
                                    .child(setting_row("Feedback on volume change", None::<String>, false, {
                                        let mut fc = store.feedback_on_change;
                                        pill_switch(*store.feedback_on_change.read(), move |v| fc.set(v))
                                    })),
                            ),
                    ]),
                ),
        )
}

/// Focus detail page matching ui_demo.html
pub fn focus_detail_page(
    store: SettingsStore,
    work_sched: State<bool>,
    sleep_sched: State<bool>,
    share_devices: State<bool>,
) -> impl IntoElement {
    let _t = use_app_theme();
    let dnd_lock = store.lock_label(&SettingKey::DoNotDisturb);
    let is_dnd_locked = dnd_lock.is_some();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(FOCUS, "Focus", "Silence notifications and reduce distractions"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                // Wide: Focus mode selector
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Focus mode",
                            dnd_lock.as_ref().map(|l| lock_badge(l)),
                        ))
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .spacing(8.)
                                .content(Content::Flex)
                                .opacity(if is_dnd_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    focus_pill("Off", *store.focus_mode.read() == "off", move || {
                                        if !is_dnd_locked {
                                            store.set(SettingKey::DoNotDisturb, "off");
                                        }
                                    })
                                })
                                .child({
                                    let store = store;
                                    focus_pill("Work", *store.focus_mode.read() == "work", move || {
                                        if !is_dnd_locked {
                                            store.set(SettingKey::DoNotDisturb, "work");
                                        }
                                    })
                                })
                                .child({
                                    let store = store;
                                    focus_pill("Personal", *store.focus_mode.read() == "personal", move || {
                                        if !is_dnd_locked {
                                            store.set(SettingKey::DoNotDisturb, "personal");
                                        }
                                    })
                                })
                                .child({
                                    let store = store;
                                    focus_pill("Sleep", *store.focus_mode.read() == "sleep", move || {
                                        if !is_dnd_locked {
                                            store.set(SettingKey::DoNotDisturb, "sleep");
                                        }
                                    })
                                }),
                        ),
                )
                // 2-Column: Schedule & Allowed
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Schedule", None::<String>))
                                    .child(setting_row("Work", Some("Weekdays · 09:00–17:00"), false, {
                                        let mut ws = work_sched;
                                        pill_switch(*work_sched.read(), move |v| ws.set(v))
                                    }))
                                    .child(setting_row("Sleep", Some("Daily · 23:00–07:00"), true, {
                                        let mut ss = sleep_sched;
                                        pill_switch(*sleep_sched.read(), move |v| ss.set(v))
                                    })),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Allowed", None::<String>))
                                    .child(setting_row("Share across devices", None::<String>, false, {
                                        let mut sd = share_devices;
                                        pill_switch(*share_devices.read(), move |v| sd.set(v))
                                    }))
                                    .child(setting_row("Allow notifications from", None::<String>, true, status_chip("Favourites", false, None))),
                            ),
                    ]),
                ),
        )
}

/// Notifications detail page matching ui_demo.html
pub fn notifications_detail_page(store: SettingsStore) -> impl IntoElement {
    let notif_lock = store.lock_label(&SettingKey::NotificationBanners);
    let is_notif_locked = notif_lock.is_some();
    let silence_lock = store.lock_label(&SettingKey::NotificationSounds);
    let is_silence_locked = silence_lock.is_some();

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
                            Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .maybe_child(notif_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(
                                        rect()
                                            .opacity(if is_notif_locked { 0.45 } else { 1.0 })
                                            .child({
                                                let store = store;
                                                pill_switch(*store.allow_notif.read(), move |v| {
                                                    if !is_notif_locked {
                                                        store.set(SettingKey::NotificationBanners, v);
                                                    }
                                                })
                                            }),
                                    ),
                            ),
                        ))
                        .child({
                            let mut ns = store.notif_style;
                            segmented_control(
                                vec![("Banners", 0), ("Alerts", 1)],
                                *store.notif_style.read(),
                                move |idx| ns.set(idx),
                            )
                        }),
                )
                // 2-Column: App alerts & Do Not Disturb
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "App alerts", None::<String>))
                                    .child(setting_row("Messages", Some("Badges, Sounds, Banners"), false, {
                                        let mut m = store.notif_messages;
                                        pill_switch(*store.notif_messages.read(), move |v| m.set(v))
                                    }))
                                    .child(setting_row("Calendar", Some("Badges, Sounds, Alerts"), true, {
                                        let mut c = store.notif_calendar;
                                        pill_switch(*store.notif_calendar.read(), move |v| c.set(v))
                                    }))
                                    .child(setting_row("Mail", Some("Badges only"), true, {
                                        let mut ml = store.notif_mail;
                                        pill_switch(*store.notif_mail.read(), move |v| ml.set(v))
                                    }))
                                    .child(setting_row("Photos", Some("Memories and shared albums"), true, {
                                        let mut p = store.notif_photos;
                                        pill_switch(*store.notif_photos.read(), move |v| p.set(v))
                                    })),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Do Not Disturb", None::<String>))
                                    .child(setting_row_locked(
                                        "Silence during sleep",
                                        None::<String>,
                                        false,
                                        silence_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.silence_sleep.read(), move |v| {
                                                if !is_silence_locked {
                                                    store.set(SettingKey::NotificationSounds, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(setting_row("Schedule", None::<String>, true, label().font_size(12.).color(use_app_theme().text_dim).text("22:00–07:00"))),
                            ),
                    ]),
                ),
        )
}

/// General detail page matching ui_demo.html
pub fn general_detail_page(store: SettingsStore) -> impl IntoElement {
    let t = use_app_theme();
    let t24_lock = store.lock_label(&SettingKey::TimeFormat24h);
    let is_t24_locked = t24_lock.is_some();
    let updates_lock = store.lock_label(&SettingKey::SystemAutoUpdates);
    let is_updates_locked = updates_lock.is_some();

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
                        .child(select_row("Region · Australia", None)),
                )
                // 2-Column: Date & Time & Software Update
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Date & time", None::<String>))
                                    .child(setting_row_locked(
                                        "24-hour time",
                                        None::<String>,
                                        false,
                                        t24_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.time_24h.read(), move |v| {
                                                if !is_t24_locked {
                                                    store.set(SettingKey::TimeFormat24h, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(setting_row("Time zone", None::<String>, true, label().font_size(12.).color(t.text_dim).text("Melbourne (GMT+10)"))),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Software update", Some(status_chip("Up to date", false, None))))
                                    .child(setting_row_locked(
                                        "Automatic updates",
                                        None::<String>,
                                        false,
                                        updates_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.auto_updates.read(), move |v| {
                                                if !is_updates_locked {
                                                    store.set(SettingKey::SystemAutoUpdates, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Check for updates", || {}))),
                            ),
                    ]),
                ),
        )
}

/// Storage detail page matching ui_demo.html
pub fn storage_detail_page(store: SettingsStore) -> impl IntoElement {
    let t = use_app_theme();
    let trash_lock = store.lock_label(&SettingKey::StorageEmptyTrashAuto);
    let is_trash_locked = trash_lock.is_some();

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
                        ])),
                )
                // 2-Column: Recommendations & Categories
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Recommendations", None::<String>))
                                    .child(setting_row_locked(
                                        "Empty Trash automatically",
                                        None::<String>,
                                        false,
                                        trash_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.empty_trash_auto.read(), move |v| {
                                                if !is_trash_locked {
                                                    store.set(SettingKey::StorageEmptyTrashAuto, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(setting_row("Save new files to Cloud", None::<String>, true, {
                                        let mut sc = store.save_cloud;
                                        pill_switch(*store.save_cloud.read(), move |v| sc.set(v))
                                    }))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Review large files", || {}))),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Categories", None::<String>))
                                    .child(setting_row("Applications", None::<String>, false, label().font_size(12.).color(t.text_dim).text("231 GB")))
                                    .child(setting_row("Photos", None::<String>, true, label().font_size(12.).color(t.text_dim).text("128 GB")))
                                    .child(setting_row("Documents", None::<String>, true, label().font_size(12.).color(t.text_dim).text("84 GB")))
                                    .child(setting_row("System", None::<String>, true, label().font_size(12.).color(t.text_dim).text("169 GB"))),
                            ),
                    ]),
                ),
        )
}

/// Battery detail page matching ui_demo.html
pub fn battery_detail_page(store: SettingsStore) -> impl IntoElement {
    let pwr_lock = store.lock_label(&SettingKey::PowerProfile);
    let is_pwr_locked = pwr_lock.is_some();

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
                        .child(tile_head(
                            None,
                            "Power mode",
                            Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .maybe_child(pwr_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(tile_sub("78% · Charging")),
                            ),
                        ))
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(16.)
                                .child(battery_ring(*store.battery_pct.read(), true))
                                .child(
                                    rect()
                                        .opacity(if is_pwr_locked { 0.45 } else { 1.0 })
                                        .child({
                                            let store = store;
                                            battery_modes(
                                                vec![
                                                    ("Low Power", "low"),
                                                    ("Balanced", "balanced"),
                                                    ("High Performance", "high"),
                                                ],
                                                *store.battery_mode.read(),
                                                true,
                                                move |m| {
                                                    if !is_pwr_locked {
                                                        store.set(SettingKey::PowerProfile, m);
                                                    }
                                                },
                                            )
                                        }),
                                ),
                        ),
                )
                // 2-Column: Past 7 days & Battery health
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Past 7 days", None::<String>))
                                    .child(mini_bar_chart([52., 70., 38., 85., 60., 44., 78.], ["M", "T", "W", "T", "F", "S", "S"])),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Battery health", Some(status_chip("Normal", false, None))))
                                    .child(setting_row("Maximum capacity", None::<String>, false, label().font_size(12.).color(use_app_theme().text_dim).text("91%")))
                                    .child(setting_row("Optimised charging", None::<String>, true, {
                                        let mut oc = store.opt_charging;
                                        pill_switch(*store.opt_charging.read(), move |v| oc.set(v))
                                    })),
                            ),
                    ]),
                ),
        )
}

/// Accessibility detail page matching ui_demo.html
pub fn accessibility_detail_page(store: SettingsStore) -> impl IntoElement {
    let t = use_app_theme();
    let preview_font_size = 13.0 + (*store.text_size.read() as f32 / 100.0) * 11.0;

    let txt_lock = store.lock_label(&SettingKey::AccessibilityTextSize);
    let is_txt_locked = txt_lock.is_some();
    let motion_lock = store.lock_label(&SettingKey::AccessibilityReduceMotion);
    let is_motion_locked = motion_lock.is_some();
    let contrast_lock = store.lock_label(&SettingKey::AccessibilityIncreaseContrast);
    let is_contrast_locked = contrast_lock.is_some();
    let transp_lock = store.lock_label(&SettingKey::AccessibilityReduceTransparency);
    let is_transp_locked = transp_lock.is_some();
    let reader_lock = store.lock_label(&SettingKey::AccessibilityScreenReader);
    let is_reader_locked = reader_lock.is_some();

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
                        .child(tile_head(
                            None,
                            "Text size",
                            txt_lock.as_ref().map(|l| lock_badge(l)),
                        ))
                        .child(
                            rect()
                                .opacity(if is_txt_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    slider_row(None, *store.text_size.read(), move |v| {
                                        if !is_txt_locked {
                                            store.set(SettingKey::AccessibilityTextSize, v);
                                        }
                                    })
                                }),
                        )
                        .child(
                            rect()
                                .margin((14., 0., 0., 0.))
                                .child(
                                    label()
                                        .font_size(preview_font_size)
                                        .color(t.text_dim)
                                        .text("The quick brown fox jumps over the lazy dog."),
                                ),
                        ),
                )
                // 2-Column: Display & Hearing & speech
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Display", None::<String>))
                                    .child(setting_row_locked(
                                        "Reduce motion",
                                        None::<String>,
                                        false,
                                        motion_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.reduce_motion.read(), move |v| {
                                                if !is_motion_locked {
                                                    store.set(SettingKey::AccessibilityReduceMotion, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(setting_row_locked(
                                        "Increase contrast",
                                        None::<String>,
                                        true,
                                        contrast_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.increase_contrast.read(), move |v| {
                                                if !is_contrast_locked {
                                                    store.set(SettingKey::AccessibilityIncreaseContrast, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(setting_row_locked(
                                        "Reduce transparency",
                                        None::<String>,
                                        true,
                                        transp_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.reduce_transparency.read(), move |v| {
                                                if !is_transp_locked {
                                                    store.set(SettingKey::AccessibilityReduceTransparency, v);
                                                }
                                            })
                                        },
                                    )),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Hearing & speech", None::<String>))
                                    .child(setting_row_locked(
                                        "Screen reader",
                                        None::<String>,
                                        false,
                                        reader_lock,
                                        {
                                            let store = store;
                                            pill_switch(*store.screen_reader.read(), move |v| {
                                                if !is_reader_locked {
                                                    store.set(SettingKey::AccessibilityScreenReader, v);
                                                }
                                            })
                                        },
                                    ))
                                    .child(select_row("Spoken content voice · Aria", None)),
                            ),
                    ]),
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
                        .child(setting_row("Storage", None::<String>, true, label().font_size(12.).color(t.text_dim).text("1 TB"))),
                )
                // 2-Column: Software & Support
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Software", Some(status_chip("Up to date", false, None))))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Check for updates", || {}))),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Support", None::<String>))
                                    .child(rect().margin((10., 0., 0., 0.)).child(ghost_button("Get help", || {}))),
                            ),
                    ]),
                ),
        )
}
