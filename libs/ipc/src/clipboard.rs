use {
    crate::{resolve_socket_path, send_command, start_server},
    serde::{Deserialize, Serialize},
    std::{
        fmt::Display,
        path::PathBuf,
        sync::{mpsc::Sender, Arc, Mutex},
    },
};

/// Default socket name for finick clipboard IPC.
pub const CLIPBOARD_SOCKET_NAME: &str = "finick-clipboard.sock";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardItem {
    pub id: u64,
    pub text: String,
    pub timestamp: u64,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClipboardEvent {
    Added(ClipboardItem),
    Selected(ClipboardItem),
    Deleted(u64),
    Pinned { id: u64, pinned: bool },
    Cleared,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClipboardRequest {
    /// Retrieve all history items
    GetHistory,
    /// Add a new text entry to clipboard history
    Add { text: String },
    /// Select an item by ID (copies to clipboard and brings to front)
    Select { id: u64 },
    /// Delete a single entry by ID
    Delete { id: u64 },
    /// Toggle pinned status of an item
    TogglePin { id: u64 },
    /// Clear all unpinned history
    Clear,
    /// Clear all history including pinned
    ClearAll,
    /// Subscribe to live clipboard updates
    Subscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClipboardResponse {
    History(Vec<ClipboardItem>),
    Item(ClipboardItem),
    Ok,
    Event(ClipboardEvent),
    Error(String),
}

/// Thread-safe broadcaster for clipboard subscribers.
#[derive(Clone, Default)]
pub struct ClipboardBroadcaster {
    subscribers: Arc<Mutex<Vec<Sender<ClipboardResponse>>>>,
}

impl ClipboardBroadcaster {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_subscriber(&self, sender: Sender<ClipboardResponse>) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(sender);
    }

    pub fn broadcast(&self, event: ClipboardEvent) {
        let mut subs = self.subscribers.lock().unwrap();
        let resp = ClipboardResponse::Event(event);
        subs.retain(|sender| sender.send(resp.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        let subs = self.subscribers.lock().unwrap();
        subs.len()
    }
}

/// Starts the clipboard IPC server using a request handler.
pub fn start_clipboard_server<H>(
    socket_path: impl Into<PathBuf> + Display,
    broadcaster: ClipboardBroadcaster,
    handler: H,
) -> std::io::Result<()>
where
    H: Fn(ClipboardRequest, Sender<ClipboardResponse>) + Send + Sync + Clone + 'static,
{
    start_server(
        socket_path,
        move |req: ClipboardRequest, sender: Sender<ClipboardResponse>| {
            if let ClipboardRequest::Subscribe = req {
                broadcaster.add_subscriber(sender);
            } else {
                handler(req, sender);
            }
        },
    )
}

/// Fetch clipboard history synchronously from the daemon.
pub fn get_history(socket: impl Into<PathBuf> + Display) -> std::io::Result<Vec<ClipboardItem>> {
    let (tx, rx) = std::sync::mpsc::channel();
    send_command(
        socket,
        &ClipboardRequest::GetHistory,
        Some(move |resp: ClipboardResponse| {
            if let ClipboardResponse::History(items) = resp {
                let _ = tx.send(items);
            }
        }),
    )?;

    Ok(rx.recv().unwrap_or_default())
}

/// Add text entry synchronously.
pub fn add_text(socket: impl Into<PathBuf> + Display, text: String) -> std::io::Result<Option<ClipboardItem>> {
    let (tx, rx) = std::sync::mpsc::channel();
    send_command(
        socket,
        &ClipboardRequest::Add { text },
        Some(move |resp: ClipboardResponse| {
            if let ClipboardResponse::Item(item) = resp {
                let _ = tx.send(item);
            }
        }),
    )?;

    Ok(rx.recv().ok())
}

/// Select an item from history (copies to clipboard and bumps to top).
pub fn select_item(socket: impl Into<PathBuf> + Display, id: u64) -> std::io::Result<()> {
    send_command(
        socket,
        &ClipboardRequest::Select { id },
        None::<fn(ClipboardResponse)>,
    )
}

/// Delete an item by ID.
pub fn delete_item(socket: impl Into<PathBuf> + Display, id: u64) -> std::io::Result<()> {
    send_command(
        socket,
        &ClipboardRequest::Delete { id },
        None::<fn(ClipboardResponse)>,
    )
}

/// Toggle pin on an item by ID.
pub fn toggle_pin_item(socket: impl Into<PathBuf> + Display, id: u64) -> std::io::Result<()> {
    send_command(
        socket,
        &ClipboardRequest::TogglePin { id },
        None::<fn(ClipboardResponse)>,
    )
}

/// Clear unpinned history.
pub fn clear_history(socket: impl Into<PathBuf> + Display) -> std::io::Result<()> {
    send_command(
        socket,
        &ClipboardRequest::Clear,
        None::<fn(ClipboardResponse)>,
    )
}

/// Clear all history including pinned.
pub fn clear_all_history(socket: impl Into<PathBuf> + Display) -> std::io::Result<()> {
    send_command(
        socket,
        &ClipboardRequest::ClearAll,
        None::<fn(ClipboardResponse)>,
    )
}

/// Subscribe to live clipboard events via tokio channel.
pub fn subscribe_channel(
    socket: impl Into<PathBuf> + Display,
) -> std::io::Result<tokio::sync::mpsc::UnboundedReceiver<ClipboardEvent>> {
    let socket_path = resolve_socket_path(socket);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    std::thread::spawn(move || {
        let _ = send_command(
            socket_path.to_string_lossy().to_string(),
            &ClipboardRequest::Subscribe,
            Some(move |resp: ClipboardResponse| {
                if let ClipboardResponse::Event(evt) = resp {
                    let _ = tx.send(evt);
                }
            }),
        );
    });

    Ok(rx)
}
