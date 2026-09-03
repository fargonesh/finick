use freya::prelude::*;
use ui::*;
use system::{HyprlandBackend, SystemBackend};

/// Responsive overview page for the settings app.
pub fn overview_page(
    current_route: State<crate::Route>,
    theme_state: State<AppTheme>,
    wifi_power: State<bool>,
    bt_power: State<bool>,
    focus_mode: State<&'static str>,
    brightness: State<f64>,
    volume: State<f64>,
    battery_pct: State<u8>,
    battery_mode: State<&'static str>,
    wallpaper_idx: State<usize>,
    notif_messages: State<bool>,
    notif_calendar: State<bool>,
    notif_mail: State<bool>,
) -> impl IntoElement {
    let t = use_app_theme();

    responsive_view(1000.0, move |compact| {

    let focus_desc = match *focus_mode.read() {
        "work" => "Notifications are silenced except from your team.",
        "personal" => "Only calls and messages from favourites come through.",
        "sleep" => "Everything is silenced until morning.",
        _ => "Notifications are delivered as usual.",
    };

    let appearance_card = {
        let mut route = current_route.clone();
        let mut theme = theme_state.clone();
        let mut wallpaper = wallpaper_idx.clone();

        tile()
            .child(tile_head(
                Some(APPEARANCE),
                "Appearance",
                Some(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(segmented_control(
                            vec![("Light", ThemeMode::Light), ("Dark", ThemeMode::Dark), ("Auto", ThemeMode::Auto)],
                            t.mode,
                            move |mode| {
                                let new_t = theme.read().with_mode(mode);
                                theme.set(new_t);
                                set_theme(&new_t);
                            },
                        ))
                        .child(
                            rect()
                                .cursor(CursorIcon::Pointer)
                                .padding((4., 6.))
                                .on_press(move |_| route.set(crate::Route::Appearance))
                                .child(label().font_size(12.).color(t.text_dim).text("→")),
                        ),
                ),
            ))
            .child(field_label("Accent colour"))
            .child({
                let mut theme = theme_state.clone();
                swatch_picker(t.accent_name, false, move |acc| {
                    let new_t = theme.read().with_accent(acc);
                    theme.set(new_t);
                    set_theme(&new_t);
                })
            })
            .child(rect().margin((14., 0., 0., 0.)).child(field_label("Desktop background")))
            .child(wallpaper_picker(4, *wallpaper_idx.read(), false, move |idx| wallpaper.set(idx)))
    };

    let wifi_card = {
        let mut route = current_route.clone();
        let mut power = wifi_power.clone();

        tile()
            .child(tile_head(
                Some(WIFI),
                "Wi-Fi",
                Some(pill_switch(*wifi_power.read(), move |v| power.set(v))),
            ))
            .child(setting_row(
                if *wifi_power.read() { "Homebase 5G" } else { "Not connected" },
                Some(if *wifi_power.read() { "Auto-join enabled" } else { "Wi-Fi is off" }),
                false,
                rect()
                    .cursor(CursorIcon::Pointer)
                    .padding((4., 6.))
                    .on_press(move |_| route.set(crate::Route::Wifi))
                    .child(label().font_size(12.).color(t.text_dim).text("→")),
            ))
    };

    let bluetooth_card = {
        let mut power = bt_power.clone();

        tile()
            .child(tile_head(
                Some(BLUETOOTH),
                "Bluetooth",
                Some(pill_switch(*bt_power.read(), move |v| power.set(v))),
            ))
            .child(setting_row("Wireless Keyboard", None::<String>, false, status_chip("Connected", true, None)))
            .child(setting_row("Headphones", None::<String>, true, status_chip("Not connected", false, None)))
    };

    let focus_card = {
        let mut route = current_route.clone();

        tile()
            .child(tile_head(
                Some(FOCUS),
                "Focus",
                Some(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 6.))
                        .on_press(move |_| route.set(crate::Route::Focus))
                        .child(label().font_size(12.).color(t.text_dim).text("→")),
                ),
            ))
            .child(
                rect()
                    .vertical()
                    .spacing(8.)
                    .child({ let mut fm = focus_mode.clone(); focus_pill("Off", *focus_mode.read() == "off", move || fm.set("off")) })
                    .child({ let mut fm = focus_mode.clone(); focus_pill("Work", *focus_mode.read() == "work", move || fm.set("work")) })
                    .child({ let mut fm = focus_mode.clone(); focus_pill("Personal", *focus_mode.read() == "personal", move || fm.set("personal")) })
                    .child({ let mut fm = focus_mode.clone(); focus_pill("Sleep", *focus_mode.read() == "sleep", move || fm.set("sleep")) }),
            )
            .child(rect().margin((12., 0., 0., 0.)).child(label().font_size(12.).color(t.text_dim).text(focus_desc)))
    };

    let display_card = {
        let mut route = current_route.clone();
        let mut value = brightness.clone();

        tile()
            .child(tile_head(
                Some(DISPLAY),
                "Display",
                Some(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 6.))
                        .on_press(move |_| route.set(crate::Route::Display))
                        .child(label().font_size(12.).color(t.text_dim).text("→")),
                ),
            ))
            .child(slider_row(Some(SUN), *brightness.read(), move |val| value.set(val)))
    };

    let sound_card = {
        let mut route = current_route.clone();
        let mut value = volume.clone();

        tile()
            .child(tile_head(
                Some(SOUND),
                "Sound",
                Some(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 6.))
                        .on_press(move |_| route.set(crate::Route::Sound))
                        .child(label().font_size(12.).color(t.text_dim).text("→")),
                ),
            ))
            .child(slider_row(Some(SOUND), *volume.read(), move |val| {
                value.set(val);
                std::thread::spawn(move || {
                    let _ = HyprlandBackend.set_volume(val as i32);
                });
            }))
    };

    let storage_card = {
        let mut route = current_route.clone();

        tile()
            .child(tile_head(
                Some(STORAGE),
                "Storage",
                Some(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(tile_sub("612 GB of 1 TB used"))
                        .child(
                            rect()
                                .cursor(CursorIcon::Pointer)
                                .padding((4., 6.))
                                .on_press(move |_| route.set(crate::Route::Storage))
                                .child(label().font_size(12.).color(t.text_dim).text("→")),
                        ),
                ),
            ))
            .child(multi_segment_bar(vec![(38., t.accent), (21., t.bg_active), (14., t.text_dim), (27., t.track)], false))
            .child(storage_legend(vec![
                StorageLegendItem { label: "Applications", value: "231 GB".into(), color: t.accent },
                StorageLegendItem { label: "Photos", value: "128 GB".into(), color: t.bg_active },
                StorageLegendItem { label: "Documents", value: "84 GB".into(), color: t.text_dim },
                StorageLegendItem { label: "System", value: "169 GB".into(), color: t.track },
            ]))
    };

    let notifications_card = {
        let mut route = current_route.clone();
        let mut messages = notif_messages.clone();
        let mut calendar = notif_calendar.clone();
        let mut mail = notif_mail.clone();

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
            .child(setting_row("Messages", None::<String>, false, pill_switch(*notif_messages.read(), move |v| messages.set(v))))
            .child(setting_row("Calendar", None::<String>, true, pill_switch(*notif_calendar.read(), move |v| calendar.set(v))))
            .child(setting_row("Mail", None::<String>, true, pill_switch(*notif_mail.read(), move |v| mail.set(v))))
    };

    let battery_card = {
        let mut route = current_route.clone();
        let mut mode = battery_mode.clone();

        tile()
            .child(tile_head(
                Some(BATTERY),
                "Battery",
                Some(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((4., 6.))
                        .on_press(move |_| route.set(crate::Route::Battery))
                        .child(label().font_size(12.).color(t.text_dim).text("→")),
                ),
            ))
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(12.)
                    .child(battery_ring(*battery_pct.read(), false))
                    .child(battery_modes(
                        vec![("Low Power", "low"), ("Balanced", "balanced"), ("High Performance", "high")],
                        *battery_mode.read(),
                        false,
                        move |m| mode.set(m),
                    )),
            )
    };

    let about_card = {
        let mut route = current_route.clone();

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
            .child(setting_row("Version", Some("14.6.1 (Finick OS)"), false, label().font_size(12.).color(t.text_dim).text("Linux 6.x")))
            .child(setting_row("Software", None::<String>, true, status_chip("Up to date", false, None)))
    };

    if compact {
        rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(appearance_card)
            .child(wifi_card)
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
                    .child(rect().width(Size::flex(1.)).child(wifi_card)),
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