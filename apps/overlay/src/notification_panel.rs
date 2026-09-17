use crate::state::{clear_notifications, dismiss_notification, fetch_notifications_blocking, subscribe_notifications_live};
use freya::prelude::*;
use ui::*;

pub fn notification_panel_app() -> Element {
    let theme_state = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let notifications = use_state(Vec::<crate::state::Notification>::new);
    use_hook(move || {
        let mut ns = notifications;
        spawn(async move {
            let initial = tokio::task::spawn_blocking(fetch_notifications_blocking).await.unwrap_or_default();
            ns.set_if_modified(initial);
        });
        subscribe_notifications_live(notifications);
        let _ = theme_state;
    });
    use_hook(|| {
        let panel_id = Platform::window_id();
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let should_close = tokio::task::spawn_blocking(|| {
                    let out = std::process::Command::new("hyprctl").args(["activewindow", "-j"]).output();
                    if let Ok(o) = out {
                        let s = String::from_utf8_lossy(&o.stdout).to_string();
                        if s.trim().is_empty() || s.contains("\"class\": \"\"") {
                            return false;
                        }
                        if s.contains("\"class\": \"overlay-notifications\"") || s.contains("\"class\": \"overlay\"") || s.contains("\"class\": \"topbar\"") {
                            return false;
                        }
                        return true;
                    }
                    false
                })
                .await
                .unwrap_or(false);
                if should_close {
                    Platform::get().close_window(panel_id);
                    break;
                }
            }
        });
    });
    let notifs = notifications.read().clone();
    let notif_count = notifs.len();
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
                .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(format!("Notifications{}", if notif_count > 0 { format!(" · {notif_count}") } else { String::new() }))),
        )
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .content(Content::Flex)
                .child(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut ns = notifications;
                            move |_| {
                                let ids: Vec<u32> = ns.read().iter().map(|n| n.id).collect();
                                for id in ids {
                                    dismiss_notification(id);
                                }
                                ns.set(vec![]);
                                clear_notifications();
                            }
                        })
                        .child(ghost_button("Clear", {
                            let mut ns = notifications;
                            move || {
                                let ids: Vec<u32> = ns.read().iter().map(|n| n.id).collect();
                                for id in ids {
                                    dismiss_notification(id);
                                }
                                ns.set(vec![]);
                            }
                        })),
                )
                .child(
                    rect()
                        .width(Size::px(28.))
                        .height(Size::px(28.))
                        .corner_radius(999.)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border))
                        .center()
                        .cursor(CursorIcon::Pointer)
                        .on_press(|_| {
                            if let Some(ctx) = GlobalContexts::get().try_get_context::<crate::NotificationPanelWindowId>() {
                                if let Ok(mut g) = ctx.0.lock() {
                                    *g = None;
                                }
                            }
                            Platform::get().close_window(Platform::window_id())
                        })
                        .child(label().font_size(13.).color(t.text_dim).text("✕")),
                ),
        );
    let list = if notifs.is_empty() {
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .center()
            .child(label().font_size(13.).color(t.text_dim).text("No notifications"))
            .into_element()
    } else {
        let mut col = rect().width(Size::fill()).vertical().spacing(10.).content(Content::Flex);
        for n in notifs.iter().rev().cloned() {
            let mut ns = notifications;
            let notif = n.clone();
            col = col.child(
                crate::notification_popup::NotificationPopup::from_notification(&notif)
                    .with_auto_dismiss(false)
                    .with_width(Size::fill())
                    .on_close(move |id| {
                        let mut list = ns.read().clone();
                        if let Some(pos) = list.iter().position(|item| item.id == id) {
                            list.remove(pos);
                            ns.set(list);
                        }
                        dismiss_notification(id);
                    }),
            );
        }
        ScrollView::new().width(Size::fill()).height(Size::fill()).show_scrollbar(true).child(col).into_element()
    };
    let inner = rect()
        .width(Size::fill())
        .height(Size::fill())
        .vertical()
        .spacing(12.)
        .padding(12.)
        .background(t.panel)
        .corner_radius(20.)
        .border(Border::new().width(1.).fill(t.border))
        .content(Content::Flex)
        .child(header)
        .child(rect().width(Size::fill()).height(Size::fill()).content(Content::Flex).child(list));
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(6.)
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(inner)
        .into_element()
}

pub fn notification_panel_window_config() -> WindowConfig {
    WindowConfig::new(notification_panel_app)
        .with_title("notifications-panel")
        .with_app_id("overlay-notifications")
        .with_size(400., 520.)
        .with_min_size(380., 360.)
        .with_max_size(400., 700.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|_ctx, _wid| {
            if let Some(panel) = GlobalContexts::get().try_get_context::<crate::NotificationPanelWindowId>() {
                if let Ok(mut g) = panel.0.lock() {
                    *g = None;
                }
            }
            CloseDecision::Close
        })
}
