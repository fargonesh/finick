use {freya::prelude::*, std::process::Command, ui::*};

#[derive(Clone, Debug, PartialEq)]
pub struct DateTimeInfo {
    pub date: String,
    pub time_12: String,
    pub time_24: String,
    pub timezone: String,
    pub universal_time: String,
    pub rtc_time: String,
    pub ntp_synced: bool,
}

pub fn fetch_date_time_info() -> DateTimeInfo {
    let mut info = DateTimeInfo {
        date: "Unknown Date".to_string(),
        time_12: "12:00:00 PM".to_string(),
        time_24: "12:00:00".to_string(),
        timezone: "UTC".to_string(),
        universal_time: "Unknown".to_string(),
        rtc_time: "Unknown".to_string(),
        ntp_synced: false,
    };

    if let Ok(out) = Command::new("date").arg("+%A, %B %d, %Y").output() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            info.date = s;
        }
    }

    if let Ok(out) = Command::new("date").arg("+%H:%M:%S").output() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            info.time_24 = s;
        }
    }

    if let Ok(out) = Command::new("date").arg("+%I:%M:%S %p").output() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            info.time_12 = s;
        }
    }

    if let Ok(output) = Command::new("timedatectl").output() {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("Time zone:") {
                info.timezone = rest.trim().to_string();
            } else if let Some(rest) = l.strip_prefix("Universal time:") {
                info.universal_time = rest.trim().to_string();
            } else if let Some(rest) = l.strip_prefix("RTC time:") {
                info.rtc_time = rest.trim().to_string();
            } else if let Some(rest) = l.strip_prefix("System clock synchronized:") {
                info.ntp_synced = rest.trim().eq_ignore_ascii_case("yes");
            } else if let Some(rest) = l.strip_prefix("NTP service:")
                && !info.ntp_synced
            {
                info.ntp_synced = rest.trim().eq_ignore_ascii_case("active");
            }
        }
    }

    if (info.timezone == "UTC" || info.timezone.is_empty())
        && let Ok(out) = Command::new("date").arg("+%:z (%Z)").output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            info.timezone = s;
        }
    }

    info
}

#[derive(PartialEq)]
pub struct DateTime;

impl Component for DateTime {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let date_time_data = use_state(fetch_date_time_info);
        let is_24_hour = use_state(|| true);
        let auto_sync = use_state(|| true);
        let mut loaded = use_state(|| true);

        if !*loaded.read() {
            loaded.set(true);
            let mut setter = date_time_data;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let info = fetch_date_time_info();
                let _ = tx.send(info);
            });

            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    setter.set(info);
                }
            });
        }

        let info = date_time_data.read().clone();
        let use_24h = *is_24_hour.read();
        let is_auto_sync = *auto_sync.read();
        let display_time = if use_24h { info.time_24.clone() } else { info.time_12.clone() };

        FadeSlideIn::new().child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(page_head(CLOCK_ICON, "Date and time", "Manage system time, time zone, and clock preferences."))
                .child(
                    tile()
                        .child(tile_head(
                            Some(CLOCK_ICON),
                            "Current time",
                            Some(secondary_button("Sync / Refresh", {
                                let mut l = loaded;
                                move || l.set(false)
                            })),
                        ))
                        .child(
                            rect()
                                .margin((0., 0., 12., 0.))
                                .child(
                                    label()
                                        .font_size(36.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(t.text)
                                        .text(display_time),
                                )
                                .child(
                                    label()
                                        .font_size(15.)
                                        .color(t.text_dim)
                                        .margin((4., 0., 0., 0.))
                                        .text(info.date.clone()),
                                ),
                        )
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .margin((4., 0., 0., 0.))
                                .child(
                                    rect()
                                        .width(Size::px(8.))
                                        .height(Size::px(8.))
                                        .corner_radius(4.)
                                        .background(if info.ntp_synced { t.accent_green } else { t.accent_orange })
                                        .margin((0., 8., 0., 0.)),
                                )
                                .child(
                                    label()
                                        .font_size(13.)
                                        .color(if info.ntp_synced { t.accent_green } else { t.text_dim })
                                        .text(if info.ntp_synced {
                                            "Clock synchronized via Network Time Protocol (NTP)".to_string()
                                        } else {
                                            "Clock running locally (Not NTP synchronized)".to_string()
                                        }),
                                ),
                        ),
                )
                .child(
                    tile()
                        .child(tile_head(None, "Time format", None::<String>))
                        .child(setting_row(
                            "24-hour time",
                            Some("Display time in 24-hour format (e.g. 15:30) instead of 12-hour AM/PM"),
                            false,
                            pill_switch(use_24h, {
                                let mut h = is_24_hour;
                                move |v| h.set(v)
                            }),
                        ))
                        .child(setting_row(
                            "Automatic date and time",
                            Some("Use network time synchronization (NTP) to keep system clock accurate"),
                            true,
                            pill_switch(is_auto_sync, {
                                let mut s = auto_sync;
                                let mut l = loaded;
                                move |v| {
                                    s.set(v);
                                    std::thread::spawn(move || {
                                        let _ = Command::new("timedatectl")
                                            .args(["set-ntp", if v { "true" } else { "false" }])
                                            .output();
                                    });
                                    l.set(false);
                                }
                            }),
                        )),
                )
                .child(
                    tile()
                        .child(tile_head(None, "Time zone", None::<String>))
                        .child(setting_row(
                            "Time zone",
                            None::<String>,
                            false,
                            label().font_size(14.).color(t.text_dim).text(info.timezone.clone()),
                        ))
                        .child(setting_row(
                            "Universal time",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(info.universal_time.clone()),
                        ))
                        .child(setting_row(
                            "Hardware clock",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(info.rtc_time.clone()),
                        )),
                ),
        )
    }
}
