use freya::prelude::*;
use ui::*;
use std::process::Command;

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
                && !info.ntp_synced {
                    info.ntp_synced = rest.trim().eq_ignore_ascii_case("active");
                }
        }
    }

    if (info.timezone == "UTC" || info.timezone.is_empty())
        && let Ok(out) = Command::new("date").arg("+%:z (%Z)").output() {
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
        let display_time = if use_24h {
            info.time_24.clone()
        } else {
            info.time_12.clone()
        };

        rect()
            .width(Size::fill())
            .child(page_header(
                "Date & Time",
                "Manage system time, time zone, and clock preferences.",
            ))
            // Clock & Date Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .child(
                                label()
                                    .font_size(13.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_secondary)
                                    .text("CURRENT TIME & DATE"),
                            )
                            .child(secondary_button("Sync / Refresh", {
                                let mut l = loaded;
                                move || l.set(false)
                            })),
                    )
                    .child(
                        rect()
                            .margin((0., 0., 12., 0.))
                            .child(
                                label()
                                    .font_size(36.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_primary)
                                    .text(display_time),
                            )
                            .child(
                                label()
                                    .font_size(15.)
                                    .color(t.text_secondary)
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
                                    .background(if info.ntp_synced {
                                        t.accent_green
                                    } else {
                                        t.accent_orange
                                    })
                                    .margin((0., 8., 0., 0.)),
                            )
                            .child(
                                label()
                                    .font_size(13.)
                                    .color(if info.ntp_synced {
                                        t.accent_green
                                    } else {
                                        t.text_secondary
                                    })
                                    .text(if info.ntp_synced {
                                        "Clock synchronized via Network Time Protocol (NTP)".to_string()
                                    } else {
                                        "Clock running locally (Not NTP synchronized)".to_string()
                                    }),
                            ),
                    ),
            )
            // Time Format Settings Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_secondary)
                            .margin((0., 0., 14., 0.))
                            .text("TIME FORMAT & SYNCHRONIZATION"),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("24-Hour Time"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((2., 0., 0., 0.))
                                            .text("Display time in 24-hour format (e.g. 15:30) instead of 12-hour AM/PM"),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(use_24h)
                                    .on_toggle({
                                        let mut h = is_24_hour;
                                        move |_| {
                                            let curr = *h.read();
                                            h.set(!curr);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Automatic Date & Time"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((2., 0., 0., 0.))
                                            .text("Use network time synchronization (NTP) to keep system clock accurate"),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(is_auto_sync)
                                    .on_toggle({
                                        let mut s = auto_sync;
                                        let mut l = loaded;
                                        move |_| {
                                            let next = !*s.read();
                                            s.set(next);
                                            std::thread::spawn(move || {
                                                let _ = Command::new("timedatectl")
                                                    .args(["set-ntp", if next { "true" } else { "false" }])
                                                    .output();
                                            });
                                            l.set(false);
                                        }
                                    }),
                            ),
                    ),
            )
            // Time Zone & Hardware Details Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_secondary)
                            .margin((0., 0., 14., 0.))
                            .text("TIME ZONE & DETAILS"),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((10., 14.))
                            .margin((0., 0., 6., 0.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text_primary)
                                    .text("Time Zone"),
                            )
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_secondary)
                                    .text(info.timezone.clone()),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((10., 14.))
                            .margin((0., 0., 6., 0.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text_primary)
                                    .text("Universal Time (UTC)"),
                            )
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_secondary)
                                    .text(info.universal_time.clone()),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((10., 14.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text_primary)
                                    .text("RTC (Hardware Clock)"),
                            )
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_secondary)
                                    .text(info.rtc_time.clone()),
                            ),
                    ),
            )
    }
}
