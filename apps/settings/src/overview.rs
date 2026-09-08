use freya::prelude::*;
use ipsea::settings::SettingKey;
use system::{DisplayInfo, HyprlandBackend, SystemBackend, WiredInfo};
use ui::*;
use crate::pages::about::AboutInfo;
use crate::pages::network::{fetch_network_info, NetworkInfo};
use system::BluetoothDevice;
use crate::state::SettingsStore;

#[derive(PartialEq)]
pub struct OverviewPage {
    pub current_route: State<crate::Route>,
    pub theme_state: State<AppTheme>,
    pub store: SettingsStore,
    pub wired: Option<WiredInfo>,
    pub about: AboutInfo,
    pub displays: Vec<DisplayInfo>,
}

impl Component for OverviewPage {
    fn render(&self) -> impl IntoElement {
        let current_route = self.current_route;
        let theme_state = self.theme_state;
        let store = self.store;
        let wired = self.wired.clone();
        let about = self.about.clone();
        let displays = self.displays.clone();
        let t = use_app_theme();
        let net_state: State<Option<NetworkInfo>> = use_state(|| None);
        use_hook(move || {
            let mut ns = net_state;
            spawn(async move {
                let info = tokio::task::spawn_blocking(fetch_network_info).await.unwrap_or_default();
                ns.set(Some(info));
            });
        });
        let net = net_state.read().clone().unwrap_or_default();
        let storage = HyprlandBackend.get_storage_info();
        let bt_devices_state: State<Vec<BluetoothDevice>> = use_state(Vec::new);
        use_hook(move || {
            let mut ds = bt_devices_state;
            spawn(async move {
                let devs = tokio::task::spawn_blocking(|| HyprlandBackend.get_paired_bluetooth_devices())
                    .await
                    .unwrap_or_default();
                ds.set(devs);
            });
        });
        let bt_devices = bt_devices_state.read().clone();

        responsive_view(1000.0, move |compact| {
            let focus_desc = match *store.focus_mode.read() {
                "work" => "Notifications are silenced except from your team.",
                "personal" => "Only calls and messages from favourites come through.",
                "sleep" => "Everything is silenced until morning.",
                _ => "Notifications are delivered as usual.",
            };

        let appearance_card = {
            let mut route = current_route;
            let theme_lock = store.lock_label(&SettingKey::ThemeMode);
            let is_theme_locked = theme_lock.is_some();
            let accent_lock = store.lock_label(&SettingKey::AccentColor);
            let is_accent_locked = accent_lock.is_some();
            let wp_lock = store.lock_label(&SettingKey::Wallpaper);
            let is_wp_locked = wp_lock.is_some();

            tile()
                .child(tile_head(
                    Some(APPEARANCE),
                    "Appearance",
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
                                        let mut theme = theme_state;
                                        let store = store;
                                        segmented_control(
                                            vec![("Light", ThemeMode::Light), ("Dark", ThemeMode::Dark), ("Auto", ThemeMode::Auto)],
                                            t.mode,
                                            move |mode| {
                                                if !is_theme_locked {
                                                    let new_t = theme.read().with_mode(mode);
                                                    theme.set(new_t);
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
                            )
                            .child(
                                rect()
                                    .cursor(CursorIcon::Pointer)
                                    .padding((4., 6.))
                                    .on_press(move |_| route.set(crate::Route::Appearance))
                                    .child(label().font_size(12.).color(t.text_dim).text("→")),
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
                            let mut theme = theme_state;
                            let store = store;
                            swatch_picker(t.accent_name, false, move |acc| {
                                if !is_accent_locked {
                                    let new_t = theme.read().with_accent(acc);
                                    theme.set(new_t);
                                    set_theme(&new_t);
                                    store.set(SettingKey::AccentColor, acc.name.to_string());
                                }
                            })
                        }),
                )
                .child(
                    rect()
                        .margin((14., 0., 0., 0.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(field_label("Desktop background"))
                        .maybe_child(wp_lock.as_ref().map(|l| lock_badge(l))),
                )
                .child(
                    rect()
                        .opacity(if is_wp_locked { 0.45 } else { 1.0 })
                        .child({
                            let store = store;
                            wallpaper_picker(4, *store.wallpaper_idx.read(), false, move |idx| {
                                if !is_wp_locked {
                                    store.set(SettingKey::Wallpaper, idx as i64);
                                }
                            })
                        }),
                )
        };

        let wifi_card = {
            let mut route = current_route;
            let wifi_lock = store.lock_label(&SettingKey::WifiEnabled);
            let is_wifi_locked = wifi_lock.is_some();
            let wifi_on = *store.wifi_power.read();

            tile()
                .child(tile_head(
                    Some(WIFI),
                    "Wi-Fi",
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
                    if !wifi_on {
                        "Not connected".to_string()
                    } else if let Some(ssid) = net.active_ssid.clone() {
                        ssid
                    } else {
                        "Not connected".to_string()
                    },
                    Some(if !wifi_on {
                        "Wi-Fi is off".to_string()
                    } else if net.active_ssid.is_some() {
                        format!("{} · {}", net.primary_interface, net.primary_ip)
                    } else {
                        "Not associated".to_string()
                    }),
                    false,
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 6.))
                        .on_press(move |_| route.set(crate::Route::Wifi))
                        .child(label().font_size(12.).color(t.text_dim).text("→")),
                ))
        };

        let wired_card = {
            tile()
                .child(tile_head(None, "Wired", None::<String>))
                .child(match wired.clone() {
                    Some(info) => {
                        let name = if info.profile.trim().is_empty() {
                            info.interface.clone()
                        } else {
                            info.profile.clone()
                        };
                        let detail = if info.ip_address.trim().is_empty() {
                            info.interface.clone()
                        } else {
                            format!("{} · {}", info.interface, info.ip_address)
                        };
                        setting_row(
                            name,
                            Some(detail),
                            false,
                            label().font_size(12.).color(t.accent).text("Connected"),
                        )
                        .into_element()
                    }
                    None => setting_row(
                        "No wired connection".to_string(),
                        None::<String>,
                        false,
                        label().font_size(12.).color(t.text_dim).text(""),
                    )
                    .into_element(),
                })
        };

        let bluetooth_card = {
            let bt_lock = store.lock_label(&SettingKey::BluetoothEnabled);
            let is_bt_locked = bt_lock.is_some();
            let bt_on = *store.bt_power.read();

            let mut base = tile()
                .child(tile_head(
                    Some(BLUETOOTH),
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
                                                std::thread::spawn(move || {
                                                    HyprlandBackend.set_bluetooth_status(v);
                                                });
                                            }
                                        })
                                    }),
                            ),
                    ),
                ));
            if bt_devices.is_empty() {
                base = base.child(tile_sub(if bt_on { "No paired devices" } else { "Bluetooth is off" }));
            } else {
                for dev in bt_devices.iter().take(3) {
                    base = base.child(setting_row(
                        dev.name.clone(),
                        Some(dev.mac.clone()),
                        false,
                        status_chip(if dev.connected { "Connected" } else { "Paired" }, dev.connected, None),
                    ));
                }
            }
            base
        };

        let focus_card = {
            let mut route = current_route;
            let dnd_lock = store.lock_label(&SettingKey::DoNotDisturb);
            let is_dnd_locked = dnd_lock.is_some();

            tile()
                .child(tile_head(
                    Some(FOCUS),
                    "Focus",
                    Some(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .maybe_child(dnd_lock.as_ref().map(|l| lock_badge(l)))
                            .child(
                                rect()
                                    .cursor(CursorIcon::Pointer)
                                    .padding((4., 6.))
                                    .on_press(move |_| route.set(crate::Route::Focus))
                                    .child(label().font_size(12.).color(t.text_dim).text("→")),
                            ),
                    ),
                ))
                .child(
                    rect()
                        .vertical()
                        .spacing(8.)
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
                )
                .child(rect().margin((12., 0., 0., 0.)).child(label().font_size(12.).color(t.text_dim).text(focus_desc)))
        };

        let display_card = {
            let mut route = current_route;
            let br_lock = store.lock_label(&SettingKey::DisplayBrightness);
            let is_br_locked = br_lock.is_some();
            let displays_len = displays.len();

            tile()
                .child(tile_head(
                    Some(DISPLAY),
                    "Display",
                    Some(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .maybe_child(br_lock.as_ref().map(|l| lock_badge(l)))
                            .child(
                                rect()
                                    .cursor(CursorIcon::Pointer)
                                    .padding((4., 6.))
                                    .on_press(move |_| route.set(crate::Route::Display))
                                    .child(label().font_size(12.).color(t.text_dim).text("→")),
                            ),
                    ),
                ))
                .child(
                    rect()
                        .opacity(if is_br_locked { 0.45 } else { 1.0 })
                        .child({
                            let store = store;
                            slider_row(Some(SUN), *store.brightness.read(), move |val| {
                                if !is_br_locked {
                                    store.set(SettingKey::DisplayBrightness, val);
                                }
                            })
                        }),
                )
                .child(setting_row(
                    "Arrangement",
                    Some(match displays_len {
                        0 => "No displays detected".to_string(),
                        1 => format!("1 display ({})", displays[0].name),
                        n => format!("{n} displays connected"),
                    }),
                    false,
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 8.))
                        .corner_radius(6.)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border))
                        .on_press(move |_| route.set(crate::Route::Display))
                        .child(label().font_size(11.).color(t.accent).text("Rearrange →")),
                ))
        };

        let sound_card = {
            let mut route = current_route;
            let vol_lock = store.lock_label(&SettingKey::AudioVolume);
            let is_vol_locked = vol_lock.is_some();

            tile()
                .child(tile_head(
                    Some(SOUND),
                    "Sound",
                    Some(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .maybe_child(vol_lock.as_ref().map(|l| lock_badge(l)))
                            .child(
                                rect()
                                    .cursor(CursorIcon::Pointer)
                                    .padding((4., 6.))
                                    .on_press(move |_| route.set(crate::Route::Sound))
                                    .child(label().font_size(12.).color(t.text_dim).text("→")),
                            ),
                    ),
                ))
                .child(
                    rect()
                        .opacity(if is_vol_locked { 0.45 } else { 1.0 })
                        .child({
                            let store = store;
                            slider_row(Some(SOUND), *store.volume.read(), move |val| {
                                if !is_vol_locked {
                                    store.set(SettingKey::AudioVolume, val as i64);
                                }
                            })
                        }),
                )
        };

        let storage_card = {
            let mut route = current_route;
            let used_pct = storage.use_percent as f32;
            let free_pct = (100.0_f32 - used_pct).max(0.0_f32);
            let storage_label = format!("{} of {} used", storage.used, storage.total);

            tile()
                .child(tile_head(
                    Some(STORAGE),
                    "Storage",
                    Some(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .child(tile_sub(storage_label))
                            .child(
                                rect()
                                    .cursor(CursorIcon::Pointer)
                                    .padding((4., 6.))
                                    .on_press(move |_| route.set(crate::Route::Storage))
                                    .child(label().font_size(12.).color(t.text_dim).text("→")),
                            ),
                    ),
                ))
                .child(multi_segment_bar(vec![(used_pct, t.accent), (free_pct, t.track)], false))
                .child(storage_legend(vec![
                    StorageLegendItem { label: "Used", value: storage.used.clone().into(), color: t.accent },
                    StorageLegendItem { label: "Available", value: storage.available.clone().into(), color: t.track },
                ]))
        };

        let notifications_card = {
            let mut route = current_route;
            let mut messages = store.notif_messages;
            let mut calendar = store.notif_calendar;
            let mut mail = store.notif_mail;

            tile()
                .child(tile_head(
                    Some(NOTIFICATIONS),
                    "Notifications",
                    Some(
                        rect()
                            .cursor(CursorIcon::Pointer)
                            .padding((4., 6.))
                            .on_press(move |_| route.set(crate::Route::Notifications))
                            .child(label().font_size(12.).color(t.text_dim).text("→")),
                    ),
                ))
                .child(setting_row("Messages", None::<String>, false, pill_switch(*store.notif_messages.read(), move |v| messages.set(v))))
                .child(setting_row("Calendar", None::<String>, true, pill_switch(*store.notif_calendar.read(), move |v| calendar.set(v))))
                .child(setting_row("Mail", None::<String>, true, pill_switch(*store.notif_mail.read(), move |v| mail.set(v))))
        };

        let battery_card = {
            let mut route = current_route;
            let pwr_lock = store.lock_label(&SettingKey::PowerProfile);
            let is_pwr_locked = pwr_lock.is_some();

            tile()
                .child(tile_head(
                    Some(BATTERY),
                    "Battery",
                    Some(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .maybe_child(pwr_lock.as_ref().map(|l| lock_badge(l)))
                            .child(
                                rect()
                                    .cursor(CursorIcon::Pointer)
                                    .padding((4., 6.))
                                    .on_press(move |_| route.set(crate::Route::Battery))
                                    .child(label().font_size(12.).color(t.text_dim).text("→")),
                            ),
                    ),
                ))
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(12.)
                        .child(battery_ring(*store.battery_pct.read(), false))
                        .child(
                            rect()
                                .opacity(if is_pwr_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    battery_modes(
                                        vec![("Low Power", "low"), ("Balanced", "balanced"), ("High Performance", "high")],
                                        *store.battery_mode.read(),
                                        false,
                                        move |m| {
                                            if !is_pwr_locked {
                                                store.set(SettingKey::PowerProfile, m);
                                            }
                                        },
                                    )
                                }),
                        ),
                )
        };

        let about_card = {
            let mut route = current_route;

            tile()
                .child(tile_head(
                    Some(ABOUT),
                    "About",
                    Some(
                        rect()
                            .cursor(CursorIcon::Pointer)
                            .padding((4., 6.))
                            .on_press(move |_| route.set(crate::Route::About))
                            .child(label().font_size(12.).color(t.text_dim).text("→")),
                    ),
                ))
                .child(setting_row("Version", Some(about.os_version.clone()), false, label().font_size(12.).color(t.text_dim).text(about.kernel.clone())))
                .child(setting_row("Software", None::<String>, true, status_chip("Up to date", false, None)))
        };

        if compact {
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(appearance_card)
                .child(wifi_card)
                .child(wired_card)
                .child(bluetooth_card)
                .child(focus_card)
                .child(display_card)
                .child(sound_card)
                .child(storage_card)
                .child(notifications_card)
                .child(battery_card)
                .child(about_card)
        } else {
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .spacing(GAP)
                        .content(Content::Flex)
                        .child(rect().width(Size::flex(1.)).child(appearance_card))
                        .child(
                            rect()
                                .width(Size::flex(1.))
                                .vertical()
                                .spacing(GAP)
                                .child(wifi_card)
                                .child(wired_card),
                        ),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .spacing(GAP)
                        .content(Content::Flex)
                        .child(rect().width(Size::flex(1.)).child(bluetooth_card))
                        .child(rect().width(Size::flex(1.)).child(focus_card)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .spacing(GAP)
                        .content(Content::Flex)
                        .child(rect().width(Size::flex(1.)).child(display_card))
                        .child(rect().width(Size::flex(1.)).child(sound_card)),
                )
                .child(storage_card)
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .spacing(GAP)
                        .content(Content::Flex)
                        .child(rect().width(Size::flex(2.)).child(notifications_card))
                        .child(rect().width(Size::flex(1.)).child(battery_card))
                        .child(rect().width(Size::flex(1.)).child(about_card)),
                )
        }
    })
    }
}

/// Helper function returning OverviewPage Component
pub fn overview_page(
    current_route: State<crate::Route>,
    theme_state: State<AppTheme>,
    store: SettingsStore,
    wired: Option<WiredInfo>,
    about: AboutInfo,
    displays: Vec<DisplayInfo>,
) -> OverviewPage {
    OverviewPage {
        current_route,
        theme_state,
        store,
        wired,
        about,
        displays,
    }
}