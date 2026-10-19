use {
    crate::state::{
        clear_notifications, dismiss_notification, fetch_notifications_blocking, group_notifications, is_app_muted,
        load_muted_apps, load_reduced_motion_blocking, set_app_muted, subscribe_notifications_live,
        subscribe_reduced_motion,
    },
    freya::prelude::*,
    std::collections::HashSet,
    ui::*,
};

pub fn notification_panel_app() -> Element {
    let theme_state = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let notifications = use_state(Vec::<crate::state::Notification>::new);
    let muted = use_state(HashSet::<String>::new);
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
        let mut rm = reduced;
        spawn(async move {
            let apps = tokio::task::spawn_blocking(load_muted_apps).await.unwrap_or_default();
            m.set_if_modified(apps);
            let r = tokio::task::spawn_blocking(load_reduced_motion_blocking).await.unwrap_or(false);
            rm.set_if_modified(r);
        });
        subscribe_reduced_motion(reduced);
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::spawn_panel_focus_watcher_guarded(
            &["overlay-notifications", "overlay", "topbar"],
            "notifications-panel",
            dismiss,
            move || crate::state::is_notifications_window_current(my_wid),
        );
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::clear_close_flag(&crate::state::CLOSE_NOTIFICATIONS);
        crate::state::spawn_close_flag_watcher_guarded(
            &crate::state::CLOSE_NOTIFICATIONS,
            dismiss,
            move || crate::state::is_notifications_window_current(my_wid),
        );
    });
    let notifs = notifications.read().clone();
    let muted_set = muted.read().clone();
    let is_reduced = *reduced.read();
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

    let mut body: Vec<Element> = Vec::new();

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
            body.push(StaggerRow::new(gi, groups.len(), dr.clone()).reduced(is_reduced).child(group).into_element());
        }
    }

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
