use {
    crate::{resolve_socket_path, send_command, start_server},
    serde::{Deserialize, Serialize},
    std::{
        fmt::Display,
        path::PathBuf,
        sync::{mpsc::Sender, Arc, Mutex},
    },
};

/// Default socket name for finick notifications IPC.
pub const NOTIFICATIONS_SOCKET_NAME: &str = "finick-notifications.sock";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: String,
    pub timeout: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationEvent {
    Show(Notification),
    Close(u32),
}

/// Request sent by clients to the notifications broadcast server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationRequest {
    /// Subscribe to live notification events.
    Subscribe,
    /// Close / dismiss a notification with the given id.
    Close(u32),
    /// Request all currently active notifications.
    GetAll,
}

/// Thread-safe broadcaster that manages connected IPC subscribers and emits events.
#[derive(Clone, Default)]
pub struct NotificationBroadcaster {
    subscribers: Arc<Mutex<Vec<Sender<NotificationEvent>>>>,
    active: Arc<Mutex<Vec<Notification>>>,
}

impl NotificationBroadcaster {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            active: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a new subscriber channel and send all currently active notifications.
    pub fn add_subscriber(&self, sender: Sender<NotificationEvent>) {
        if let Ok(active) = self.active.lock() {
            for notif in active.iter() {
                let _ = sender.send(NotificationEvent::Show(notif.clone()));
            }
        }
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(sender);
    }

    /// Broadcast an event to all connected subscribers, automatically pruning disconnected channels.
    pub fn broadcast(&self, event: NotificationEvent) {
        if let Ok(mut active) = self.active.lock() {
            match &event {
                NotificationEvent::Show(notif) => {
                    if let Some(pos) = active.iter().position(|n| n.id == notif.id) {
                        active[pos] = notif.clone();
                    } else {
                        active.push(notif.clone());
                    }
                }
                NotificationEvent::Close(id) => {
                    active.retain(|n| n.id != *id);
                }
            }
        }
        let mut subs = self.subscribers.lock().unwrap();
        subs.retain(|sender| sender.send(event.clone()).is_ok());
    }

    /// Total active subscribers count.
    pub fn subscriber_count(&self) -> usize {
        let subs = self.subscribers.lock().unwrap();
        subs.len()
    }

    /// Get all currently active notifications.
    pub fn get_active(&self) -> Vec<Notification> {
        self.active.lock().map(|a| a.clone()).unwrap_or_default()
    }
}

/// Starts the notifications IPC server on `socket_path` using the provided broadcaster.
/// Optionally takes an on_close callback when a client requests `NotificationRequest::Close(id)`.
pub fn start_notification_server<F>(
    socket_path: impl Into<PathBuf> + Display,
    broadcaster: NotificationBroadcaster,
    on_close: Option<F>,
) -> std::io::Result<()>
where
    F: Fn(u32) + Send + Sync + Clone + 'static,
{
    start_server(
        socket_path,
        move |req: NotificationRequest, sender: Sender<NotificationEvent>| match req {
            NotificationRequest::Subscribe => {
                broadcaster.add_subscriber(sender);
            }
            NotificationRequest::Close(id) => {
                if let Some(ref cb) = on_close {
                    cb(id);
                }
                broadcaster.broadcast(NotificationEvent::Close(id));
            }
            NotificationRequest::GetAll => {
                for n in broadcaster.get_active() {
                    let _ = sender.send(NotificationEvent::Show(n));
                }
            }
        },
    )
}

/// Helper function to fetch all currently active notifications synchronously.
pub fn get_active_notifications(
    socket: impl Into<PathBuf> + Display,
) -> std::io::Result<Vec<Notification>> {
    let mut list = Vec::new();
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = send_command(
        socket,
        &NotificationRequest::GetAll,
        Some(move |evt: NotificationEvent| {
            if let NotificationEvent::Show(n) = evt {
                let _ = tx.send(n);
            }
        }),
    )?;
    while let Ok(n) = rx.recv_timeout(std::time::Duration::from_millis(50)) {
        list.push(n);
    }
    Ok(list)
}

/// Helper function to subscribe to notification events synchronously with a callback.
pub fn subscribe_notifications<H>(
    socket: impl Into<PathBuf> + Display,
    handler: H,
) -> std::io::Result<()>
where
    H: Fn(NotificationEvent) + Send + 'static,
{
    send_command(
        socket,
        &NotificationRequest::Subscribe,
        Some(move |evt: NotificationEvent| handler(evt)),
    )
}

/// Helper function to subscribe to notification events and receive them via an asynchronous Tokio unbounded channel.
pub fn subscribe_channel(
    socket: impl Into<PathBuf> + Display,
) -> std::io::Result<tokio::sync::mpsc::UnboundedReceiver<NotificationEvent>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let socket_str = resolve_socket_path(socket).to_string_lossy().to_string();
    std::thread::spawn(move || {
        let _ = subscribe_notifications(&socket_str, move |evt| {
            let _ = tx.send(evt);
        });
    });
    Ok(rx)
}

/// Request closing a notification by ID.
pub fn close_notification(
    socket: impl Into<PathBuf> + Display,
    id: u32,
) -> std::io::Result<()> {
    send_command::<NotificationRequest, NotificationEvent, fn(NotificationEvent)>(
        socket,
        &NotificationRequest::Close(id),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread, time::Duration};

    #[test]
    fn test_notification_broadcaster_basic() {
        let broadcaster = NotificationBroadcaster::new();
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();

        broadcaster.add_subscriber(tx1);
        broadcaster.add_subscriber(tx2);
        assert_eq!(broadcaster.subscriber_count(), 2);

        let notif = Notification {
            id: 1,
            app_name: "test".into(),
            summary: "Hello".into(),
            body: "World".into(),
            icon: "info".into(),
            timeout: 5000,
        };

        broadcaster.broadcast(NotificationEvent::Show(notif.clone()));
        assert_eq!(rx1.recv().unwrap(), NotificationEvent::Show(notif.clone()));
        assert_eq!(rx2.recv().unwrap(), NotificationEvent::Show(notif));

        broadcaster.broadcast(NotificationEvent::Close(1));
        assert_eq!(rx1.recv().unwrap(), NotificationEvent::Close(1));
        assert_eq!(rx2.recv().unwrap(), NotificationEvent::Close(1));

        // Dropping rx2 should prune subscriber on next broadcast
        drop(rx2);
        broadcaster.broadcast(NotificationEvent::Close(2));
        assert_eq!(rx1.recv().unwrap(), NotificationEvent::Close(2));
        assert_eq!(broadcaster.subscriber_count(), 1);
    }

    #[tokio::test]
    async fn test_notification_ipc_server_and_client() {
        let socket = format!("test-notif-ipc-{}", std::process::id());
        let broadcaster = NotificationBroadcaster::new();
        let broadcaster_clone = broadcaster.clone();
        let socket_clone = socket.clone();

        thread::spawn(move || {
            let _ = start_notification_server::<fn(u32)>(socket_clone, broadcaster_clone, None);
        });

        // Allow server time to bind
        thread::sleep(Duration::from_millis(150));

        let mut rx = subscribe_channel(&socket).expect("subscribe_channel failed");

        // Wait for subscriber to connect
        for _ in 0..50 {
            if broadcaster.subscriber_count() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(broadcaster.subscriber_count(), 1);

        // Broadcast Show
        let notif = Notification {
            id: 42,
            app_name: "Finick App".into(),
            summary: "Alert".into(),
            body: "Something happened".into(),
            icon: "bell".into(),
            timeout: 3000,
        };
        broadcaster.broadcast(NotificationEvent::Show(notif.clone()));

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout receiving show event")
            .expect("channel closed unexpectedly");
        assert_eq!(event, NotificationEvent::Show(notif));

        // Broadcast Close
        broadcaster.broadcast(NotificationEvent::Close(42));
        let close_event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout receiving close event")
            .expect("channel closed unexpectedly");
        assert_eq!(close_event, NotificationEvent::Close(42));
    }
}
