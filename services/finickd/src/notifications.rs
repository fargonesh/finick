use {
    ipsea::notifications::{
        Notification, NotificationBroadcaster, NotificationEvent,
    },
    std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicU32, Ordering},
            Arc,
        },
    },
    zbus::interface,
};

/// DBus Notification Server implementing `org.freedesktop.Notifications`.
pub struct NotificationServer {
    broadcaster: NotificationBroadcaster,
    next_id: Arc<AtomicU32>,
}

impl NotificationServer {
    pub fn new(broadcaster: NotificationBroadcaster) -> Self {
        Self {
            broadcaster,
            next_id: Arc::new(AtomicU32::new(1)),
        }
    }

    pub fn with_id_counter(broadcaster: NotificationBroadcaster, next_id: Arc<AtomicU32>) -> Self {
        Self {
            broadcaster,
            next_id,
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    /// Send a notification to the desktop.
    async fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        _hints: HashMap<String, zbus::zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            let mut next = self.next_id.fetch_add(1, Ordering::Relaxed);
            if next == 0 {
                next = self.next_id.fetch_add(1, Ordering::Relaxed);
            }
            next
        };

        let notif = Notification {
            id,
            app_name,
            summary,
            body,
            icon: app_icon,
            timeout: expire_timeout,
        };

        self.broadcaster.broadcast(NotificationEvent::Show(notif));
        id
    }

    /// Causes a notification to be forcibly closed and dismissed.
    async fn close_notification(
        &mut self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        id: u32,
    ) {
        self.broadcaster.broadcast(NotificationEvent::Close(id));
        let _ = Self::notification_closed(&emitter, id, 3).await;
    }

    /// Query the capabilities of this notification server.
    async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "actions".to_string(),
            "body".to_string(),
            "body-hyperlinks".to_string(),
            "body-markup".to_string(),
            "icon-static".to_string(),
        ]
    }

    /// Query basic server information (name, vendor, version, spec_version).
    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "finick-notifications".to_string(),
            "Finick".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        )
    }

    /// Emitted when a notification is closed.
    #[zbus(signal)]
    pub async fn notification_closed(
        signal_ctxt: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// Emitted when an action was invoked by the user.
    #[zbus(signal)]
    pub async fn action_invoked(
        signal_ctxt: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

/// Helper function to emit the `NotificationClosed` signal from an active connection.
pub async fn emit_notification_closed(
    conn: &zbus::Connection,
    id: u32,
    reason: u32,
) -> zbus::Result<()> {
    let emitter = zbus::object_server::SignalEmitter::new(conn, "/org/freedesktop/Notifications")?;
    NotificationServer::notification_closed(&emitter, id, reason).await
}

/// Helper function to emit the `ActionInvoked` signal from an active connection.
pub async fn emit_action_invoked(
    conn: &zbus::Connection,
    id: u32,
    action_key: &str,
) -> zbus::Result<()> {
    let emitter = zbus::object_server::SignalEmitter::new(conn, "/org/freedesktop/Notifications")?;
    NotificationServer::action_invoked(&emitter, id, action_key).await
}

/// Starts the DBus notification service on the session bus.
pub async fn start_dbus_notifications_server(
    server: NotificationServer,
) -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", server)?
        .build()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipsea::notifications::{subscribe_channel, start_notification_server};
    use std::time::Duration;

    #[tokio::test]
    async fn test_dbus_methods_and_ipc_broadcast() {
        let broadcaster = NotificationBroadcaster::new();
        let mut server = NotificationServer::new(broadcaster.clone());

        let socket_name = format!("test-dbus-notif-{}", std::process::id());
        let broadcaster_clone = broadcaster.clone();
        let socket_name_clone = socket_name.clone();

        std::thread::spawn(move || {
            let _ = start_notification_server::<fn(u32)>(socket_name_clone, broadcaster_clone, None);
        });

        // Allow IPC socket time to bind
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut rx = subscribe_channel(&socket_name).expect("Failed to subscribe");

        // Wait for subscriber to connect
        for _ in 0..50 {
            if broadcaster.subscriber_count() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(broadcaster.subscriber_count(), 1);

        // 1. Call Notify
        let notif_id = server
            .notify(
                "test-suite".to_string(),
                0,
                "bell".to_string(),
                "Meeting Alert".to_string(),
                "Standup in 5 minutes".to_string(),
                vec![],
                HashMap::new(),
                3000,
            )
            .await;
        assert_eq!(notif_id, 1);

        // Receive on IPC stream
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timed out waiting for Show event")
            .expect("Channel closed");

        match event {
            NotificationEvent::Show(notif) => {
                assert_eq!(notif.id, 1);
                assert_eq!(notif.app_name, "test-suite");
                assert_eq!(notif.summary, "Meeting Alert");
                assert_eq!(notif.body, "Standup in 5 minutes");
                assert_eq!(notif.icon, "bell");
                assert_eq!(notif.timeout, 3000);
            }
            _ => panic!("Expected Show event"),
        }

        // 2. Call CloseNotification
        // We can broadcast Close directly or call broadcaster.broadcast(NotificationEvent::Close(1))
        broadcaster.broadcast(NotificationEvent::Close(notif_id));

        let close_event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timed out waiting for Close event")
            .expect("Channel closed");

        match close_event {
            NotificationEvent::Close(id) => {
                assert_eq!(id, 1);
            }
            _ => panic!("Expected Close event"),
        }

        // 3. Verify GetServerInformation
        let (name, vendor, version, spec) = server.get_server_information().await;
        assert_eq!(name, "finick-notifications");
        assert_eq!(vendor, "Finick");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        assert_eq!(spec, "1.2");

        // 4. Verify GetCapabilities
        let caps = server.get_capabilities().await;
        assert!(caps.contains(&"body".to_string()));
        assert!(caps.contains(&"actions".to_string()));
    }
}
