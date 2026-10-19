use {
    crate::state::{
        DndSchedule, clear_notifications, dismiss_notification, dnd_schedule_active, fetch_notifications_blocking,
        group_notifications, is_app_muted, load_dnd_schedule, load_muted_apps, load_reduced_motion_blocking, now_hm_local,
        query_media_status, save_dnd_schedule, set_app_muted, subscribe_notifications_live, subscribe_reduced_motion,
    },
    freya::prelude::*,
    ipsea::settings::SettingKey,
    std::collections::HashSet,
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
                .child(label().font_size(10.).font_weight(FontWeight::BOLD).color(t.text_dim).text(name))
                .into_element(),
        );
    }
    rows.push(rect().width(Size::fill()).horizontal().spacing(2.).content(Content::Flex).children(header).into_element());
    let mut cells: Vec<Element> = Vec::new();
    for _ in 0..start_col {
        cells.push(rect().width(Size::flex(1.)).height(Size::px(22.)).into_element());
    }
    for day in 1..=days_in_month {
        let is_today = day as u32 == today.day();
        cells.push(
            rect()
                .width(Size::flex(1.))
                .height(Size::px(22.))
                .corner_radius(999.)
                .background(if is_today { t.accent } else { Color::TRANSPARENT })
                .center()
                .child(
                    label()
                        .font_size(11.)
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
                    .spacing(2.)
                    .content(Content::Flex)
                    .children(std::mem::take(&mut cells))
                    .into_element(),
            );
        }
    }
    if !cells.is_empty() {
        while cells.len() < 7 {
            cells.push(rect().width(Size::flex(1.)).height(Size::px(22.)).into_element());
        }
        rows.push(rect().width(Size::fill()).horizontal().spacing(2.).content(Content::Flex).children(cells).into_element());
    }
    rect().width(Size::fill()).vertical().spacing(2.).content(Content::Flex).children(rows).into_element()
}

pub fn notification_panel_app() -> Element {
    let theme_state = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let notifications = use_state(Vec::<crate::state::Notification>::new);
    let muted = use_state(HashSet::<String>::new);
    let dnd_sched = use_state(DndSchedule::default);
    let dnd_manual = use_state(|| false);
    let media = use_state(crate::state::MediaStatus::unavailable);
    let now_date = use_state(|| chrono::Local::now().format("%A, %-d %B").to_string());
    let reduced = use_state(|| false);
    let mut dismiss = use_state(|| false);
    let wid = Platform::window_id();
    use_hook(move || {
        let mut ns = notifications;
        spawn(async move {
            let initial = tokio::task::spawn_blocking(fetch_notifications_blocking).await.unwrap_or_default();
            ns.set_if_modified(initial);
        });
        subscribe_notifications_live(notifications);
        let _ = theme_state;
    });
    use_hook(move || {
        let mut m = muted;
        let mut ds = dnd_sched;
        let mut dm = dnd_manual;
        let mut md = media;
        let mut nd = now_date;
        let mut rm = reduced;
        spawn(async move {
            let apps = tokio::task::spawn_blocking(load_muted_apps).await.unwrap_or_default();
            m.set_if_modified(apps);
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
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                nd_poll.set_if_modified(chrono::Local::now().format("%A, %-d %B").to_string());
            }
        });
    });
    use_hook(|| {
        crate::state::spawn_panel_focus_watcher(
            &["overlay-notifications", "overlay", "topbar"],
            "notifications-panel",
            dismiss,
        );
    });
    use_hook(move || {
        crate::state::clear_close_flag(&crate::state::CLOSE_NOTIFICATIONS);
        crate::state::spawn_close_flag_watcher(&crate::state::CLOSE_NOTIFICATIONS, dismiss);
    });
    let notifs = notifications.read().clone();
    let muted_set = muted.read().clone();
    let sched = dnd_sched.read().clone();
    let sched_active = dnd_schedule_active(&sched, &now_hm_local());
    let dnd_on = *dnd_manual.read();
    let is_reduced = *reduced.read();
    let date_str = now_date.read().clone();
    let media_state = media.read().clone();
    let groups = group_notifications(&notifs);
    let notif_count = notifs.len();
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
                .child(icon(NOTIFICATIONS, 16., t.text))
                .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(format!(
                    "Notifications{}",
                    if notif_count > 0 { format!(" · {notif_count}") } else { String::new() }
                ))),
        )
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .content(Content::Flex)
                .child(ghost_button("Clear all", {
                    let mut ns = notifications;
                    move || {
                        let ids: Vec<u32> = ns.read().iter().map(|n| n.id).collect();
                        for id in ids {
                            dismiss_notification(id);
                        }
                        ns.set(vec![]);
                        clear_notifications();
                    }
                }))
                .child(small_close_button(move || dismiss.set(true))),
        );

    let calendar_card = tile()
        .child(tile_head(Some(CLOCK_ICON), "Today", None::<Element>))
        .child(setting_row(
            date_str,
            Some(chrono::Local::now().format("%H:%M").to_string()),
            false,
            status_chip(
                if dnd_on || sched_active { "DND" } else { "Active" },
                dnd_on || sched_active,
                None::<EventHandler<()>>,
            ),
        ))
        .child(calendar_month_grid());

    let sched_label = format!("{}–{}", sched.start, sched.end);
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
        card = card.child(segmented_control(
            vec![
                ("Off", "off".to_string()),
                ("22–07", "22-07".to_string()),
                ("23–07", "23-07".to_string()),
                ("21–06", "21-06".to_string()),
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
        if !media_state.available {
            card = card.child(tile_sub("Unavailable: playerctl not found"));
        } else if media_state.title.is_empty() && media_state.artist.is_empty() {
            card = card.child(tile_sub(format!(
                "No track playing{}",
                if media_state.player.is_empty() { String::new() } else { format!(" · {}", media_state.player) }
            )));
        } else {
            let line = if media_state.artist.is_empty() {
                media_state.title.clone()
            } else {
                format!("{} — {}", media_state.title, media_state.artist)
            };
            card = card.child(setting_row(
                line,
                Some(if media_state.player.is_empty() {
                    media_state.status.clone()
                } else {
                    format!("{} · {}", media_state.player, media_state.status)
                }),
                false,
                status_chip(media_state.status.clone(), media_state.status == "Playing", None::<EventHandler<()>>),
            ));
            card = card.child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .spacing(8.)
                    .content(Content::Flex)
                    .child(rect().width(Size::flex(1.)).child(ghost_button("Prev", || {
                        std::thread::spawn(|| {
                            crate::state::media_control("previous");
                        });
                    })))
                    .child(rect().width(Size::flex(1.)).child(ghost_button("Play/Pause", || {
                        std::thread::spawn(|| {
                            crate::state::media_control("play-pause");
                        });
                    })))
                    .child(rect().width(Size::flex(1.)).child(ghost_button("Next", || {
                        std::thread::spawn(|| {
                            crate::state::media_control("next");
                        });
                    }))),
            );
        }
        card
    };

    let mut body: Vec<Element> = Vec::new();
    body.push(StaggerRow::new(0, 3, dr.clone()).reduced(is_reduced).child(calendar_card).into_element());
    body.push(StaggerRow::new(1, 3, dr.clone()).reduced(is_reduced).child(dnd_card).into_element());
    body.push(StaggerRow::new(2, 3, dr.clone()).reduced(is_reduced).child(media_card).into_element());

    if notifs.is_empty() {
        body.push(empty_state(NOTIFICATIONS, "No notifications", "You're all caught up").into_element());
    } else {
        for (gi, (app, items)) in groups.iter().enumerate() {
            let app_name = app.clone();
            let app_count = items.len();
            let app_muted = is_app_muted(&muted_set, &app_name);
            let mut group = tile().child(tile_head(
                None,
                format!("{app_name} · {app_count}"),
                Some({
                    let mut m = muted;
                    let name = app_name.clone();
                    let cur = app_muted;
                    pill_switch(!cur, move |v| {
                        let mut set = m.read().clone();
                        set_app_muted(&mut set, &name, !v);
                        m.set(set);
                    })
                }),
            ));
            if app_muted {
                group = group.child(setting_row(
                    format!("{app_count} hidden"),
                    Some("Muted for this app"),
                    false,
                    status_chip("Muted", true, None::<EventHandler<()>>),
                ));
            } else {
                let mut col = rect().width(Size::fill()).vertical().spacing(8.).padding(4.).content(Content::Flex);
                for n in items.iter().cloned() {
                    let mut ns = notifications;
                    let id = n.id;
                    let card = crate::notification_popup::NotificationPopup::from_notification(&n)
                        .with_auto_dismiss(false)
                        .with_width(Size::fill())
                        .on_close(move |id| {
                            let mut list = ns.read().clone();
                            if let Some(pos) = list.iter().position(|item| item.id == id) {
                                list.remove(pos);
                                ns.set(list);
                            }
                            dismiss_notification(id);
                        });
                    col = col.child(FadeSlideIn::new().reduced(is_reduced).child(card));
                    let _ = id;
                }
                group = group.child(col);
            }
            group = group.child(setting_row(
                format!("Clear {app_name}"),
                None::<String>,
                true,
                ghost_button("Clear", {
                    let mut ns = notifications;
                    let name = app_name.clone();
                    move || {
                        let ids: Vec<u32> = ns.read().iter().filter(|n| n.app_name == name).map(|n| n.id).collect();
                        for id in ids {
                            dismiss_notification(id);
                        }
                        let mut list = ns.read().clone();
                        list.retain(|n| n.app_name != name);
                        ns.set(list);
                    }
                }),
            ));
            body.push(StaggerRow::new(gi + 3, groups.len() + 3, dr.clone()).reduced(is_reduced).child(group).into_element());
        }
    }

    let list = rect().width(Size::fill()).vertical().spacing(10.).padding(6.).content(Content::Flex).children(body);
    let inner = tile().spacing(12.).content(Content::Flex).child(header).child(
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
                .child(inner),
        )
        .into_element()
}

pub fn notification_panel_window_config() -> WindowConfig {
    WindowConfig::new(notification_panel_app)
        .with_title("notifications-panel")
        .with_app_id("overlay-notifications")
        .with_size(400., 620.)
        .with_min_size(380., 360.)
        .with_max_size(400., 760.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|ctx, _wid| {
            if let Some(panel) = ctx.global_contexts.try_get_context::<crate::NotificationPanelWindowId>() {
                if let Ok(mut g) = panel.0.lock() {
                    *g = None;
                }
            }
            CloseDecision::Close
        })
}
