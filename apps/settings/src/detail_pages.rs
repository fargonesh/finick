use freya::prelude::*;
use ipsea::settings::SettingKey;
use system::{DisplayInfo, SystemBackend};
use ui::*;
use crate::pages::network::{fetch_network_info, NetworkInfo};
use crate::state::*;

pub fn grid2(children: impl IntoIterator<Item = impl IntoElement>) -> impl IntoElement {
    let items: Vec<Element> = children.into_iter().map(|c| c.into_element()).collect();
    responsive_view(720.0, move |compact| {
        if compact {
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .children(items.clone())
                .content(Content::Flex)
        } else {
            rect()
                .width(Size::fill())
                .horizontal()
                .spacing(GAP)
                .children(items.clone())
                .content(Content::Flex)
        }
    })
}

#[derive(PartialEq)]
pub struct AppearanceDetailPage {
    pub theme_state: State<AppTheme>,
    pub store: SettingsStore,
}

impl Component for AppearanceDetailPage {
    fn render(&self) -> impl IntoElement {
        let theme_state = self.theme_state;
        let store = self.store;
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
                                // Left: Wallpaper
                                rect()
                                    .width(Size::flex(1.))
                                    .child({
                                        let cur_wp = store.wallpaper.read().clone();
                                        let is_custom_image = {
                                            let clean = cur_wp.strip_prefix("file://").unwrap_or(&cur_wp);
                                            !clean.is_empty() && std::path::Path::new(clean).is_file()
                                        };
                                        let image_label = if is_custom_image {
                                            let clean = cur_wp.strip_prefix("file://").unwrap_or(&cur_wp);
                                            std::path::Path::new(clean)
                                                .file_name()
                                                .map(|f| f.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "Custom image".to_string())
                                        } else {
                                            "Custom image".to_string()
                                        };
                                        let image_subtext = if is_custom_image {
                                            let clean = cur_wp.strip_prefix("file://").unwrap_or(&cur_wp);
                                            Some(clean.to_string())
                                        } else {
                                            Some("Select an image to display across all monitors".to_string())
                                        };

                                        tile()
                                            .child(tile_head(
                                                Some(PICTURE),
                                                "Wallpaper",
                                                wp_lock.as_ref().map(|l| lock_badge(l)),
                                            ))
                                            .child(
                                                rect()
                                                    .opacity(if is_wp_locked { 0.45 } else { 1.0 })
                                                    .vertical()
                                                    .spacing(10.)
                                                    .child(field_label("Colour defaults"))
                                                    .child({
                                                        let store = store;
                                                        wallpaper_color_picker(&cur_wp, move |hex| {
                                                            if !is_wp_locked {
                                                                store.set(SettingKey::Wallpaper, hex);
                                                            }
                                                        })
                                                    })
                                                    .child(
                                                        rect().margin((8., 0., 0., 0.)).child(field_label("Custom background"))
                                                    )
                                                     .child(setting_row(
                                                        image_label,
                                                        image_subtext,
                                                        false,
                                                        rect()
                                                            .horizontal()
                                                            .spacing(8.)
                                                            .content(Content::Flex)
                                                            .child(secondary_button("Choose file…", move || {
                                                                if is_wp_locked {
                                                                    return;
                                                                }
                                                                let store = store;
                                                                freya::prelude::spawn(async move {
                                                                    if let Some(file) = rfd::AsyncFileDialog::new()
                                                                        .add_filter(
                                                                            "Images",
                                                                            &["png", "jpg", "jpeg", "webp", "svg", "bmp"],
                                                                        )
                                                                        .pick_file()
                                                                        .await
                                                                    {
                                                                        let path_str = file.path().to_string_lossy().to_string();
                                                                        store.set(SettingKey::Wallpaper, path_str);
                                                                    }
                                                                });
                                                            }))
                                                            .maybe(is_custom_image, |el| {
                                                                let store = store;
                                                                el.child(secondary_button("Reset", move || {
                                                                    if !is_wp_locked {
                                                                        store.set(SettingKey::Wallpaper, "#1e1e2e".to_string());
                                                                    }
                                                                }))
                                                            }),
                                                    ))
                                                    .child(
                                                        rect()
                                                            .margin((14., 0., 0., 0.))
                                                            .vertical()
                                                            .spacing(8.)
                                                            .child(field_label("Folder slideshow"))
                                                            .child(setting_row(
                                                                if store.wallpaper_folder.read().is_empty() {
                                                                    "No folder selected".to_string()
                                                                } else {
                                                                    store.wallpaper_folder.read().clone()
                                                                },
                                                                Some("Pick a folder to browse images as wallpapers".to_string()),
                                                                false,
                                                                rect()
                                                                    .horizontal()
                                                                    .spacing(8.)
                                                                    .content(Content::Flex)
                                                                    .child(secondary_button("Browse folder…", {
                                                                        let store = store;
                                                                        move || {
                                                                            if is_wp_locked {
                                                                                return;
                                                                            }
                                                                            let store = store;
                                                                            freya::prelude::spawn(async move {
                                                                                if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                                                                                    let path = folder.path().to_string_lossy().to_string();
                                                                                    let mut f = store.wallpaper_folder;
                                                                                    f.set(path.clone());
                                                                                    store.set(SettingKey::Custom("appearance.wallpaper_folder".to_string()), path);
                                                                                }
                                                                            });
                                                                        }
                                                                    }))
                                                                    .maybe(!store.wallpaper_folder.read().is_empty(), |el| {
                                                                        let store = store;
                                                                        el.child(ghost_button("Clear", move || {
                                                                            let mut f = store.wallpaper_folder;
                                                                            f.set(String::new());
                                                                            store.set(
                                                                                SettingKey::Custom("appearance.wallpaper_folder".to_string()),
                                                                                String::new(),
                                                                            );
                                                                        }))
                                                                    }),
                                                            ))
                                                            .child({
                                                                let folder = store.wallpaper_folder.read().clone();
                                                                if folder.is_empty() {
                                                                    rect().into_element()
                                                                } else {
                                                                    let images: Vec<String> = std::fs::read_dir(&folder)
                                                                        .map(|rd| {
                                                                            rd.filter_map(|e| e.ok())
                                                                                .filter_map(|e| {
                                                                                    let p = e.path();
                                                                                    let ext = p.extension()?.to_string_lossy().to_lowercase();
                                                                                    if ["png", "jpg", "jpeg", "webp", "bmp"].contains(&ext.as_str()) {
                                                                                        Some(p.to_string_lossy().to_string())
                                                                                    } else {
                                                                                        None
                                                                                    }
                                                                                })
                                                                                .take(12)
                                                                                .collect()
                                                                        })
                                                                        .unwrap_or_default();
                                                                    if images.is_empty() {
                                                                        rect()
                                                                            .margin((6., 0., 0., 0.))
                                                                            .child(label().font_size(11.).color(use_app_theme().text_dim).text("No images found in folder"))
                                                                            .into_element()
                                                                    } else {
                                                                        rect()
                                                                            .width(Size::fill())
                                                                            .vertical()
                                                                            .spacing(8.)
                                                                            .margin((6., 0., 0., 0.))
                                                                            .child(
                                                                                rect()
                                                                                    .width(Size::fill())
                                                                                    .horizontal()
                                                                                    .spacing(6.)
                                                                                    .content(Content::Flex)
                                                                                    .children(images.iter().take(6).cloned().map({
                                                                                        let store = store;
                                                                                        let cur = cur_wp.clone();
                                                                                        move |path| {
                                                                                            let is_sel = cur == path || cur == format!("file://{path}");
                                                                                            let p = path.clone();
                                                                                            let store = store;
                                                                                            rect()
                                                                                                .width(Size::flex(1.))
                                                                                                .height(Size::px(52.))
                                                                                                .corner_radius(8.)
                                                                                                .background(use_app_theme().panel_raised)
                                                                                                .border(Border::new().width(if is_sel { 2. } else { 1. }).fill(if is_sel { use_app_theme().accent } else { use_app_theme().border }))
                                                                                                .cursor(CursorIcon::Pointer)
                                                                                                .center()
                                                                                                .on_press(move |_| {
                                                                                                    store.set(SettingKey::Wallpaper, p.clone());
                                                                                                })
                                                                                                .child(label().font_size(9.).color(use_app_theme().text_dim).text(
                                                                                                    std::path::Path::new(&path)
                                                                                                        .file_name()
                                                                                                        .map(|n| n.to_string_lossy().chars().take(10).collect::<String>())
                                                                                                        .unwrap_or_else(|| "img".to_string()),
                                                                                                ))
                                                                                        }
                                                                                    })),
                                                                            )
                                                                            .into_element()
                                                                    }
                                                                }
                                                            })
                                                            .child(
                                                                rect()
                                                                    .margin((10., 0., 0., 0.))
                                                                    .horizontal()
                                                                    .cross_align(Alignment::Center)
                                                                    .spacing(8.)
                                                                    .child(field_label("Slideshow interval"))
                                                                    .child({
                                                                        let store = store;
                                                                        segmented_control(
                                                                            vec![("Off", 0), ("30s", 1), ("5m", 2), ("15m", 3), ("1h", 4)],
                                                                            *store.wallpaper_interval.read(),
                                                                            move |v| {
                                                                                let mut s = store.wallpaper_interval;
                                                                                s.set(v);
                                                                                store.set(
                                                                                    SettingKey::Custom("appearance.wallpaper_interval".to_string()),
                                                                                    v as i64,
                                                                                );
                                                                            },
                                                                        )
                                                                    }),
                                                            )
                                                            .child(tile_sub("When interval is set, Finick will rotate wallpapers from the chosen folder")),
                                                    ),
                                            )
                                    }),
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
}

/// Helper function returning AppearanceDetailPage Component
pub fn appearance_detail_page(
    theme_state: State<AppTheme>,
    store: SettingsStore,
) -> AppearanceDetailPage {
    AppearanceDetailPage { theme_state, store }
}

#[derive(PartialEq)]
pub struct WifiDetailPage {
    pub store: SettingsStore,
}

impl Component for WifiDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let t = use_app_theme();
        let is_wifi_locked = store.is_locked(&SettingKey::WifiEnabled);
        let wifi_lock = store.lock_label(&SettingKey::WifiEnabled);
        let wifi_on = *store.wifi_power.read();

        let net_state: State<Option<NetworkInfo>> = use_state(|| None);
        let loading = use_state(|| true);
        use_hook(move || {
            let mut ns = net_state;
            let mut ld = loading;
            spawn(async move {
                let info = tokio::task::spawn_blocking(fetch_network_info).await.unwrap_or_default();
                ns.set(Some(info));
                ld.set(false);
            });
        });

        let net = net_state.read().clone().unwrap_or_default();
        let active_label = if !wifi_on {
            "Not connected".to_string()
        } else if let Some(ssid) = &net.active_ssid {
            ssid.clone()
        } else {
            "Not connected".to_string()
        };
        let active_sub = if !wifi_on {
            Some("Wi-Fi is off".to_string())
        } else if net.active_ssid.is_some() {
            Some(format!("{} · {}", net.primary_interface, net.primary_ip))
        } else {
            Some("Not associated".to_string())
        };
        let sec_label = net
            .available_networks
            .iter()
            .find(|n| Some(&n.ssid) == net.active_ssid.as_ref())
            .map(|n| n.security.clone())
            .unwrap_or_else(|| if wifi_on { "—".to_string() } else { "—".to_string() });

        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(WIFI, "Wi-Fi", "Manage saved networks and connection preferences"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
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
                                active_label,
                                active_sub,
                                false,
                                label().font_size(12.).color(t.accent).text(if wifi_on && net.active_ssid.is_some() { "Connected" } else { "" }),
                            ))
                            .child(setting_row("IP address", None::<String>, true, label().font_size(12.).color(t.text_dim).text(net.primary_ip.clone())))
                            .child(setting_row("Security", None::<String>, true, label().font_size(12.).color(t.text_dim).text(sec_label))),
                    )
                    .child(
                        grid2([
                            rect()
                                .width(Size::flex(1.))
                                .child({
                                    let mut known = tile().child(tile_head(None, "Known networks", None::<String>));
                                    if *loading.read() {
                                        known = known.child(tile_sub("Scanning…"));
                                    } else if net.known_networks.is_empty() {
                                        known = known.child(tile_sub("No saved networks"));
                                    } else {
                                        for kn in net.known_networks.iter().take(5) {
                                            let is_conn = Some(&kn.ssid) == net.active_ssid.as_ref();
                                            known = known.child(setting_row(
                                                kn.ssid.clone(),
                                                Some(kn.security.clone()),
                                                false,
                                                if is_conn { status_chip("Connected", true, None).into_element() } else { status_chip("Saved", false, None).into_element() },
                                            ));
                                        }
                                    }
                                    known
                                }),
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
}

pub fn wifi_detail_page(store: SettingsStore) -> WifiDetailPage {
    WifiDetailPage { store }
}

#[derive(PartialEq)]
pub struct BluetoothDetailPage {
    pub store: SettingsStore,
}

impl Component for BluetoothDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let t = use_app_theme();
        let bt_lock = store.lock_label(&SettingKey::BluetoothEnabled);
        let is_bt_locked = bt_lock.is_some();
        let bt_on = *store.bt_power.read();
        let disc_on = *store.bt_discoverable.read();

        let devices: State<Vec<system::BluetoothDevice>> = use_state(Vec::new);
        let loading: State<bool> = use_state(|| true);
        let scan_nearby: State<Vec<system::BluetoothDevice>> = use_state(Vec::new);

        use_hook(move || {
            let mut dev = devices;
            let mut ld = loading;
            let mut nearby = scan_nearby;
            spawn(async move {
                let paired = tokio::task::spawn_blocking(|| system::HyprlandBackend.get_paired_bluetooth_devices())
                    .await
                    .unwrap_or_default();
                let all_nearby = paired.iter().filter(|d| !d.connected).cloned().collect::<Vec<_>>();
                dev.set(paired);
                nearby.set(all_nearby);
                ld.set(false);
            });
        });

        let paired = devices.read().clone();
        let nearby_devices = scan_nearby.read().clone();
        let is_loading = *loading.read();

        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(BLUETOOTH, "Bluetooth", "Connect and manage nearby devices"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
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
                                                            std::thread::spawn(move || {
                                                                system::HyprlandBackend.set_bluetooth_status(v);
                                                            });
                                                        }
                                                    })
                                                }),
                                        ),
                                ),
                            ))
                            .child(setting_row("Discoverable", None::<String>, false, {
                                let mut d = store.bt_discoverable;
                                pill_switch(disc_on, move |v| d.set(v))
                            }))
                            .child(tile_sub(if bt_on {
                                "Bluetooth is on — scanning for devices"
                            } else {
                                "Turn on Bluetooth to connect to devices"
                            })),
                    )
                    .child(
                        grid2([
                            rect().width(Size::flex(1.)).child({
                                let mut base = tile().child(tile_head(None, "My devices", None::<String>));
                                if is_loading {
                                    base = base.child(tile_sub("Loading paired devices…"));
                                } else if paired.is_empty() {
                                    base = base.child(tile_sub("No paired devices")).child(
                                        rect().margin((8., 0., 0., 0.)).child(ghost_button("Scan for devices", {
                                            let mut ld = loading;
                                            move || ld.set(true)
                                        })),
                                    );
                                } else {
                                    for dev in paired.iter().take(6) {
                                        let mac = dev.mac.clone();
                                        let name = dev.name.clone();
                                        let is_conn = dev.connected;
                                        let status = if is_conn { "Connected" } else { "Paired" };
                                        let mac2 = mac.clone();
                                        let mac3 = mac.clone();
                                        base = base.child(setting_row(
                                            name,
                                            Some(mac.clone()),
                                            false,
                                            rect()
                                                .horizontal()
                                                .spacing(8.)
                                                .content(Content::Flex)
                                                .child(status_chip(status, is_conn, None))
                                                .child(secondary_button(if is_conn { "Disconnect" } else { "Connect" }, move || {
                                                    let m = mac.clone();
                                                    let c = is_conn;
                                                    std::thread::spawn(move || {
                                                        if c {
                                                            system::HyprlandBackend.disconnect_bluetooth_device(&m);
                                                        } else {
                                                            system::HyprlandBackend.connect_bluetooth_device(&m);
                                                        }
                                                    });
                                                }))
                                                .child(ghost_button("Forget", move || {
                                                    let m = mac2.clone();
                                                    std::thread::spawn(move || {
                                                        system::HyprlandBackend.remove_bluetooth_device(&m);
                                                    });
                                                })),
                                        ));
                                        let _ = mac3;
                                    }
                                }
                                base
                            }),
                            rect().width(Size::flex(1.)).child({
                                let mut base = tile().child(tile_head(None, "Nearby", None::<String>));
                                if is_loading {
                                    base = base.child(tile_sub("Scanning…"));
                                } else if nearby_devices.is_empty() {
                                    base = base.child(tile_sub("No nearby devices found")).child(
                                        rect()
                                            .margin((8., 0., 0., 0.))
                                            .horizontal()
                                            .spacing(8.)
                                            .child(ghost_button("Start scan", || {
                                                std::thread::spawn(|| {
                                                    let _ = std::process::Command::new("bluetoothctl").args(["scan", "on"]).output();
                                                });
                                            }))
                                            .child(label().font_size(11.).color(t.text_dim).text("bluetoothctl scan")),
                                    );
                                } else {
                                    for dev in nearby_devices.iter().take(5) {
                                        let mac = dev.mac.clone();
                                        let name = dev.name.clone();
                                        base = base.child(setting_row(
                                            name,
                                            Some(mac.clone()),
                                            false,
                                            secondary_button("Pair", move || {
                                                let m = mac.clone();
                                                std::thread::spawn(move || {
                                                    let _ = std::process::Command::new("bluetoothctl").args(["pair", &m]).output();
                                                    let _ = std::process::Command::new("bluetoothctl").args(["connect", &m]).output();
                                                });
                                            }),
                                        ));
                                    }
                                }
                                base
                            }),
                        ]),
                    ),
            )
    }
}

pub fn bluetooth_detail_page(store: SettingsStore) -> BluetoothDetailPage {
    BluetoothDetailPage { store }
}

#[derive(PartialEq)]
pub struct DisplayDetailPage {
    pub store: SettingsStore,
    pub displays: State<Vec<DisplayInfo>>,
}

impl Component for DisplayDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let displays = self.displays;
        let selected_name: State<Option<String>> = use_state(|| None);
        let dragged_name: State<Option<String>> = use_state(|| None);
        let order_status: State<Option<Result<String, String>>> = use_state(|| None);
        let t = use_app_theme();

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
        let is_arrangement_locked = is_res_locked;

        let displays_list = displays.read().clone();
        let displays_len = displays_list.len();

        let cur_selected_name: Option<String> = {
            let s = selected_name.read().clone();
            if s.as_ref().map_or(false, |name| displays_list.iter().any(|d| &d.name == name)) {
                s
            } else {
                displays_list.first().map(|d| d.name.clone())
            }
        };

        let selected_display: Option<DisplayInfo> = displays_list
            .iter()
            .find(|d| Some(&d.name) == cur_selected_name.as_ref())
            .cloned();

        let status_chip_text: Option<Element> = match &*order_status.read() {
            Some(Ok(msg)) => Some(status_chip(msg.clone(), false, None).into_element()),
            Some(Err(e)) => {
                let msg = if e.len() > 48 { format!("{}…", &e[..48]) } else { e.clone() };
                Some(status_chip(format!("Error: {msg}"), true, None).into_element())
            }
            None => None,
        };
        let refresh_btn = {
            let mut ds = displays;
            let mut status = order_status;
            rect()
                .cursor(CursorIcon::Pointer)
                .padding((4., 8.))
                .corner_radius(6.)
                .background(t.panel_raised)
                .border(Border::new().width(1.).fill(t.border))
                .on_press(move |_| {
                    status.set(None);
                    freya::prelude::spawn(async move {
                        let d = tokio::task::spawn_blocking(move || {
                            system::HyprlandBackend.get_displays()
                        })
                        .await
                        .unwrap_or_default();
                        ds.set(d);
                    });
                })
                .child(label().font_size(11.).color(t.text_dim).text("Detect Displays"))
        };
        let header_right: Option<Element> = {
            let lock = res_lock.clone().map(|l| lock_badge(l).into_element());
            let mut right_row = rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .child(refresh_btn);
            if let Some(l) = lock {
                right_row = right_row.child(l);
            }
            if let Some(s) = status_chip_text {
                right_row = right_row.child(s);
            }
            Some(right_row.into_element())
        };

        let arrangement_tile = tile()
            .child(tile_head(None, "Arrangement", header_right))
            .child(tile_sub("Drag displays to match your physical arrangement. Click a display to customize its settings."))
            .child({
                if displays_len == 0 {
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
                    let items: Vec<DisplayMockItem> = displays_list
                        .iter()
                        .map(|d| {
                            let is_sel = Some(&d.name) == cur_selected_name.as_ref();
                            let res_label = if d.resolution.is_empty() {
                                String::new()
                            } else if let Ok(hz) = d.refresh_rate.parse::<f64>() {
                                format!("{} ({:.0}Hz)", d.resolution, hz)
                            } else {
                                format!("{} ({})", d.resolution, d.refresh_rate)
                            };
                            DisplayMockItem {
                                name: d.name.clone(),
                                description: d.description.clone(),
                                resolution: res_label,
                                height_px: 68.,
                                transform: d.transform,
                                is_selected: is_sel,
                            }
                        })
                        .collect();

                    let on_select = {
                        let mut sel = selected_name;
                        EventHandler::new(move |name: String| {
                            sel.set(Some(name));
                        })
                    };

                    let on_swap = {
                        let mut displays_state = displays;
                        let locked = is_arrangement_locked;
                        EventHandler::new(move |(from_name, to_name): (String, String)| {
                            if locked {
                                return;
                            }
                            let current_list = displays_state.read().clone();
                            if let (Some(pos1), Some(pos2)) = (
                                current_list.iter().position(|d| d.name == from_name),
                                current_list.iter().position(|d| d.name == to_name),
                            ) {
                                let mut new_order = current_list;
                                new_order.swap(pos1, pos2);
                                displays_state.set(new_order);
                            }
                        })
                    };

                    let on_drag_end = {
                        let mut displays_state = displays;
                        let mut status_state = order_status;
                        let locked = is_arrangement_locked;
                        EventHandler::new(move |()| {
                            if locked {
                                return;
                            }
                            let names: Vec<String> =
                                displays_state.read().iter().map(|d| d.name.clone()).collect();
                            status_state.set(None);
                            freya::prelude::spawn(async move {
                                let res = tokio::task::spawn_blocking(move || {
                                    system::HyprlandBackend.set_display_order(&names)
                                })
                                .await;
                                match res {
                                    Ok(Ok(())) => {
                                        let updated = tokio::task::spawn_blocking(move || {
                                            system::HyprlandBackend.get_displays()
                                        })
                                        .await
                                        .unwrap_or_default();
                                        if !updated.is_empty() {
                                            displays_state.set(updated);
                                        }
                                        status_state.set(Some(Ok("Arrangement saved".to_string())));
                                    }
                                    Ok(Err(e)) => status_state.set(Some(Err(e))),
                                    Err(e) => {
                                        status_state.set(Some(Err(format!("join error: {e}"))))
                                    }
                                }
                            });
                        })
                    };

                    rect()
                        .width(Size::fill())
                        .margin((10., 0., 0., 0.))
                        .opacity(if is_arrangement_locked { 0.45 } else { 1.0 })
                        .child(draggable_displays_mock(
                            items,
                            dragged_name,
                            on_swap,
                            Some(on_select),
                            Some(on_drag_end),
                        ))
                        .into_element()
                }
            });

        let selected_tile = if let Some(selected) = selected_display {
            let sel_name = selected.name.clone();
            let sel_desc = if !selected.description.is_empty() {
                format!("{} ({})", selected.description, selected.name)
            } else {
                selected.name.clone()
            };
            let sel_res = selected.resolution.clone();
            let sel_hz = if let Ok(hz) = selected.refresh_rate.parse::<f64>() {
                format!("{:.0} Hz", hz)
            } else if !selected.refresh_rate.is_empty() {
                selected.refresh_rate.clone()
            } else {
                "60 Hz".to_string()
            };

            let mut distinct_resolutions: Vec<String> = Vec::new();
            for m in &selected.available_modes {
                let res_part = m.split('@').next().unwrap_or("").trim().to_string();
                if !res_part.is_empty() && !distinct_resolutions.contains(&res_part) {
                    distinct_resolutions.push(res_part);
                }
            }
            if distinct_resolutions.is_empty() {
                distinct_resolutions = vec![
                    "1920x1080".to_string(),
                    "1600x900".to_string(),
                    "1280x1024".to_string(),
                    "1280x720".to_string(),
                ];
            }
            let top_resolutions: Vec<String> = distinct_resolutions.into_iter().take(5).collect();

            let rotation_items = vec![
                ("Standard (0°)", 0u8),
                ("90° Portrait", 1u8),
                ("180° Inverted", 2u8),
                ("270° Portrait", 3u8),
            ];
            let cur_transform = selected.transform;

            let rotation_control = {
                let mut ds = displays;
                let mut status_state = order_status;
                let mut sel = selected_name;
                let sel_n = sel_name.clone();
                let locked = is_res_locked;
                segmented_control(
                    rotation_items,
                    cur_transform,
                    move |new_tf: u8| {
                        if locked {
                            return;
                        }
                        status_state.set(None);
                        let name = sel_n.clone();
                        sel.set(Some(name.clone()));
                        freya::prelude::spawn(async move {
                            let res = tokio::task::spawn_blocking(move || {
                                system::HyprlandBackend.set_display_config(&name, None, Some(new_tf))
                            })
                            .await;
                            match res {
                                Ok(Ok(())) => {
                                    let updated = tokio::task::spawn_blocking(move || {
                                        system::HyprlandBackend.get_displays()
                                    })
                                    .await
                                    .unwrap_or_default();
                                    if !updated.is_empty() {
                                        ds.set(updated);
                                    }
                                    status_state.set(Some(Ok("Rotation applied".to_string())));
                                }
                                Ok(Err(e)) => status_state.set(Some(Err(e))),
                                Err(e) => {
                                    status_state.set(Some(Err(format!("join error: {e}"))))
                                }
                            }
                        });
                    },
                )
            };

            let res_items: Vec<(String, String)> = top_resolutions
                .iter()
                .map(|r| (r.replace('x', " × "), r.clone()))
                .collect();
            let cur_res_choice = sel_res.clone();

            let resolution_control = {
                let mut ds = displays;
                let mut status_state = order_status;
                let mut sel = selected_name;
                let sel_n = sel_name.clone();
                let locked = is_res_locked;
                let modes = selected.available_modes.clone();
                segmented_control_dynamic(
                    res_items,
                    cur_res_choice,
                    move |chosen_res: String| {
                        if locked {
                            return;
                        }
                        status_state.set(None);
                        let name = sel_n.clone();
                        sel.set(Some(name.clone()));
                        let target_mode = modes
                            .iter()
                            .find(|m| m.starts_with(&chosen_res))
                            .cloned()
                            .unwrap_or_else(|| chosen_res.clone());

                        freya::prelude::spawn(async move {
                            let res = tokio::task::spawn_blocking(move || {
                                system::HyprlandBackend.set_display_config(&name, Some(&target_mode), None)
                            })
                            .await;
                            match res {
                                Ok(Ok(())) => {
                                    let updated = tokio::task::spawn_blocking(move || {
                                        system::HyprlandBackend.get_displays()
                                    })
                                    .await
                                    .unwrap_or_default();
                                    if !updated.is_empty() {
                                        ds.set(updated);
                                    }
                                    status_state.set(Some(Ok("Resolution applied".to_string())));
                                }
                                Ok(Err(e)) => status_state.set(Some(Err(e))),
                                Err(e) => {
                                    status_state.set(Some(Err(format!("join error: {e}"))))
                                }
                            }
                        });
                    },
                )
            };

            tile()
                .child(tile_head(
                    None,
                    format!("Display Settings — {sel_desc}"),
                    res_lock.as_ref().map(|l| lock_badge(l)),
                ))
                .child(setting_row(
                    "Current mode",
                    None::<String>,
                    false,
                    label()
                        .font_size(12.)
                        .color(t.text_dim)
                        .text(format!("{sel_res} @ {sel_hz}")),
                ))
                .child(
                    rect()
                        .margin((12., 0., 4., 0.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(field_label("Resolution")),
                )
                .child(
                    rect()
                        .opacity(if is_res_locked { 0.45 } else { 1.0 })
                        .child(resolution_control),
                )
                .child(
                    rect()
                        .margin((16., 0., 4., 0.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(field_label("Orientation / Rotation")),
                )
                .child(
                    rect()
                        .opacity(if is_res_locked { 0.45 } else { 1.0 })
                        .child(rotation_control),
                )
                .into_element()
        } else {
            rect().into_element()
        };

        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(DISPLAY, "Display", "Arrangement, brightness, resolution and colour"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
                    // Wide: Arrangement — interactive draggable (top priority)
                    .child(arrangement_tile)
                    // Wide: Per-display configuration (Resolution & Rotation)
                    .maybe_child((displays_len > 0).then(|| selected_tile))
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
                    ),
            )
    }
}

/// Display detail page helper returning DisplayDetailPage component
pub fn display_detail_page(
    store: SettingsStore,
    displays: State<Vec<DisplayInfo>>,
) -> DisplayDetailPage {
    DisplayDetailPage { store, displays }
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

pub fn screen_time_detail_page(store: SettingsStore) -> impl IntoElement {
    let t = use_app_theme();
    let st_lock = store.lock_label(&SettingKey::ScreenTimeEnabled);
    let is_st_locked = st_lock.is_some();
    let enabled = *store.screen_time_enabled.read();
    let downtime_on = *store.downtime_enabled.read();
    let app_limits_on = *store.app_limits_enabled.read();
    let from = store.downtime_from.read().clone();
    let to = store.downtime_to.read().clone();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(GENERAL, "Screen Time", "App usage limits, downtime and communication"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Screen Time",
                            Some(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .maybe_child(st_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(
                                        rect()
                                            .opacity(if is_st_locked { 0.45 } else { 1.0 })
                                            .child({
                                                let store = store;
                                                pill_switch(enabled, move |v| {
                                                    if !is_st_locked {
                                                        store.set(SettingKey::ScreenTimeEnabled, v);
                                                    }
                                                })
                                            }),
                                    ),
                            ),
                        ))
                        .child(tile_sub(if enabled {
                            "Limits and downtime are enforced"
                        } else {
                            "Screen Time is off"
                        }))
                        .child(
                            rect()
                                .width(Size::fill())
                                .margin((12., 0., 0., 0.))
                                .opacity(if !enabled { 0.45 } else { 1.0 })
                                .child(mini_bar_chart([62., 48., 71., 55., 80., 44., 67.], ["M", "T", "W", "T", "F", "S", "S"])),
                        )
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .main_align(Alignment::SpaceBetween)
                                .margin((8., 0., 0., 0.))
                                .child(label().font_size(11.).color(t.text_dim).text("Weekly average: 3h 42m / day"))
                                .child(label().font_size(11.).color(t.accent).text("See All Activity →")),
                        ),
                )
                .child(
                    grid2([
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "App Limits", st_lock.as_ref().map(|l| lock_badge(l))))
                                .child(
                                    rect()
                                        .opacity(if !enabled { 0.45 } else { 1.0 })
                                        .vertical()
                                        .child(setting_row("Limits enabled", None::<String>, false, {
                                            let store = store;
                                            pill_switch(app_limits_on, move |v| {
                                                if !is_st_locked {
                                                    let mut s = store.app_limits_enabled;
                                                    s.set(v);
                                                }
                                            })
                                        }))
                                        .child(setting_row("Social · 1h 30m", Some("Instagram, TikTok"), false, status_chip("1h 12m left", true, None)))
                                        .child(setting_row("Games · 1h", Some("Steam, Minecraft"), true, status_chip("42m left", true, None)))
                                        .child(setting_row("Browser · 2h", Some("Safari, Chrome"), true, ghost_button("Add Limit", || {}))),
                                ),
                        ),
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Downtime", st_lock.as_ref().map(|l| lock_badge(l))))
                                .child(
                                    rect()
                                        .opacity(if !enabled { 0.45 } else { 1.0 })
                                        .vertical()
                                        .child(setting_row("Downtime", None::<String>, false, {
                                            let store = store;
                                            pill_switch(downtime_on, move |v| {
                                                if !is_st_locked {
                                                    let mut s = store.downtime_enabled;
                                                    s.set(v);
                                                }
                                            })
                                        }))
                                        .child(setting_row(
                                            "Schedule",
                                            Some(format!("{from} — {to}")),
                                            true,
                                            ghost_button("Edit", || {}),
                                        ))
                                        .child(tile_sub("Only apps you allow and phone calls will be available")),
                                ),
                        ),
                    ]),
                )
                .child(
                    grid2([
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Communication", None::<String>))
                                .child(setting_row("During downtime", Some("Contacts only"), false, status_chip("Contacts", false, None)))
                                .child(setting_row("During Screen Time", Some("Everyone"), true, ghost_button("Manage", || {}))),
                        ),
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Content & Privacy", st_lock.as_ref().map(|l| lock_badge(l))))
                                .child(setting_row("Content restrictions", None::<String>, false, pill_switch(false, |_| {})))
                                .child(setting_row("App installs", None::<String>, true, pill_switch(true, |_| {}))),
                        ),
                    ]),
                ),
        )
}

pub fn desktop_detail_page(store: SettingsStore) -> impl IntoElement {
    let layout = *store.window_layout.read();
    let gap = *store.workspace_gap.read();
    let dock_pos = *store.dock_position.read();
    let dock_sz = *store.dock_size.read();
    let dock_hide = *store.dock_autohide.read();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(LAYOUT_GRID, "Desktop & Dock", "Workspaces, window management and dock"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(
                    grid2([
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Window Management", None::<String>))
                                .child(field_label("Layout"))
                                .child({
                                    let store = store;
                                    segmented_control(
                                        vec![("Master", 0), ("Dwindle", 1), ("Floating", 2)],
                                        layout,
                                        move |v| {
                                            let mut s = store.window_layout;
                                            s.set(v);
                                            let val = match v {
                                                0 => "master",
                                                1 => "dwindle",
                                                _ => "floating",
                                            };
                                            store.set(SettingKey::Custom("desktop.layout".to_string()), val);
                                        },
                                    )
                                })
                                .child(
                                    rect()
                                        .margin((14., 0., 0., 0.))
                                        .child(field_label(format!("Workspace gap · {} px", gap as i32))),
                                )
                                .child({
                                    let store = store;
                                    slider_row(None, gap, move |v| {
                                        let mut s = store.workspace_gap;
                                        s.set(v);
                                        store.set(SettingKey::Custom("desktop.workspace_gap".to_string()), v);
                                        let _ = std::process::Command::new("hyprctl")
                                            .args(["keyword", "general:gaps_out", &format!("{}", v as i32)])
                                            .output();
                                    })
                                })
                                .child(tile_sub("Gaps are applied live via hyprctl")),
                        ),
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Dock", None::<String>))
                                .child(field_label("Position"))
                                .child({
                                    let store = store;
                                    segmented_control(
                                        vec![("Bottom", 0), ("Left", 1), ("Hidden", 2)],
                                        dock_pos,
                                        move |v| {
                                            let mut s = store.dock_position;
                                            s.set(v);
                                            store.set(SettingKey::Custom("desktop.dock_position".to_string()), v as i64);
                                        },
                                    )
                                })
                                .child(
                                    rect()
                                        .margin((12., 0., 0., 0.))
                                        .child(field_label(format!("Size · {} px", dock_sz as i32))),
                                )
                                .child({
                                    let store = store;
                                    slider_row(None, dock_sz, move |v| {
                                        let mut s = store.dock_size;
                                        s.set(v.clamp(36., 72.));
                                        store.set(SettingKey::Custom("desktop.dock_size".to_string()), v);
                                    })
                                })
                                .child(setting_row("Autohide", None::<String>, true, {
                                    let store = store;
                                    pill_switch(dock_hide, move |v| {
                                        let mut s = store.dock_autohide;
                                        s.set(v);
                                        store.set(SettingKey::Custom("desktop.dock_autohide".to_string()), v);
                                    })
                                })),
                        ),
                    ]),
                )
                .child(
                    tile()
                        .child(tile_head(None, "Workspaces", None::<String>))
                        .child(tile_sub("Drag to reorder — gaps and layout update live"))
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(92.))
                                .horizontal()
                                .spacing(10.)
                                .margin((10., 0., 0., 0.))
                                .content(Content::Flex)
                                .children((0..5).map(|i| {
                                    let t = use_app_theme();
                                    let is_active = i == 1;
                                    rect()
                                        .width(Size::flex(1.))
                                        .height(Size::fill())
                                        .corner_radius(10.)
                                        .background(if is_active { t.accent } else { t.panel_raised })
                                        .border(Border::new().width(1.).fill(if is_active { t.accent } else { t.border }))
                                        .center()
                                        .child(label().font_size(13.).font_weight(FontWeight::BOLD).color(if is_active { Color::WHITE } else { t.text_dim }).text(format!("{}", i + 1)))
                                })),
                        )
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .spacing(8.)
                                .margin((12., 0., 0., 0.))
                                .content(Content::Flex)
                                .child(rect().width(Size::flex(1.)).child(ghost_button("Add Desktop", || {})))
                                .child(rect().width(Size::flex(1.)).child(secondary_button("Reset Layout", || {}))),
                        ),
                )
                .child(
                    grid2([
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Widgets", None::<String>))
                                .child(setting_row("Show widgets", Some("Clock, weather on desktop"), false, pill_switch(true, |_| {})))
                                .child(setting_row("Widget style", Some("Translucent"), true, status_chip("Translucent", false, None))),
                        ),
                        rect().width(Size::flex(1.)).child(
                            tile()
                                .child(tile_head(None, "Mission Control", None::<String>))
                                .child(setting_row("Hot corner", Some("Top-left · Mission Control"), false, ghost_button("Configure", || {})))
                                .child(setting_row("Swipe gesture", Some("Three-finger up"), true, pill_switch(true, |_| {}))),
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
