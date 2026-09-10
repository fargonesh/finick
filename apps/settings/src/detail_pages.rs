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
                                        .vertical()
                                        .spacing(8.)
                                        .child({
                                            let mut th = theme_state;
                                            let store = store;
                                            let wp = store.wallpaper.read().clone();
                                            accent_picker_with_material(
                                                t.accent_name,
                                                wp,
                                                true,
                                                move |acc| {
                                                    if !is_accent_locked {
                                                        let new_t = th.read().with_accent(acc);
                                                        th.set(new_t);
                                                        set_theme(&new_t);
                                                        store.set(SettingKey::AccentColor, acc.hex.to_string());
                                                        let clean_hex = acc.hex.trim_start_matches('#');
                                                        let _ = std::process::Command::new("hyprctl")
                                                            .args(["keyword", "general:col.active_border", &format!("0xff{clean_hex}")])
                                                            .status();
                                                    }
                                                },
                                            )
                                        })

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
                                            let raw = std::path::Path::new(clean)
                                                .file_name()
                                                .map(|f| f.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "Custom image".to_string());
                                            if raw.chars().count() > 24 {
                                                format!("{}…", raw.chars().take(24).collect::<String>())
                                            } else {
                                                raw
                                            }
                                        } else {
                                            "Custom image".to_string()
                                        };
                                        let image_subtext = if is_custom_image {
                                            Some("Custom desktop wallpaper".to_string())
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
                                                                            .child(label().font_size(11.).color(t.text_dim).text("No images found in folder"))
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
                                                                                        let t = t;
                                                                                        move |path| {
                                                                                            let is_sel = cur == path || cur == format!("file://{path}");
                                                                                            let p = path.clone();
                                                                                            let store = store;
                                                                                            rect()
                                                                                                .width(Size::flex(1.))
                                                                                                .height(Size::px(52.))
                                                                                                .corner_radius(8.)
                                                                                                .background(t.panel_raised)
                                                                                                .border(Border::new().width(if is_sel { 2. } else { 1. }).fill(if is_sel { t.accent } else { t.border }))
                                                                                                .cursor(CursorIcon::Pointer)
                                                                                                .center()
                                                                                                .on_press(move |_| {
                                                                                                    store.set(SettingKey::Wallpaper, p.clone());
                                                                                                })
                                                                                                .child(label().font_size(9.).color(t.text_dim).text(
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
                                                            .maybe_child((!store.wallpaper_folder.read().is_empty()).then(|| {
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
                                                                    })
                                                            }))
                                                            .maybe_child(store.wallpaper_folder.read().is_empty().then(|| rect().margin((8.,0.,0.,0.)).child(tile_sub("Pick a folder to enable slideshow"))))
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
                                                        let mut th = theme_state;
                                                        segmented_control(
                                                            vec![("Automatically", 0), ("When scrolling", 1), ("Always", 2)],
                                                            *store.scrollbar_pref.read(),
                                                            move |idx| {
                                                                if !is_scroll_locked {
                                                                    let new_t = th.read().with_scrollbar_pref(idx);
                                                                    th.set(new_t);
                                                                    set_theme(&new_t);
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
                                                        let mut th = theme_state;
                                                        segmented_control(
                                                            vec![("Small", 0), ("Medium", 1), ("Large", 2)],
                                                            *store.icon_size_pref.read(),
                                                            move |idx| {
                                                                if !is_icon_locked {
                                                                    let new_t = th.read().with_icon_size(idx);
                                                                    th.set(new_t);
                                                                    set_theme(&new_t);
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
                let initial = tokio::task::spawn_blocking(fetch_network_info).await.unwrap_or_default();
                ns.set(Some(initial));
                ld.set(false);
                let _ = tokio::task::spawn_blocking(|| {
                    let _ = std::process::Command::new("nmcli").args(["dev", "wifi", "rescan"]).output();
                }).await;
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                let updated = tokio::task::spawn_blocking(fetch_network_info).await.unwrap_or_default();
                if !updated.available_networks.is_empty() {
                    ns.set(Some(updated));
                }
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
                        tile()
                            .child(tile_head(None, "Nearby networks", None::<String>))
                            .child({
                                let mut base = rect().width(Size::fill()).vertical();
                                if *loading.read() {
                                    base = base.child(tile_sub("Scanning for nearby networks…"));
                                } else if net.available_networks.is_empty() {
                                    base = base.child(tile_sub("No nearby networks found"));
                                } else {
                                    for nw in net.available_networks.iter().take(8) {
                                        let ssid = nw.ssid.clone();
                                        let is_conn = nw.connected || Some(&ssid) == net.active_ssid.as_ref();
                                        let ssid_conn = ssid.clone();
                                        base = base.child(setting_row(
                                            ssid,
                                            Some(format!("Signal {}% · {}", nw.signal, nw.security)),
                                            false,
                                            if is_conn {
                                                status_chip("Connected", true, None).into_element()
                                            } else {
                                                secondary_button("Connect", move || {
                                                    let s = ssid_conn.clone();
                                                    std::thread::spawn(move || {
                                                        system::HyprlandBackend.connect_wifi(&s);
                                                    });
                                                }).into_element()
                                            },
                                        ));
                                    }
                                }
                                base
                            }),
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
                let _ = tokio::task::spawn_blocking(|| {
                    let _ = std::process::Command::new("bluetoothctl").args(["scan", "on"]).output();
                })
                .await;
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                let paired = tokio::task::spawn_blocking(|| system::HyprlandBackend.get_paired_bluetooth_devices())
                    .await
                    .unwrap_or_default();
                let all_known = tokio::task::spawn_blocking(|| {
                    let out = std::process::Command::new("bluetoothctl").args(["devices"]).output().ok();
                    if let Some(o) = out {
                        let s = String::from_utf8_lossy(&o.stdout).to_string();
                        s.lines().filter_map(|line| {
                            let parts: Vec<&str> = line.splitn(3, ' ').collect();
                            if parts.len() >= 3 {
                                let mac = parts[1].to_string();
                                let raw_name = parts[2].trim().to_string();
                                let clean = raw_name.replace([':', '-'], "");
                                let is_mac_name = raw_name.is_empty() || (clean.len() == 12 && clean.chars().all(|c| c.is_ascii_hexdigit()));
                                let name = if is_mac_name {
                                    std::process::Command::new("bluetoothctl").args(["info", &mac]).output().ok().and_then(|info_o| {
                                        let is = String::from_utf8_lossy(&info_o.stdout);
                                        let mut alias = None;
                                        let mut n = None;
                                        for l in is.lines() {
                                            let t = l.trim();
                                            if let Some(rest) = t.strip_prefix("Alias:") { alias = Some(rest.trim().to_string()); }
                                            if let Some(rest) = t.strip_prefix("Name:") { n = Some(rest.trim().to_string()); }
                                        }
                                        alias.or(n)
                                    }).unwrap_or_else(|| format!("Device ({})", &mac[..mac.len().min(8)]))
                                } else {
                                    raw_name
                                };
                                Some(system::BluetoothDevice {
                                    name,
                                    mac,
                                    connected: false,
                                })
                            } else { None }
                        }).collect::<Vec<_>>()
                    } else { Vec::new() }
                }).await.unwrap_or_default();
                let paired_macs: std::collections::HashSet<String> = paired.iter().map(|d| d.mac.clone()).collect();
                let nearby_only: Vec<_> = all_known.into_iter().filter(|d| !paired_macs.contains(&d.mac)).collect();
                dev.set(paired);
                nearby.set(nearby_only);
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
                                    base = base.child(tile_sub("No paired devices — scanning…"));
                                } else {
                                    for dev in paired.iter().take(6) {
                                        let mac = dev.mac.clone();
                                        let name = dev.name.clone();
                                        let is_conn = dev.connected;
                                        let subtitle = if is_conn {
                                            format!("Connected · {}", mac)
                                        } else {
                                            mac.clone()
                                        };
                                        let mac_for_action = mac.clone();
                                        base = base.child(
                                            rect()
                                                .width(Size::fill())
                                                .horizontal()
                                                .cross_align(Alignment::Center)
                                                .main_align(Alignment::SpaceBetween)
                                                .content(Content::Flex)
                                                .padding((8., 12.))
                                                .margin((2., 0.))
                                                .corner_radius(8.)
                                                .cursor(CursorIcon::Pointer)
                                                .background(t.bg_card)
                                                .border(Border::new().width(1.).fill(t.border))
                                                .on_press(move |_| {
                                                    let m = mac_for_action.clone();
                                                    let c = is_conn;
                                                    std::thread::spawn(move || {
                                                        if c {
                                                            system::HyprlandBackend.disconnect_bluetooth_device(&m);
                                                        } else {
                                                            system::HyprlandBackend.connect_bluetooth_device(&m);
                                                        }
                                                    });
                                                })
                                                .child(
                                                    rect().vertical().spacing(2.)
                                                        .child(label().font_size(12.).color(t.text).text(name))
                                                        .child(label().font_size(10.).color(if is_conn { t.accent } else { t.text_dim }).text(subtitle))
                                                )
                                        );
                                    }
                                }
                                base
                            }),
                            rect().width(Size::flex(1.)).child({
                                let mut base = tile().child(tile_head(None, "Nearby", None::<String>));
                                if is_loading {
                                    base = base.child(tile_sub("Scanning…"));
                                } else if nearby_devices.is_empty() {
                                    base = base.child(tile_sub("No nearby devices found — scanning…"));
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
        let identify_active: State<bool> = use_state(|| false);
        let position_offsets: State<std::collections::HashMap<String, (i32,i32)>> = use_state(Default::default);
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
        let identify_btn = {
            let mut active = identify_active;
            let mut status = order_status;
            rect()
                .cursor(CursorIcon::Pointer)
                .padding((4., 8.))
                .corner_radius(6.)
                .background(t.accent)
                .on_press(move |_| {
                    active.set(true);
                    status.set(Some(Ok("Identifying displays…".to_string())));
                    let mut a = active;
                    let mut s = status;
                    freya::prelude::spawn(async move {
                        let _ = tokio::task::spawn_blocking(|| {
                            let _ = std::process::Command::new("hyprctl").args(["dispatch", "exec", "notify-send 'Display Identify' 'Overlay shown'"]).output();
                        }).await;
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        a.set(false);
                        s.set(Some(Ok("Identify finished".to_string())));
                    });
                })
                .child(label().font_size(11.).color(t.bg).text("Identify"))
        };
        let header_right: Option<Element> = {
            let lock = res_lock.clone().map(|l| lock_badge(l).into_element());
            let mut right_row = rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .child(refresh_btn)
                .child(identify_btn);
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
                    let offsets = position_offsets.read().clone();
                    let ident = *identify_active.read();
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
                            let (ox, oy) = offsets.get(&d.name).cloned().unwrap_or((d.x, d.y));
                            let (raw_w, raw_h) = system::parse_display_dimensions(&d.resolution);
                            let sc: f64 = d.scale.parse().unwrap_or(1.0);
                            let sc = if sc <= 0.0 {1.0} else {sc};
                            let mut lw = (raw_w as f64 / sc).round() as i32;
                            let mut lh = (raw_h as f64 / sc).round() as i32;
                            if d.transform == 1 || d.transform == 3 { std::mem::swap(&mut lw, &mut lh); }
                            DisplayMockItem {
                                name: d.name.clone(),
                                description: d.description.clone(),
                                resolution: res_label,
                                height_px: 68.,
                                transform: d.transform,
                                is_selected: is_sel,
                                x: ox,
                                y: oy,
                                identify_active: ident,
                                logical_w: lw,
                                logical_h: lh,
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
                        let mut offs_state = position_offsets;
                        let locked = is_arrangement_locked;
                        EventHandler::new(move |()| {
                            if locked {
                                return;
                            }
                            let offs = offs_state.read().clone();
                            let has_free = !offs.is_empty();
                            if has_free {
                                let positions: Vec<(String,i32,i32)> = displays_state.read().iter().map(|d| {
                                    if let Some((nx,ny)) = offs.get(&d.name) { (d.name.clone(), *nx, *ny) } else { (d.name.clone(), d.x, d.y) }
                                }).collect();
                                status_state.set(None);
                                freya::prelude::spawn(async move {
                                    let res = tokio::task::spawn_blocking(move || {
                                        system::HyprlandBackend.set_display_positions(&positions)
                                    }).await;
                                    match res {
                                        Ok(Ok(())) => {
                                            let updated = tokio::task::spawn_blocking(move || system::HyprlandBackend.get_displays()).await.unwrap_or_default();
                                            if !updated.is_empty() { displays_state.set(updated); }
                                            offs_state.set(Default::default());
                                            status_state.set(Some(Ok("Positions saved".to_string())));
                                        }
                                        Ok(Err(e)) => status_state.set(Some(Err(e))),
                                        Err(e) => status_state.set(Some(Err(format!("join error: {e}")))),
                                    }
                                });
                            } else {
                                let names: Vec<String> = displays_state.read().iter().map(|d| d.name.clone()).collect();
                                status_state.set(None);
                                freya::prelude::spawn(async move {
                                    let res = tokio::task::spawn_blocking(move || {
                                        system::HyprlandBackend.set_display_order(&names)
                                    }).await;
                                    match res {
                                        Ok(Ok(())) => {
                                            let updated = tokio::task::spawn_blocking(move || system::HyprlandBackend.get_displays()).await.unwrap_or_default();
                                            if !updated.is_empty() { displays_state.set(updated); }
                                            status_state.set(Some(Ok("Arrangement saved".to_string())));
                                        }
                                        Ok(Err(e)) => status_state.set(Some(Err(e))),
                                        Err(e) => status_state.set(Some(Err(format!("join error: {e}")))),
                                    }
                                });
                            }
                        })
                    };

                    let on_position = {
                        let mut offs = position_offsets;
                        let mut displays_state = displays;
                        EventHandler::new(move |(name, nx, ny): (String,i32,i32)| {
                            let list_snapshot = displays_state.read().clone();
                            let self_info = list_snapshot.iter().find(|d| d.name==name);
                            let (sw, sh) = if let Some(d) = self_info {
                                let (rw, rh) = system::parse_display_dimensions(&d.resolution);
                                let sc: f64 = d.scale.parse().unwrap_or(1.0);
                                let sc = if sc <= 0.0 {1.0} else {sc};
                                let mut w = (rw as f64 / sc).round() as i32;
                                let mut h = (rh as f64 / sc).round() as i32;
                                if d.transform==1||d.transform==3 { std::mem::swap(&mut w, &mut h); }
                                (w, h)
                            } else {(1920,1080)};
                            let (mut snap_x, mut snap_y) = (nx, ny);
                            if list_snapshot.len() > 1 {
                                let mut best: Option<(i32,i32,i32)> = None;
                                for d in &list_snapshot {
                                    if d.name == name { continue; }
                                    let (ox, oy) = (d.x, d.y);
                                    let (rw, rh) = system::parse_display_dimensions(&d.resolution);
                                    let sc: f64 = d.scale.parse().unwrap_or(1.0);
                                    let sc = if sc <= 0.0 {1.0} else {sc};
                                    let mut ow = (rw as f64 / sc).round() as i32;
                                    let mut oh = (rh as f64 / sc).round() as i32;
                                    if d.transform==1||d.transform==3 { std::mem::swap(&mut ow, &mut oh); }
                                    for (cx, cy) in [(ox+ow, oy), (ox - sw, oy), (ox, oy+oh), (ox, oy - sh)] {
                                        let dx = nx - cx;
                                        let dy = ny - cy;
                                        let dist = dx*dx + dy*dy;
                                        if best.is_none() || dist < best.unwrap().0 { best = Some((dist, cx, cy)); }
                                    }
                                }
                                if let Some((_, bx, by)) = best { snap_x = bx; snap_y = by; }
                            } else { snap_x = 0; snap_y = 0; }
                            snap_x = snap_x.clamp(-3000, 3000);
                            snap_y = snap_y.clamp(-2000, 2000);
                            let mut map = offs.read().clone();
                            map.insert(name.clone(), (snap_x, snap_y));
                            offs.set(map.clone());
                            let mut list = displays_state.read().clone();
                            for d in list.iter_mut() { if d.name == name { d.x = snap_x; d.y = snap_y; } }
                            displays_state.set(list);
                        })
                    };
                    rect()
                        .width(Size::fill())
                        .margin((10., 0., 0., 0.))
                        .opacity(if is_arrangement_locked { 0.45 } else { 1.0 })
                        .child(draggable_displays_mock_with_position(
                            items,
                            dragged_name,
                            on_swap,
                            Some(on_select),
                            Some(on_drag_end),
                            on_position,
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
                    // Wide: Brightness (hidden when unavailable)
                    .maybe_child({
                        let has_backlight = system::HyprlandBackend.supports_brightness();
                        has_backlight.then(|| {
                            let opacity = if is_br_locked { 0.45 } else { 1.0 };
                            tile()
                                .child(tile_head(
                                    None,
                                    "Brightness",
                                    br_lock.as_ref().map(|l| lock_badge(l)),
                                ))
                                .child(
                                    rect()
                                        .opacity(opacity)
                                        .child({
                                            let store = store;
                                            slider_row(Some(SUN), *store.brightness.read(), move |v| {
                                                if !is_br_locked {
                                                    store.set(SettingKey::DisplayBrightness, v);
                                                    std::thread::spawn(move || system::HyprlandBackend.set_brightness(v as u32));
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
                        })
                    })
                    // Wide: Night Shift (wired, resolution removed — per-display controls above are functional)
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
                                                            let ct = *store.color_temp.read() as f32;
                                                            std::thread::spawn(move || {
                                                                let _ = system::HyprlandBackend.set_night_shift(v, ct);
                                                            });
                                                        }
                                                    })
                                                }),
                                        ),
                                ),
                            ))
                            .child({
                                let store = store;
                                segmented_control(
                                    vec![("Off", 0), ("Sunset to sunrise", 1), ("Custom", 2)],
                                    *store.night_shift_mode.read(),
                                    move |idx| {
                                        let mut nm = store.night_shift_mode;
                                        nm.set(idx);
                                        store.set(SettingKey::Custom("display.night_shift_mode".to_string()), idx as i64);
                                    },
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
                                                if *store.night_shift.read() {
                                                    std::thread::spawn(move || { let _ = system::HyprlandBackend.set_night_shift(true, v as f32); });
                                                }
                                            }
                                        })
                                    }),
                            ),
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

#[derive(PartialEq)]
struct InputLevelMeter;

impl Component for InputLevelMeter {
    fn render(&self) -> impl IntoElement {
        let level: State<f64> = use_state(|| 35.0);
        use_hook({
            let mut lvl = level;
            move || {
                spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        let t_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
                        let n = ((t_ms / 150) % 100) as f64;
                        let wave = ((t_ms as f64 * 0.003).sin() * 20.0).abs();
                        let v = 15.0 + wave + (n * 0.3).clamp(0.0, 45.0);
                        lvl.set(v.clamp(5.0, 95.0));
                    }
                });
            }
        });
        input_level_pill(*level.read())
    }
}

#[derive(PartialEq)]
pub struct SoundDetailPage {
    pub store: SettingsStore,
}

impl Component for SoundDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let sinks = use_hook(|| system::HyprlandBackend.get_audio_sinks());
        let sources = use_hook(|| system::HyprlandBackend.get_audio_sources());
        let alert_sound = use_state(|| "Tri-tone".to_string());
        let vol_lock = store.lock_label(&SettingKey::AudioVolume);
        let is_vol_locked = vol_lock.is_some();
        let mute_lock = store.lock_label(&SettingKey::AudioMuted);
        let is_mute_locked = mute_lock.is_some();
        let sink_name = store.default_sink.read().clone();
        let source_name = store.default_source.read().clone();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(SOUND, "Sound", "Output, input and alert sounds"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
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
                                    .maybe_child(mute_lock.as_ref().map(|l| lock_badge(l)))
                                    .child(tile_sub(if sink_name.is_empty() { "Studio Speakers".to_string() } else { sink_name.clone() })),
                            ),
                        ))
                        .child(
                            rect()
                                .opacity(if is_vol_locked && is_mute_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    let muted = *store.is_mute.read();
                                    let vol = *store.volume.read();
                                    slider_row_with_mute(
                                        Some(SOUND),
                                        vol,
                                        muted,
                                        move |v| {
                                            if !is_vol_locked {
                                                store.set(SettingKey::AudioVolume, v as i64);
                                                std::thread::spawn(move || system::HyprlandBackend.set_volume(v as i32));
                                            }
                                        },
                                        move |m| {
                                            if !is_mute_locked {
                                                store.set(SettingKey::AudioMuted, m);
                                                std::thread::spawn(move || system::HyprlandBackend.set_mute(m));
                                            }
                                        },
                                    )
                                }),
                        )
                        .child({
                            let opts: Vec<DropdownOption> = if sinks.is_empty() {
                                vec![DropdownOption { label: sink_name.clone(), value: sink_name.clone() }]
                            } else {
                                sinks.iter().map(|d| DropdownOption { label: d.name.clone(), value: d.id.clone() }).collect()
                            };
                            let cur_sink = if sink_name.is_empty() {
                                sinks.first().map(|s| s.name.clone()).unwrap_or_else(|| "Studio Speakers".to_string())
                            } else {
                                sink_name.clone()
                            };
                            dropdown_select(cur_sink, opts, EventHandler::new(move |id: String| {
                                let store = store;
                                store.set(SettingKey::AudioDefaultSink, id.clone());
                                std::thread::spawn(move || { system::HyprlandBackend.set_default_audio_sink(&id); });
                            }))
                        }),
                )
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Input", None::<String>))
                                    .child(
                                        rect()
                                            .margin((6., 0., 10., 0.))
                                            .child(InputLevelMeter),
                                    )
                                    .child({
                                        let opts2: Vec<DropdownOption> = if sources.is_empty() {
                                            vec![DropdownOption { label: source_name.clone(), value: source_name.clone() }]
                                        } else {
                                            sources.iter().map(|d| DropdownOption { label: d.name.clone(), value: d.id.clone() }).collect()
                                        };
                                        let cur_source = if source_name.is_empty() {
                                            sources.first().map(|s| s.name.clone()).unwrap_or_else(|| "Internal Microphone".to_string())
                                        } else {
                                            source_name.clone()
                                        };
                                        dropdown_select(cur_source, opts2, EventHandler::new(move |id: String| {
                                            let store = store;
                                            store.set(SettingKey::Custom("audio.default_source".to_string()), id.clone());
                                            std::thread::spawn(move || {
                                                let _ = std::process::Command::new("wpctl").args(["set-default", &id]).output()
                                                    .or_else(|_| std::process::Command::new("pactl").args(["set-default-source", &id]).output());
                                            });
                                        }))
                                    }),
                            ),
                        rect()
                            .width(Size::flex(1.))
                            .child(
                                tile()
                                    .child(tile_head(None, "Sound effects", None::<String>))
                                    .child({
                                        let alert_cur = alert_sound.read().clone();
                                        dropdown_select(
                                            alert_cur,
                                            vec![
                                                DropdownOption { label: "Tri-tone".to_string(), value: "Tri-tone".to_string() },
                                                DropdownOption { label: "Ping".to_string(), value: "Ping".to_string() },
                                                DropdownOption { label: "Chime".to_string(), value: "Chime".to_string() },
                                            ],
                                            EventHandler::new({
                                                let mut alert = alert_sound;
                                                move |v: String| {
                                                    alert.set(v);
                                                    std::thread::spawn(|| {
                                                        let _ = std::process::Command::new("canberra-gtk-play").args(["-i", "bell"]).output();
                                                    });
                                                }
                                            }),
                                        )
                                    })
                                    .child(
                                        rect()
                                            .margin((14., 0., 0., 0.))
                                            .child(setting_row("Feedback on volume change", None::<String>, true, {
                                                let mut fc = store.feedback_on_change;
                                                pill_switch(*store.feedback_on_change.read(), move |v| fc.set(v))
                                            })),
                                    ),
                            ),
                    ]),
                ),
        )
    }
}

pub fn sound_detail_page(store: SettingsStore) -> SoundDetailPage {
    SoundDetailPage { store }
}

#[derive(PartialEq)]
pub struct FocusDetailPage {
    pub store: SettingsStore,
    pub work_sched: State<bool>,
    pub sleep_sched: State<bool>,
    pub share_devices: State<bool>,
}

impl Component for FocusDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let work_sched = self.work_sched;
        let sleep_sched = self.sleep_sched;
        let share_devices = self.share_devices;
        let show_modal: State<bool> = use_state(|| false);
        let t = use_app_theme();
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
                .child(
                    grid2([
                        rect()
                            .width(Size::flex(1.))
                            .child({
                                tile()
                                    .child(tile_head(
                                        None,
                                        "Schedule",
                                        Some({
                                            let mut sm = show_modal;
                                            rect()
                                                .cursor(CursorIcon::Pointer)
                                                .on_press(move |_| sm.set(true))
                                                .child(ghost_button("Add Schedule", {
                                                    let mut sm2 = show_modal;
                                                    move || sm2.set(true)
                                                }))
                                        }),
                                    ))
                                    .child(setting_row("Work", Some("Weekdays · 09:00–17:00"), false, {
                                        let mut ws = work_sched;
                                        pill_switch(*work_sched.read(), move |v| ws.set(v))
                                    }))
                                    .child(setting_row("Sleep", Some("Daily · 23:00–07:00"), true, {
                                        let mut ss = sleep_sched;
                                        pill_switch(*sleep_sched.read(), move |v| ss.set(v))
                                    }))
                                    .maybe_child((*show_modal.read()).then({
                                        let t = t;
                                        move || {
                                        let mut sm = show_modal;
                                        rect()
                                            .width(Size::fill())
                                            .margin((12., 0., 0., 0.))
                                            .padding(12.)
                                            .background(t.panel_raised)
                                            .border(Border::new().width(1.).fill(t.border))
                                            .corner_radius(12.)
                                            .vertical()
                                            .spacing(10.)
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .main_align(Alignment::SpaceBetween)
                                                    .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text("Add Schedule"))
                                                    .child(
                                                        rect().cursor(CursorIcon::Pointer).on_press(move |_| sm.set(false)).child(label().font_size(12.).color(t.text_dim).text("✕")),
                                                    ),
                                            )
                                            .child(field_label("Name"))
                                            .child(
                                                rect()
                                                    .width(Size::fill())
                                                    .padding((8., 10.))
                                                    .background(t.panel)
                                                    .border(Border::new().width(1.).fill(t.border))
                                                    .corner_radius(8.)
                                                    .child(label().font_size(12.).color(t.text_dim).text("e.g. Evening focus")),
                                            )
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .spacing(8.)
                                                    .content(Content::Flex)
                                                    .child(rect().width(Size::flex(1.)).child(field_label("From · 09:00")))
                                                    .child(rect().width(Size::flex(1.)).child(field_label("To · 17:00"))),
                                            )
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .spacing(8.)
                                                    .child({
                                                        let mut sm2 = show_modal;
                                                        secondary_button("Cancel", move || sm2.set(false))
                                                    })
                                                    .child(primary_button("Add", {
                                                        let mut sm3 = show_modal;
                                                        move || sm3.set(false)
                                                    })),
                                            )
                                            .into_element()
                                        }
                                    }))
                            }),
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
}

pub fn focus_detail_page(
    store: SettingsStore,
    work_sched: State<bool>,
    sleep_sched: State<bool>,
    share_devices: State<bool>,
) -> FocusDetailPage {
    FocusDetailPage { store, work_sched, sleep_sched, share_devices }
}

#[derive(PartialEq)]
pub struct NotificationsDetailPage {
    pub store: SettingsStore,
}

impl Component for NotificationsDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let t = use_app_theme();
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
                                    .child(setting_row("Schedule", None::<String>, true, label().font_size(12.).color(t.text_dim).text("22:00–07:00"))),
                            ),
                    ]),
                ),
        )
    }
}

pub fn notifications_detail_page(store: SettingsStore) -> NotificationsDetailPage {
    NotificationsDetailPage { store }
}

#[derive(PartialEq)]
pub struct GeneralDetailPage {
    pub store: SettingsStore,
}

impl Component for GeneralDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
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
}

pub fn general_detail_page(store: SettingsStore) -> GeneralDetailPage {
    GeneralDetailPage { store }
}

#[derive(Clone, PartialEq)]
struct StorageStats {
    total: String,
    used: String,
    available: String,
    apps: String,
    photos: String,
    docs: String,
    system: String,
    percentages: Vec<f32>,
}

fn query_storage_stats() -> StorageStats {
    let mut total_str = "1 TB".to_string();
    let mut used_str = "0 GB".to_string();
    let mut avail_str = "0 GB".to_string();
    let mut total_bytes: u64 = 1_000_000_000_000;
    let mut used_bytes: u64 = 0;

    if let Ok(out) = std::process::Command::new("df").args(["-B1", "/"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                total_bytes = parts[1].parse().unwrap_or(1);
                used_bytes = parts[2].parse().unwrap_or(0);
                let avail_bytes: u64 = parts[3].parse().unwrap_or(0);
                total_str = format_storage_size(total_bytes);
                used_str = format_storage_size(used_bytes);
                avail_str = format_storage_size(avail_bytes);
                break;
            }
        }
    }

    let db_path = config::finick_root().join("index.db");
    let mut apps_bytes: u64 = 0;
    let mut photos_bytes: u64 = 0;
    let mut docs_bytes: u64 = 0;

    if db_path.exists() {
        if let Ok(out) = std::process::Command::new("sqlite3")
            .args([
                db_path.to_str().unwrap_or(""),
                "SELECT \
                 COALESCE(SUM(CASE WHEN desktop = 1 OR executable = 1 THEN size ELSE 0 END), 0), \
                 COALESCE(SUM(CASE WHEN is_dir = 0 AND (name LIKE '%.png' OR name LIKE '%.jpg' OR name LIKE '%.jpeg' OR name LIKE '%.webp' OR name LIKE '%.gif') THEN size ELSE 0 END), 0), \
                 COALESCE(SUM(CASE WHEN is_dir = 0 AND (name LIKE '%.pdf' OR name LIKE '%.txt' OR name LIKE '%.md' OR name LIKE '%.doc%' OR name LIKE '%.docx') THEN size ELSE 0 END), 0) \
                 FROM files;"
            ])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = s.trim().split('|').collect();
            if parts.len() == 3 {
                apps_bytes = parts[0].trim().parse().unwrap_or(0);
                photos_bytes = parts[1].trim().parse().unwrap_or(0);
                docs_bytes = parts[2].trim().parse().unwrap_or(0);
            }
        }
    }

    if apps_bytes == 0 && photos_bytes == 0 && docs_bytes == 0 && used_bytes > 0 {
        apps_bytes = (used_bytes as f64 * 0.38) as u64;
        photos_bytes = (used_bytes as f64 * 0.21) as u64;
        docs_bytes = (used_bytes as f64 * 0.14) as u64;
    }
    let system_bytes = used_bytes.saturating_sub(apps_bytes + photos_bytes + docs_bytes);

    let tot = if total_bytes > 0 { total_bytes as f32 } else { 1.0 };
    let p_apps = ((apps_bytes as f32 / tot) * 100.0).max(1.0);
    let p_photos = ((photos_bytes as f32 / tot) * 100.0).max(1.0);
    let p_docs = ((docs_bytes as f32 / tot) * 100.0).max(1.0);
    let p_sys = ((system_bytes as f32 / tot) * 100.0).max(1.0);

    StorageStats {
        total: total_str,
        used: used_str,
        available: avail_str,
        apps: format_storage_size(apps_bytes),
        photos: format_storage_size(photos_bytes),
        docs: format_storage_size(docs_bytes),
        system: format_storage_size(system_bytes),
        percentages: vec![p_apps, p_photos, p_docs, p_sys],
    }
}

fn format_storage_size(b: u64) -> String {
    if b >= 1024 * 1024 * 1024 * 1024 {
        format!("{:.1} TB", b as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0))
    } else if b >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if b >= 1024 * 1024 {
        format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
    } else if b >= 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{b} B")
    }
}

#[derive(PartialEq)]
pub struct StorageDetailPage {
    pub store: SettingsStore,
}

impl Component for StorageDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let t = use_app_theme();
        let stats = use_hook(query_storage_stats);
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
                    .child(
                        tile()
                            .child(tile_head(None, format!("{} drive", stats.total), Some(tile_sub(format!("{} used · {} available", stats.used, stats.available)))))
                            .child(multi_segment_bar(
                                vec![
                                    (stats.percentages.get(0).copied().unwrap_or(38.), t.accent),
                                    (stats.percentages.get(1).copied().unwrap_or(21.), t.bg_active),
                                    (stats.percentages.get(2).copied().unwrap_or(14.), t.text_dim),
                                    (stats.percentages.get(3).copied().unwrap_or(27.), t.track),
                                ],
                                true,
                            ))
                            .child(storage_legend(vec![
                                StorageLegendItem { label: "Applications", value: stats.apps.clone(), color: t.accent },
                                StorageLegendItem { label: "Photos", value: stats.photos.clone(), color: t.bg_active },
                                StorageLegendItem { label: "Documents", value: stats.docs.clone(), color: t.text_dim },
                                StorageLegendItem { label: "System", value: stats.system.clone(), color: t.track },
                            ])),
                    )
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
                                        )),
                                ),
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(None, "Categories", None::<String>))
                                        .child(setting_row("Applications", None::<String>, false, label().font_size(12.).color(t.text_dim).text(stats.apps)))
                                        .child(setting_row("Photos", None::<String>, true, label().font_size(12.).color(t.text_dim).text(stats.photos)))
                                        .child(setting_row("Documents", None::<String>, true, label().font_size(12.).color(t.text_dim).text(stats.docs)))
                                        .child(setting_row("System", None::<String>, true, label().font_size(12.).color(t.text_dim).text(stats.system))),
                                ),
                        ]),
                    ),
            )
    }
}

pub fn storage_detail_page(store: SettingsStore) -> StorageDetailPage {
    StorageDetailPage { store }
}

#[derive(PartialEq)]
pub struct BatteryDetailPage {
    pub store: SettingsStore,
}

impl Component for BatteryDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let t = use_app_theme();
        let pwr_lock = store.lock_label(&SettingKey::PowerProfile);
        let is_pwr_locked = pwr_lock.is_some();
        let power = system::HyprlandBackend.get_power_info();
        let has_battery = power.capacity != "Unknown" && power.status != "Unknown";
        let cap = power.capacity.clone();
        let status = power.status.clone();
        let health = power.health_percent.map(|h| format!("{h}%")).unwrap_or_else(|| "91%".to_string());

    if !has_battery {
        return rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(BATTERY, "Battery", "Usage, health and power mode"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
                    .child(
                        tile()
                            .child(tile_head(None, "Power", Some(tile_sub("No battery — plugged in"))))
                            .child(tile_sub("This device is running on AC power. Showing system power usage instead."))
                            .child(
                                rect()
                                    .margin((12., 0., 0., 0.))
                                    .child(mini_bar_chart([42., 58., 35., 62., 48., 55., 60.], ["M", "T", "W", "T", "F", "S", "S"])),
                            )
                            .child(
                                rect()
                                    .margin((10., 0., 0., 0.))
                                    .horizontal()
                                    .spacing(8.)
                                    .child(status_chip("AC Power", false, None))
                                    .child(label().font_size(12.).color(t.text_dim).text(status)),
                            ),
                    )
                    .child(
                        grid2([
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(None, "Power mode", pwr_lock.as_ref().map(|l| lock_badge(l))))
                                        .child(
                                            rect()
                                                .opacity(if is_pwr_locked { 0.45 } else { 1.0 })
                                                .child({
                                                    let store = store;
                                                    let cur_mode = match *store.battery_mode.read() {
                                                        "low" | "power-saver" => "low",
                                                        "high" | "performance" => "high",
                                                        _ => "balanced",
                                                    };
                                                    segmented_control(
                                                        vec![("Low Power", "low"), ("Balanced", "balanced"), ("High Performance", "high")],
                                                        cur_mode,
                                                        move |m| {
                                                            if !is_pwr_locked {
                                                                store.set(SettingKey::PowerProfile, m);
                                                                let profile = match m {
                                                                    "low" => "power-saver",
                                                                    "high" => "performance",
                                                                    _ => "balanced",
                                                                };
                                                                let _ = std::process::Command::new("powerprofilesctl").args(["set", profile]).output();
                                                            }
                                                        },
                                                    )
                                                }),
                                        ),
                                ),
                            rect()
                                .width(Size::flex(1.))
                                .child(
                                    tile()
                                        .child(tile_head(None, "System power", None::<String>))
                                        .child(setting_row("Source", None::<String>, false, label().font_size(12.).color(t.text_dim).text("Mains")))
                                        .child(setting_row("Up time", None::<String>, true, {
                                            let up = system::HyprlandBackend.get_host_info().uptime;
                                            label().font_size(12.).color(t.text_dim).text(up)
                                        })),
                                ),
                        ]),
                    ),
            );
    }

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(BATTERY, "Battery", "Usage, health and power mode"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
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
                                    .child(tile_sub(format!("{cap} · {status}"))),
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
                                            let cur_mode = match *store.battery_mode.read() {
                                                "low" | "power-saver" => "low",
                                                "high" | "performance" => "high",
                                                _ => "balanced",
                                            };
                                            segmented_control(
                                                vec![("Low Power", "low"), ("Balanced", "balanced"), ("High Performance", "high")],
                                                cur_mode,
                                                move |m| {
                                                    if !is_pwr_locked {
                                                        store.set(SettingKey::PowerProfile, m);
                                                        let profile = match m {
                                                            "low" => "power-saver",
                                                            "high" => "performance",
                                                            _ => "balanced",
                                                        };
                                                        let _ = std::process::Command::new("powerprofilesctl").args(["set", profile]).output();
                                                    }
                                                },
                                            )
                                        }),
                                ),
                        ),
                )
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
                                    .child(setting_row("Maximum capacity", None::<String>, false, label().font_size(12.).color(t.text_dim).text(health)))
                                    .child(setting_row("Optimised charging", None::<String>, true, {
                                        let mut oc = store.opt_charging;
                                        pill_switch(*store.opt_charging.read(), move |v| oc.set(v))
                                    })),
                            ),
                    ]),
                ),
        )
    }
}

pub fn battery_detail_page(store: SettingsStore) -> BatteryDetailPage {
    BatteryDetailPage { store }
}

#[derive(PartialEq)]
pub struct AccessibilityDetailPage {
    pub store: SettingsStore,
}

impl Component for AccessibilityDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
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

    {
        let rm = *store.reduce_motion.read();
        let ic = *store.increase_contrast.read();
        let rt = *store.reduce_transparency.read();
        let _ = (rm, ic, rt);
    }

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(ACCESSIBILITY, "Accessibility", "Make your device easier to see, hear and use"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(
                    tile()
                        .child(tile_head(None, "Text size", txt_lock.as_ref().map(|l| lock_badge(l))))
                        .child(
                            rect()
                                .opacity(if is_txt_locked { 0.45 } else { 1.0 })
                                .child({
                                    let store = store;
                                    slider_row(None, *store.text_size.read(), move |v| {
                                        if !is_txt_locked {
                                            store.set(SettingKey::AccessibilityTextSize, v);
                                            if let Some(mut th) = try_consume_context::<State<AppTheme>>() {
                                                let scale = 0.85 + (v as f32 / 100.0) * 0.3;
                                                th.set(th.read().with_font_scale(scale));
                                            }
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
                .child(
                    tile()
                        .child(tile_head(None, "Display", None::<String>))
                        .child(setting_row_locked(
                            "Reduce motion",
                            Some("Disables animations system-wide"),
                            false,
                            motion_lock,
                            {
                                let store = store;
                                pill_switch(*store.reduce_motion.read(), move |v| {
                                    if !is_motion_locked {
                                        store.set(SettingKey::AccessibilityReduceMotion, v);
                                        store.set(SettingKey::Custom("accessibility.reduce_motion_applied".to_string()), v);
                                        let _ = std::process::Command::new("hyprctl")
                                            .args(["keyword", "animations:enabled", if v { "0" } else { "1" }])
                                            .output();
                                    }
                                })
                            },
                        ))
                        .child(setting_row_locked(
                            "Increase contrast",
                            Some("Stronger borders and text"),
                            true,
                            contrast_lock,
                            {
                                let store = store;
                                pill_switch(*store.increase_contrast.read(), move |v| {
                                    if !is_contrast_locked {
                                        store.set(SettingKey::AccessibilityIncreaseContrast, v);
                                        store.set(SettingKey::Custom("accessibility.increase_contrast_applied".to_string()), v);
                                        if let Some(mut th) = try_consume_context::<State<AppTheme>>() {
                                            th.set(th.read().with_contrast(v));
                                        }
                                    }
                                })
                            },
                        ))
                        .child(setting_row_locked(
                            "Reduce transparency",
                            Some("Opaque panels and blur disabled"),
                            true,
                            transp_lock,
                            {
                                let store = store;
                                pill_switch(*store.reduce_transparency.read(), move |v| {
                                    if !is_transp_locked {
                                        store.set(SettingKey::AccessibilityReduceTransparency, v);
                                        store.set(SettingKey::Custom("accessibility.reduce_transparency_applied".to_string()), v);
                                        let _ = std::process::Command::new("hyprctl")
                                            .args(["keyword", "decoration:blur:enabled", if v { "false" } else { "true" }])
                                            .output();
                                    }
                                })
                            },
                        )),
                ),
        )
    }
}

pub fn accessibility_detail_page(store: SettingsStore) -> AccessibilityDetailPage {
    AccessibilityDetailPage { store }
}

#[derive(PartialEq)]
pub struct ScreenTimeDetailPage {
    pub store: SettingsStore,
}

impl Component for ScreenTimeDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
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
}

pub fn screen_time_detail_page(store: SettingsStore) -> ScreenTimeDetailPage {
    ScreenTimeDetailPage { store }
}

#[derive(PartialEq)]
pub struct DesktopDetailPage {
    pub store: SettingsStore,
}

impl Component for DesktopDetailPage {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let _t = use_app_theme();
        let inner_gap: State<f64> = use_state(|| 4.0);
        let layout = *store.window_layout.read();
        let gap = *store.workspace_gap.read();

    rect()
        .width(Size::fill())
        .vertical()
        .child(page_head(LAYOUT_GRID, "Desktop", "Workspaces, window management and appearance"))
        .child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(
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
                        .child(
                            rect()
                                .margin((14., 0., 0., 0.))
                                .child(field_label(format!("Inner gap · {} px", *inner_gap.read() as i32))),
                        )
                        .child({
                            let store = store;
                            let mut ig = inner_gap;
                            slider_row(None, *inner_gap.read(), move |v| {
                                ig.set(v);
                                store.set(SettingKey::Custom("desktop.inner_gap".to_string()), v);
                                let _ = std::process::Command::new("hyprctl")
                                    .args(["keyword", "general:gaps_in", &format!("{}", v as i32)])
                                    .output();
                            })
                        }),
                )
                .child(
                    tile()
                        .child(tile_head(None, "Control Panel", None::<String>))
                        .child(tile_sub("Choose which controls appear in the top-bar panel"))
                        .child(setting_row("Show Wi-Fi", None::<String>, false, {
                            let store = store;
                            let cur = *store.wifi_power.read();
                            pill_switch(cur, move |v| store.set(SettingKey::Custom("topbar.show_wifi".to_string()), v))
                        }))
                        .child(setting_row("Show Bluetooth", None::<String>, true, {
                            let store = store;
                            let cur = *store.bt_power.read();
                            pill_switch(cur, move |v| store.set(SettingKey::Custom("topbar.show_bluetooth".to_string()), v))
                        }))
                        .child(setting_row("Show Wired", None::<String>, true, {
                            let store = store;
                            pill_switch(true, move |v| store.set(SettingKey::Custom("topbar.show_wired".to_string()), v))
                        }))
                        .child(setting_row("Show Sound", None::<String>, true, {
                            let store = store;
                            pill_switch(!*store.is_mute.read(), move |v| store.set(SettingKey::Custom("topbar.show_sound".to_string()), v))
                        }))
                        .child(setting_row("Show Brightness", None::<String>, true, {
                            let store = store;
                            pill_switch(true, move |v| store.set(SettingKey::Custom("topbar.show_brightness".to_string()), v))
                        }))
                        .child(setting_row("Show Battery", None::<String>, true, {
                            let store = store;
                            pill_switch(*store.battery_pct.read() > 0, move |v| store.set(SettingKey::Custom("topbar.show_battery".to_string()), v))
                        }))
                        .child(setting_row("Show Notifications", None::<String>, true, {
                            let store = store;
                            pill_switch(*store.allow_notif.read(), move |v| store.set(SettingKey::Custom("topbar.show_notifications".to_string()), v))
                        }))
                        .child(setting_row("Show Focus / DND", None::<String>, true, {
                            let store = store;
                            let on = *store.focus_mode.read() != "off";
                            pill_switch(on, move |v| store.set(SettingKey::Custom("topbar.show_focus".to_string()), v))
                        })),
                ),
        )
    }
}

pub fn desktop_detail_page(store: SettingsStore) -> DesktopDetailPage {
    DesktopDetailPage { store }
}

#[derive(PartialEq)]
pub struct AboutDetailPage;

impl Component for AboutDetailPage {
    fn render(&self) -> impl IntoElement {
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
}

pub fn about_detail_page() -> AboutDetailPage {
    AboutDetailPage
}
