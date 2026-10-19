use {
    crate::state::{
        DndSchedule, dnd_schedule_active, load_dnd_schedule, load_reduced_motion_blocking, now_hm_local,
        query_media_status, save_dnd_schedule, subscribe_reduced_motion,
    },
    freya::prelude::*,
    ipsea::settings::SettingKey,
    ui::*,
};

fn preset_key(sched: &DndSchedule) -> String {
    if !sched.enabled {
        return "off".to_string();
    }
    match (sched.start.as_str(), sched.end.as_str()) {
        ("22:00", "07:00") => "22-07".to_string(),
        ("23:00", "07:00") => "23-07".to_string(),
        ("21:00", "06:00") => "21-06".to_string(),
        _ => "22-07".to_string(),
    }
}

fn apply_preset(key: &str) -> DndSchedule {
    match key {
        "23-07" => DndSchedule { enabled: true, start: "23:00".to_string(), end: "07:00".to_string() },
        "21-06" => DndSchedule { enabled: true, start: "21:00".to_string(), end: "06:00".to_string() },
        "22-07" => DndSchedule::default(),
        _ => DndSchedule { enabled: false, start: "22:00".to_string(), end: "07:00".to_string() },
    }
}

fn calendar_month_grid() -> Element {
    use chrono::Datelike;
    let t = use_app_theme();
    let today = chrono::Local::now().date_naive();
    let first = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let start_col = first.weekday().num_days_from_monday() as usize;
    let days_in_month =
        (1..=31).filter(|d| chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), *d).is_some()).count();
    let mut rows: Vec<Element> = Vec::new();
    let mut header: Vec<Element> = Vec::new();
    for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
        header.push(
            rect()
                .width(Size::flex(1.))
                .center()
                .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_dim).text(name))
                .into_element(),
        );
    }
    rows.push(rect().width(Size::fill()).horizontal().spacing(4.).content(Content::Flex).children(header).into_element());
    let mut cells: Vec<Element> = Vec::new();
    for _ in 0..start_col {
        cells.push(rect().width(Size::flex(1.)).height(Size::px(28.)).into_element());
    }
    for day in 1..=days_in_month {
        let is_today = day as u32 == today.day();
        cells.push(
            rect()
                .width(Size::flex(1.))
                .height(Size::px(28.))
                .corner_radius(999.)
                .background(if is_today { t.accent } else { Color::TRANSPARENT })
                .center()
                .child(
                    label()
                        .font_size(12.)
                        .font_weight(if is_today { FontWeight::BOLD } else { FontWeight::MEDIUM })
                        .color(if is_today { Color::WHITE } else { t.text })
                        .text(day.to_string()),
                )
                .into_element(),
        );
        if cells.len() == 7 {
            rows.push(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .spacing(4.)
                    .content(Content::Flex)
                    .children(std::mem::take(&mut cells))
                    .into_element(),
            );
        }
    }
    if !cells.is_empty() {
        while cells.len() < 7 {
            cells.push(rect().width(Size::flex(1.)).height(Size::px(28.)).into_element());
        }
        rows.push(rect().width(Size::fill()).horizontal().spacing(4.).content(Content::Flex).children(cells).into_element());
    }
    rect().width(Size::fill()).vertical().spacing(4.).content(Content::Flex).children(rows).into_element()
}

pub fn date_panel_app() -> Element {
    let theme_state = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let dnd_sched = use_state(DndSchedule::default);
    let dnd_manual = use_state(|| false);
    let media = use_state(crate::state::MediaStatus::unavailable);
    let now_date = use_state(|| chrono::Local::now().format("%A, %-d %B").to_string());
    let now_time = use_state(|| chrono::Local::now().format("%H:%M").to_string());
    let reduced = use_state(|| false);
    let mut dismiss = use_state(|| false);
    let wid = Platform::window_id();
    use_hook(move || {
        let _ = theme_state;
    });
    use_hook(move || {
        let mut ds = dnd_sched;
        let mut dm = dnd_manual;
        let mut md = media;
        let mut nd = now_date;
        let mut nt = now_time;
        let mut rm = reduced;
        spawn(async move {
            let sched = tokio::task::spawn_blocking(load_dnd_schedule).await.unwrap_or_default();
            ds.set_if_modified(sched);
            let manual = tokio::task::spawn_blocking(|| {
                ipsea::settings::get_all_settings(crate::state::socket())
                    .unwrap_or_default()
                    .iter()
                    .find(|e| e.key == SettingKey::DoNotDisturb)
                    .map(|e| e.value.as_str().map(|s| s != "off").unwrap_or(false) || e.value.as_bool().unwrap_or(false))
                    .unwrap_or(false)
            })
            .await
            .unwrap_or(false);
            dm.set_if_modified(manual);
            let st = tokio::task::spawn_blocking(query_media_status).await.unwrap_or_else(|_| query_media_status());
            md.set(st);
            nd.set(chrono::Local::now().format("%A, %-d %B").to_string());
            nt.set(chrono::Local::now().format("%H:%M").to_string());
            let r = tokio::task::spawn_blocking(load_reduced_motion_blocking).await.unwrap_or(false);
            rm.set_if_modified(r);
        });
        subscribe_reduced_motion(reduced);
        let mut md_poll = media;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                let st = tokio::task::spawn_blocking(query_media_status).await.unwrap_or_else(|_| query_media_status());
                md_poll.set_if_modified(st);
            }
        });
        let mut nd_poll = now_date;
        let mut nt_poll = now_time;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                nd_poll.set_if_modified(chrono::Local::now().format("%A, %-d %B").to_string());
                nt_poll.set_if_modified(chrono::Local::now().format("%H:%M").to_string());
            }
        });
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::spawn_panel_focus_watcher_guarded(
            &["overlay-calendar", "overlay", "topbar"],
            "calendar-panel",
            dismiss,
            move || crate::state::is_calendar_window_current(my_wid),
        );
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::clear_close_flag(&crate::state::CLOSE_CALENDAR);
        crate::state::spawn_close_flag_watcher_guarded(
            &crate::state::CLOSE_CALENDAR,
            dismiss,
            move || crate::state::is_calendar_window_current(my_wid),
        );
    });
    let sched = dnd_sched.read().clone();
    let sched_active = dnd_schedule_active(&sched, &now_hm_local());
    let dnd_on = *dnd_manual.read();
    let is_reduced = *reduced.read();
    let date_str = now_date.read().clone();
    let time_str = now_time.read().clone();
    let media_state = media.read().clone();
    let dr = dismiss.into_readable();

    let header = rect()
        .width(Size::fill())
        .horizontal()
        .main_align(Alignment::SpaceBetween)
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .content(Content::Flex)
                .child(icon(CLOCK_ICON, 16., t.text))
                .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(date_str.clone())),
        )
        .child(small_close_button(move || dismiss.set(true)));

    let calendar_card = tile()
        .child(tile_head(Some(CLOCK_ICON), "Today", None::<Element>))
        .child(setting_row(
            date_str,
            Some(time_str),
            false,
            status_chip(
                if dnd_on || sched_active { "DND" } else { "Active" },
                dnd_on || sched_active,
                None::<EventHandler<()>>,
            ),
        ))
        .child(calendar_month_grid());

    let sched_label = format!("{}-{}", sched.start, sched.end);
    let sched_meta = if !sched.enabled {
        "Off".to_string()
    } else if sched_active || dnd_on {
        format!("{sched_label} · silencing toasts")
    } else {
        format!("{sched_label} · armed")
    };
    let dnd_card = {
        let mut card = tile().child(tile_head(
            Some(FOCUS),
            "Do Not Disturb",
            Some({
                let mut dm = dnd_manual;
                pill_switch(dnd_on, move |v| {
                    dm.set_if_modified(v);
                    std::thread::spawn(move || {
                        crate::state::apply(
                            SettingKey::DoNotDisturb,
                            if v { "on".to_string().into() } else { "off".to_string().into() },
                        );
                    });
                })
            }),
        ));
        card = card.child(setting_row("Schedule", Some(sched_meta), false, {
            let mut ds = dnd_sched;
            pill_switch(sched.enabled, move |v| {
                let mut s = ds.read().clone();
                s.enabled = v;
                if v && crate::state::parse_hm(&s.start).is_none() {
                    s.start = "22:00".to_string();
                    s.end = "07:00".to_string();
                }
                save_dnd_schedule(&s);
                ds.set(s);
            })
        }));
        let current_preset = preset_key(&sched);
        card = card.child(segmented_control_full(
            vec![
                ("Off", "off".to_string()),
                ("22-07", "22-07".to_string()),
                ("23-07", "23-07".to_string()),
                ("21-06", "21-06".to_string()),
            ],
            current_preset,
            EventHandler::new({
                let mut ds = dnd_sched;
                move |v: String| {
                    let s = apply_preset(&v);
                    save_dnd_schedule(&s);
                    ds.set(s);
                }
            }),
        ));
        card
    };

    let media_card = {
        let mut card = tile().child(tile_head(Some(MUSIC), "Now playing", None::<Element>));
        let pretty_player = {
            let p = media_state.player.trim();
            if p.is_empty() {
                String::new()
            } else {
                let mut c = p.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            }
        };
        if !media_state.available {
            card = card.child(tile_sub("Unavailable: playerctl not found"));
        } else if media_state.title.is_empty() && media_state.artist.is_empty() {
            card = card.child(tile_sub(format!(
                "No track playing{}",
                if pretty_player.is_empty() { String::new() } else { format!(" · {}", pretty_player) }
            )));
        } else {
            let title_display = if media_state.title.is_empty() {
                media_state.artist.clone()
            } else {
                media_state.title.clone()
            };
            let mut meta_parts: Vec<String> = Vec::new();
            if !media_state.title.is_empty() && !media_state.artist.is_empty() {
                meta_parts.push(media_state.artist.clone());
            }
            if !pretty_player.is_empty() {
                meta_parts.push(pretty_player);
            }
            let meta_text = if meta_parts.is_empty() { None } else { Some(meta_parts.join(" · ")) };
            let status_trimmed = media_state.status.trim();
            let status_label = if status_trimmed.is_empty() { "Stopped".to_string() } else { status_trimmed.to_string() };
            let is_playing = status_label.eq_ignore_ascii_case("playing");
            card = card.child(setting_row(
                title_display,
                meta_text,
                false,
                status_chip(status_label, is_playing, None::<EventHandler<()>>),
            ));
            let play_icon = if is_playing { PAUSE } else { PLAY };
            card = card.child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .spacing(8.)
                    .content(Content::Flex)
                    .child(
                        rect().width(Size::flex(1.)).content(Content::Flex).child(ghost_icon_button_full(SKIP_BACK, || {
                            std::thread::spawn(|| {
                                crate::state::media_control("previous");
                            });
                        })),
                    )
                    .child(
                        rect().width(Size::flex(1.)).content(Content::Flex).child(ghost_icon_button_full(play_icon, || {
                            std::thread::spawn(|| {
                                crate::state::media_control("play-pause");
                            });
                        })),
                    )
                    .child(
                        rect().width(Size::flex(1.)).content(Content::Flex).child(ghost_icon_button_full(SKIP_FORWARD, || {
                            std::thread::spawn(|| {
                                crate::state::media_control("next");
                            });
                        })),
                    ),
            );
        }
        card
    };

    let mut body: Vec<Element> = Vec::new();
    body.push(StaggerRow::new(0, 3, dr.clone()).reduced(is_reduced).child(calendar_card).into_element());
    body.push(StaggerRow::new(1, 3, dr.clone()).reduced(is_reduced).child(dnd_card).into_element());
    body.push(StaggerRow::new(2, 3, dr.clone()).reduced(is_reduced).child(media_card).into_element());

    let list = rect().width(Size::fill()).vertical().spacing(10.).padding(6.).content(Content::Flex).children(body);
    let panel_inner = rect()
        .width(Size::fill())
        .vertical()
        .spacing(12.)
        .padding(10.)
        .background(t.panel)
        .content(Content::Flex)
        .corner_radius(20.)
        .border(Border::new().width(1.).fill(t.border))
        .child(header)
        .child(
            rect()
                .width(Size::fill())
                .height(Size::auto())
                .content(Content::Flex)
                .child(ScrollView::new().width(Size::fill()).height(Size::px(560.)).show_scrollbar(true).child(list)),
        );
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(6.)
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(
            PanelShell::new(dr)
                .reduced(is_reduced)
                .on_dismiss(EventHandler::new(move |()| Platform::get().close_window(wid)))
                .child(panel_inner),
        )
        .into_element()
}

pub fn date_panel_window_config() -> WindowConfig {
    WindowConfig::new(date_panel_app)
        .with_title("calendar-panel")
        .with_app_id("overlay-calendar")
        .with_size(400., 620.)
        .with_min_size(380., 360.)
        .with_max_size(400., 760.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|ctx, _wid| {
            if let Some(panel) = ctx.global_contexts.try_get_context::<crate::CalendarPanelWindowId>() {
                if let Ok(mut g) = panel.0.lock() {
                    *g = None;
                }
            }
            CloseDecision::Close
        })
}
