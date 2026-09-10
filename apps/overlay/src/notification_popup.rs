use {
    freya::prelude::*,
    ipsea::notifications::Notification,
    std::path::PathBuf,
    ui::*,
};

/// Helper to render an icon for a notification.
/// Handles SVGs, local image files, named icon aliases, and defaults to the notification bell icon.
pub fn render_notification_icon(icon_str: &str, size: f32, color: Color) -> Element {
    let trimmed = icon_str.trim();

    if trimmed.starts_with("<svg") {
        let bytes = Bytes::copy_from_slice(trimmed.as_bytes());
        return SvgViewer::new(bytes)
            .width(Size::px(size))
            .height(Size::px(size))
            .color(color)
            .into_element();
    }

    if trimmed.ends_with(".png")
        || trimmed.ends_with(".jpg")
        || trimmed.ends_with(".jpeg")
        || trimmed.ends_with(".webp")
    {
        return ImageViewer::new(PathBuf::from(trimmed))
            .width(Size::px(size))
            .height(Size::px(size))
            .into_element();
    }

    if trimmed.ends_with(".svg") {
        return SvgViewer::new(PathBuf::from(trimmed))
            .width(Size::px(size))
            .height(Size::px(size))
            .color(color)
            .into_element();
    }

    let lower = trimmed.to_lowercase();
    let svg_str = match lower.as_str() {
        "sound" | "audio" | "volume" => SOUND,
        "network" | "wifi" | "wireless" => WIFI,
        "wired" | "ethernet" => WIRED,
        "bluetooth" => BLUETOOTH,
        "battery" | "power" => BATTERY,
        "display" | "screen" | "monitor" => DISPLAY,
        "folder" => FOLDER,
        "file" | "document" => FILE,
        "download" => DOWNLOAD,
        "lock" | "security" => LOCK,
        "info" | "information" | "dialog-information" | "about" => ABOUT,
        "moon" | "night" | "focus" => FOCUS,
        _ => NOTIFICATIONS,
    };

    icon(svg_str, size, color).into_element()
}

/// Freya component displaying a notification popup with title, body, and icon.
/// Automatically dismisses after `timeout` milliseconds if `timeout > 0` (or default 5000ms if <= 0).
#[derive(PartialEq, Clone)]
pub struct NotificationPopup {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub icon: String,
    pub app_name: String,
    pub timeout: i32,
    pub width: Size,
    pub on_close: Option<EventHandler<u32>>,
}

impl std::fmt::Debug for NotificationPopup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationPopup")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("body", &self.body)
            .field("icon", &self.icon)
            .field("app_name", &self.app_name)
            .field("timeout", &self.timeout)
            .field("width", &self.width)
            .finish()
    }
}

impl NotificationPopup {
    pub fn new(title: impl Into<String>, body: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: 0,
            title: title.into(),
            body: body.into(),
            icon: icon.into(),
            app_name: String::new(),
            timeout: 5000,
            width: Size::px(360.),
            on_close: None,
        }
    }

    pub fn from_notification(notif: &Notification) -> Self {
        Self {
            id: notif.id,
            title: notif.summary.clone(),
            body: notif.body.clone(),
            icon: notif.icon.clone(),
            app_name: notif.app_name.clone(),
            timeout: notif.timeout,
            width: Size::px(360.),
            on_close: None,
        }
    }

    pub fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    pub fn with_timeout(mut self, timeout: i32) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn on_close(mut self, handler: impl Into<EventHandler<u32>>) -> Self {
        self.on_close = Some(handler.into());
        self
    }

    pub fn on_dismiss(mut self, handler: impl Into<EventHandler<u32>>) -> Self {
        self.on_close = Some(handler.into());
        self
    }
}

impl Component for NotificationPopup {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let is_hovered = use_state(|| false);
        let close_hovered = use_state(|| false);

        let id = self.id;
        let timeout = self.timeout;
        let on_close = self.on_close.clone();

        // Auto-dismiss hook called at the top level of render
        use_hook(move || {
            let effective_timeout = if timeout > 0 {
                timeout as u64
            } else {
                5000
            };

            if let Some(on_close) = on_close {
                spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(effective_timeout)).await;
                    on_close.call(id);
                });
            }
        });

        let card_bg = if *is_hovered.read() {
            t.panel_raised
        } else {
            t.panel
        };

        let close_bg = if *close_hovered.read() {
            t.border
        } else {
            Color::TRANSPARENT
        };

        let mut card_h1 = is_hovered;
        let mut card_h2 = is_hovered;
        let mut close_h1 = close_hovered;
        let mut close_h2 = close_hovered;

        let display_app = if !self.app_name.is_empty() {
            self.app_name.clone()
        } else {
            "Notification".to_string()
        };

        let title_text = self.title.clone();
        let body_text = self.body.clone();
        let icon_str = self.icon.clone();
        let on_close_handler = self.on_close.clone();

        rect()
            .width(self.width.clone())
            .vertical()
            .spacing(8.)
            .padding((10., 14.))
            .corner_radius(16.)
            .background(card_bg)
            .border(Border::new().width(1.).fill(t.border))
            .content(Content::Flex)
            .on_pointer_enter(move |_| card_h1.set(true))
            .on_pointer_leave(move |_| card_h2.set(false))
            // Header Row: App Name tag + Spacer + Close Button
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .content(Content::Flex)
                    .child(
                        label()
                            .font_size(11.)
                            .font_weight(FontWeight::SEMI_BOLD)
                            .color(t.accent)
                            .text(display_app),
                    )
                    .child(rect().width(Size::flex(1.)))
                    .child(
                        rect()
                            .width(Size::px(22.))
                            .height(Size::px(22.))
                            .corner_radius(999.)
                            .background(close_bg)
                            .center()
                            .cursor(CursorIcon::Pointer)
                            .on_pointer_enter(move |_| close_h1.set(true))
                            .on_pointer_leave(move |_| close_h2.set(false))
                            .on_press(move |_| {
                                if let Some(ref h) = on_close_handler {
                                    h.call(id);
                                }
                            })
                            .child(
                                label()
                                    .font_size(11.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_dim)
                                    .text("✕"),
                            ),
                    ),
            )
            // Content Row: Icon Container + Text Info
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Start)
                    .spacing(12.)
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(38.))
                            .height(Size::px(38.))
                            .corner_radius(10.)
                            .background(t.panel_raised)
                            .border(Border::new().width(1.).fill(t.border))
                            .center()
                            .child(render_notification_icon(&icon_str, 20., t.text)),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .vertical()
                            .spacing(3.)
                            .content(Content::Flex)
                            .child(
                                label()
                                    .font_size(13.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text)
                                    .text(title_text),
                            )
                            .child(
                                label()
                                    .font_size(12.)
                                    .color(t.text_dim)
                                    .text(body_text),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_popup_new() {
        let popup = NotificationPopup::new("Test Title", "Test Body", "sound");
        assert_eq!(popup.title, "Test Title");
        assert_eq!(popup.body, "Test Body");
        assert_eq!(popup.icon, "sound");
        assert_eq!(popup.timeout, 5000);
        assert_eq!(popup.id, 0);
        assert_eq!(popup.width, Size::px(360.));
    }

    #[test]
    fn test_notification_popup_from_notification() {
        let notif = Notification {
            id: 42,
            app_name: "Spotify".to_string(),
            summary: "Now Playing".to_string(),
            body: "Song - Artist".to_string(),
            icon: "sound".to_string(),
            timeout: 3000,
        };

        let popup = NotificationPopup::from_notification(&notif)
            .with_width(Size::fill());

        assert_eq!(popup.id, 42);
        assert_eq!(popup.title, "Now Playing");
        assert_eq!(popup.body, "Song - Artist");
        assert_eq!(popup.app_name, "Spotify");
        assert_eq!(popup.icon, "sound");
        assert_eq!(popup.timeout, 3000);
        assert_eq!(popup.width, Size::fill());
    }

    #[test]
    fn test_notification_popup_builder() {
        let popup = NotificationPopup::new("Alert", "Something happened", "lock")
            .with_id(99)
            .with_app_name("System")
            .with_timeout(10000);

        assert_eq!(popup.id, 99);
        assert_eq!(popup.app_name, "System");
        assert_eq!(popup.timeout, 10000);
    }
}
