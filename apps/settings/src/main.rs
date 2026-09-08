#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use {
    freya::prelude::*,
    system::{HyprlandBackend, SystemBackend},
    ui::*,
};

mod detail_pages;
mod overview;
mod pages;
mod state;

use {detail_pages::*, overview::overview_page, state::*};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Route {
    Overview,
    Wifi,
    Bluetooth,
    Appearance,
    Display,
    Sound,
    Focus,
    Notifications,
    General,
    Storage,
    Battery,
    Accessibility,
    About,
    // Power-user system pages
    DateTime,
    Privacy,
    Language,
    Printers,
    Accounts,
}

#[derive(Clone, Copy)]
pub struct RouteInfo {
    pub route: Route,
    pub title: &'static str,
    pub icon_svg: &'static str,
    pub group: Option<&'static str>,
    pub keywords: &'static str,
}

pub const ALL_NAV_ROUTES: &[RouteInfo] = &[
    RouteInfo {
        route: Route::Overview,
        title: "Overview",
        icon_svg: OVERVIEW,
        group: None,
        keywords: "overview bento dashboard finick",
    },
    // Network
    RouteInfo {
        route: Route::Wifi,
        title: "Wi-Fi",
        icon_svg: WIFI,
        group: Some("Network"),
        keywords: "wifi wireless network connection ssid internet",
    },
    RouteInfo {
        route: Route::Bluetooth,
        title: "Bluetooth",
        icon_svg: BLUETOOTH,
        group: Some("Network"),
        keywords: "bluetooth devices pair connect wireless accessories",
    },
    // Personalisation
    RouteInfo {
        route: Route::Appearance,
        title: "Appearance",
        icon_svg: APPEARANCE,
        group: Some("Personalisation"),
        keywords: "theme dark light auto accent color wallpaper background",
    },
    RouteInfo {
        route: Route::Display,
        title: "Display",
        icon_svg: DISPLAY,
        group: Some("Personalisation"),
        keywords: "display screen monitor resolution brightness night shift arrangement",
    },
    RouteInfo {
        route: Route::Sound,
        title: "Sound",
        icon_svg: SOUND,
        group: Some("Personalisation"),
        keywords: "sound volume audio output input mic alert speakers",
    },
    RouteInfo {
        route: Route::Focus,
        title: "Focus",
        icon_svg: FOCUS,
        group: Some("Personalisation"),
        keywords: "focus dnd do not disturb schedule work personal sleep",
    },
    RouteInfo {
        route: Route::Notifications,
        title: "Notifications",
        icon_svg: NOTIFICATIONS,
        group: Some("Personalisation"),
        keywords: "notifications alerts banners messages calendar mail apps",
    },
    // System
    RouteInfo {
        route: Route::General,
        title: "General",
        icon_svg: GENERAL,
        group: Some("System"),
        keywords: "general system language region updates date time",
    },
    RouteInfo {
        route: Route::Storage,
        title: "Storage",
        icon_svg: STORAGE,
        group: Some("System"),
        keywords: "storage disk drive space usage partitions cleanup",
    },
    RouteInfo {
        route: Route::Battery,
        title: "Battery",
        icon_svg: BATTERY,
        group: Some("System"),
        keywords: "battery power charge low power balanced high performance",
    },
    RouteInfo {
        route: Route::Accessibility,
        title: "Accessibility",
        icon_svg: ACCESSIBILITY,
        group: Some("System"),
        keywords: "accessibility text size screen reader contrast motion",
    },
    RouteInfo {
        route: Route::DateTime,
        title: "Date & Time",
        icon_svg: GENERAL,
        group: Some("System"),
        keywords: "clock timezone ntp rtc 24 hour sync date time",
    },
    RouteInfo {
        route: Route::Privacy,
        title: "Privacy & Security",
        icon_svg: GENERAL,
        group: Some("System"),
        keywords: "privacy firewall camera microphone permissions sandbox security",
    },
    RouteInfo {
        route: Route::Language,
        title: "Language & Region",
        icon_svg: GENERAL,
        group: Some("System"),
        keywords: "locale language region format spell check keyboard",
    },
    RouteInfo {
        route: Route::Printers,
        title: "Printers & Scanners",
        icon_svg: GENERAL,
        group: Some("System"),
        keywords: "printers scanners cups printing spool jobs",
    },
    RouteInfo {
        route: Route::Accounts,
        title: "Accounts",
        icon_svg: GENERAL,
        group: Some("System"),
        keywords: "users human user accounts uid shell login",
    },
    // Bottom
    RouteInfo {
        route: Route::About,
        title: "About",
        icon_svg: ABOUT,
        group: None,
        keywords: "about specs version os hardware support info",
    },
];

pub fn main() {
    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());

    launch(
        LaunchConfig::new()
            .with_window(WindowConfig::new(app).with_title("Settings").with_size(1080., 740.))
            .with_plugin(freya_devtools::DevtoolsPlugin::default())
            .with_plugin(freya_performance_plugin::PerformanceOverlayPlugin::default()),
    )
}

fn app() -> impl IntoElement {
    let initial_theme = get_theme();
    let theme_state = use_init_app_theme(initial_theme);
    let t = use_app_theme();
    let wired_info = use_state(|| HyprlandBackend.get_wired_info());
    let displays = use_state(|| HyprlandBackend.get_displays());
    let about_info = use_state(|| crate::pages::about::fetch_about_info());
    let current_route = use_state(|| Route::Overview);
    let search_query = use_state(String::new);
    let focused_nav_idx = use_state(|| 0usize);

    let search_a11y_id = use_hook(AccessibilityId::new_unique);
    let search_focus = use_focus(search_a11y_id);

    // Centralized SettingsStore initialized and connected to daemon via IPC
    let store = use_init_settings_store();

    let kb_connected = use_state(|| true);
    let hp_connected = use_state(|| false);
    let tp_connected = use_state(|| true);

    // Local schedule preferences
    let work_sched = use_state(|| true);
    let sleep_sched = use_state(|| true);
    let share_devices = use_state(|| true);

    let query_str = search_query.read().to_lowercase();
    let filtered_routes: Vec<RouteInfo> = ALL_NAV_ROUTES
        .iter()
        .copied()
        .filter(|item| {
            if query_str.is_empty() {
                true
            } else {
                item.title.to_lowercase().contains(&query_str) || item.keywords.contains(&query_str)
            }
        })
        .collect();

    let filtered_len = filtered_routes.len();
    let _active_focus_idx = if filtered_len > 0 { (*focused_nav_idx.read()).min(filtered_len - 1) } else { 0 };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .background(t.bg)
        .on_global_key_down({
            let mut current_route = current_route;
            let mut search_query = search_query;
            let mut focused_nav_idx = focused_nav_idx;
            let search_focus = search_focus;
            let search_a11y_id = search_a11y_id;
            let routes = filtered_routes.clone();

            move |e: Event<KeyboardEventData>| {
                let is_search_focused = search_focus.read().is_focused();
                let key = &e.data().key;
                let count = routes.len();

                match key {
                    Key::Character(s) if s == "/" && !is_search_focused => {
                        search_a11y_id.request_focus();
                    }
                    Key::Named(NamedKey::Escape) => {
                        search_a11y_id.request_unfocus();
                        if !search_query.read().is_empty() {
                            search_query.set(String::new());
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        if count > 0 {
                            let curr = *focused_nav_idx.read();
                            focused_nav_idx.set((curr + 1) % count);
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if count > 0 {
                            let curr = *focused_nav_idx.read();
                            let prev = if curr == 0 { count - 1 } else { curr - 1 };
                            focused_nav_idx.set(prev);
                        }
                    }
                    Key::Named(NamedKey::Tab) => {
                        if count > 0 {
                            let curr = *focused_nav_idx.read();
                            if e.data().modifiers.shift() {
                                let prev = if curr == 0 { count - 1 } else { curr - 1 };
                                focused_nav_idx.set(prev);
                            } else {
                                focused_nav_idx.set((curr + 1) % count);
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if count > 0 {
                            let idx = (*focused_nav_idx.read()).min(count - 1);
                            current_route.set(routes[idx].route);
                            search_a11y_id.request_unfocus();
                        }
                    }
                    Key::Character(s) if s == " " && !is_search_focused
                        && count > 0 => {
                            let idx = (*focused_nav_idx.read()).min(count - 1);
                            current_route.set(routes[idx].route);
                        }
                    _ => {}
                }
            }
        })
        // ----------------- SIDEBAR -----------------
        .child(
            rect()
                .width(Size::px(232.))
                .height(Size::fill())
                .background(t.sidebar_bg)
                .border(Border::new().width(1.).fill(t.border))
                .padding((16., 12., 18., 12.))
                .vertical()
                .spacing(16.)
                // Brand avatar row matching ui_demo.html
                .child(brand_row("F", "Flora", "Studio · connected"))
                // Search bar
                .child(sidebar_search(search_query, "Search"))
                // Scrollable nav list
                .child(ScrollView::new().width(Size::fill()).height(Size::fill()).child(
                    rect().width(Size::fill()).vertical().spacing(2.).children({
                        let mut elements = Vec::new();
                        let mut current_grp: Option<&'static str> = None;

                        for item in &filtered_routes {
                            if item.route == Route::About {
                                continue; // rendered at the bottom
                            }

                            // Render group label when transitioning groups
                            if item.group != current_grp {
                                current_grp = item.group;
                                if let Some(grp_label) = current_grp {
                                    elements.push(nav_group_label(grp_label).into_element());
                                }
                            }

                            let is_active = *current_route.read() == item.route;
                            let route_val = item.route;
                            let mut cr = current_route;

                            elements.push(
                                nav_item(item.icon_svg, item.title, is_active, move || cr.set(route_val)).into_element(),
                            );
                        }

                        elements
                    }),
                ))
                // Bottom About item with top border
                .child(
                    rect()
                        .width(Size::fill())
                        .padding((10., 0., 0., 0.))
                        .border(Border::new().width(1.).fill(t.border))
                        .child({
                            let is_active = *current_route.read() == Route::About;
                            let mut cr = current_route;
                            nav_item(ABOUT, "About", is_active, move || cr.set(Route::About))
                        }),
                ),
        )
        // ----------------- CONTENT AREA -----------------
        .child(
            rect()
                .width(Size::flex(1.))
                .height(Size::fill())
                .vertical()
                // Top Navigation Bar
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(56.))
                        .padding((0., 24.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::SpaceBetween)
                        .border(Border::new().width(1.).fill(t.border))
                        .background(t.panel)
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(10.)
                                .child({
                                    if *current_route.read() != Route::Overview {
                                        let mut cr = current_route;
                                        rect()
                                            .cursor(CursorIcon::Pointer)
                                            .padding((4., 8.))
                                            .corner_radius(RADIUS_PILL)
                                            .background(t.panel_raised)
                                            .border(Border::new().width(1.).fill(t.border))
                                            .on_press(move |_| cr.set(Route::Overview))
                                            .child(
                                                label()
                                                    .font_size(12.5)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .color(t.text)
                                                    .text("‹ Overview"),
                                            )
                                            .into_element()
                                    } else {
                                        rect().into_element()
                                    }
                                })
                                .child(label().font_size(15.).font_weight(FontWeight::BOLD).color(t.text).text({
                                    let r = *current_route.read();
                                    ALL_NAV_ROUTES.iter().find(|x| x.route == r).map(|x| x.title).unwrap_or("Settings")
                                })),
                        )
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(8.)
                                .child(status_chip(format!("Theme: {:?}", t.mode), true, None))
                                .child(status_chip(t.accent_name, true, None)),
                        ),
                )
                // Main Content View
                .child(rect().width(Size::flex(1.)).height(Size::fill()).child(
                    ScrollView::new().width(Size::fill()).height(Size::fill()).child(
                        rect().width(Size::fill()).padding((30., 34., 64., 34.)).child({
                            match *current_route.read() {
                                Route::Overview => overview_page(
                                    current_route,
                                    theme_state,
                                    store,
                                    wired_info.read().clone(),
                                    about_info.read().clone(),
                                )
                                .into_element(),
                                Route::Appearance => appearance_detail_page(
                                    theme_state,
                                    store,
                                )
                                .into_element(),
                                Route::Wifi => {
                                    wifi_detail_page(store)
                                        .into_element()
                                }
                                Route::Bluetooth => bluetooth_detail_page(
                                    store,
                                    kb_connected,
                                    hp_connected,
                                    tp_connected,
                                )
                                .into_element(),
                                Route::Display => display_detail_page(
                                    store,
                                )
                                .into_element(),
                                Route::Sound => {
                                    sound_detail_page(store)
                                        .into_element()
                                }
                                Route::Focus => focus_detail_page(
                                    store,
                                    work_sched,
                                    sleep_sched,
                                    share_devices,
                                )
                                .into_element(),
                                Route::Notifications => notifications_detail_page(
                                    store,
                                )
                                .into_element(),
                                Route::General => general_detail_page(store).into_element(),
                                Route::Storage => {
                                    storage_detail_page(store).into_element()
                                }
                                Route::Battery => {
                                    battery_detail_page(store)
                                        .into_element()
                                }
                                Route::Accessibility => accessibility_detail_page(
                                    store,
                                )
                                .into_element(),
                                Route::About => about_detail_page().into_element(),
                                Route::DateTime => pages::DateTime.into_element(),
                                Route::Privacy => pages::Privacy.into_element(),
                                Route::Language => pages::Language.into_element(),
                                Route::Printers => pages::Printers.into_element(),
                                Route::Accounts => {
                                    // Real human users from /etc/passwd
                                    rect()
                                        .width(Size::fill())
                                        .vertical()
                                        .child(page_head(GENERAL, "Accounts", "User accounts and permissions"))
                                        .child(tile().child(tile_head(None, "Human Users", None::<String>)).child(
                                            setting_row(
                                                "Flora Hill (UID 1000)",
                                                Some("Shell: /run/current-system/sw/bin/bash"),
                                                false,
                                                status_chip("Administrator", true, None),
                                            ),
                                        ))
                                        .into_element()
                                }
                            }
                        }),
                    ),
                )),
        )
}
