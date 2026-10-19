use {
    freya::prelude::*,
    ipsea::clipboard::{
        CLIPBOARD_SOCKET_NAME, ClipboardBroadcaster, ClipboardEvent, ClipboardItem, ClipboardRequest, ClipboardResponse,
    },
    std::sync::{Arc, Mutex},
    ui::*,
};

fn trunc(s: &str, n: usize) -> String {
    let n = n.max(8);
    let flat = s.replace(|c: char| c.is_control() && c != '\n' && c != '\t', " ");
    let first_line = flat.lines().next().unwrap_or("").to_string();
    let mut it = first_line.chars();
    let mut out: String = it.by_ref().take(n).collect();
    if it.next().is_some() || flat.lines().count() > 1 {
        while out.chars().count() >= n {
            out.pop();
        }
        out.push('…');
    }
    if out.is_empty() { "(empty)".to_string() } else { out }
}

#[derive(PartialEq)]
pub struct ClipboardManager {
    pub dismiss: State<bool>,
}

impl Component for ClipboardManager {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let items = use_state(Vec::<ClipboardItem>::new);
        let filter = use_state(String::new);
        let mut dismiss = self.dismiss;
        let wid = Platform::window_id();
        use_hook(move || {
            let mut it = items;
            spawn(async move {
                loop {
                    let initial =
                        tokio::task::spawn_blocking(|| ipsea::clipboard::get_history(CLIPBOARD_SOCKET_NAME).unwrap_or_default())
                            .await
                            .unwrap_or_default();
                    let mut sorted = initial;
                    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    it.set(sorted);
                    if let Ok(mut rx) = ipsea::clipboard::subscribe_channel(CLIPBOARD_SOCKET_NAME) {
                        while let Some(evt) = rx.recv().await {
                            match evt {
                                ClipboardEvent::Added(item) => {
                                    let mut cur = it.read().clone();
                                    cur.retain(|x| x.id != item.id);
                                    cur.insert(0, item);
                                    it.set(cur);
                                }
                                ClipboardEvent::Selected(item) => {
                                    let mut cur = it.read().clone();
                                    cur.retain(|x| x.id != item.id);
                                    cur.insert(0, item);
                                    it.set(cur);
                                }
                                ClipboardEvent::Deleted(id) => {
                                    let mut cur = it.read().clone();
                                    cur.retain(|x| x.id != id);
                                    it.set(cur);
                                }
                                ClipboardEvent::Pinned { id, pinned } => {
                                    let mut cur = it.read().clone();
                                    for x in cur.iter_mut() {
                                        if x.id == id {
                                            x.pinned = pinned;
                                        }
                                    }
                                    it.set(cur);
                                }
                                ClipboardEvent::Cleared => {
                                    let mut cur = it.read().clone();
                                    cur.retain(|x| x.pinned);
                                    it.set(cur);
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
        });
        let query = filter.read().to_lowercase();
        let list = items.read().clone();
        let mut filtered: Vec<ClipboardItem> =
            list.into_iter().filter(|i| query.is_empty() || i.text.to_lowercase().contains(&query)).collect();
        filtered.truncate(100);
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(6.)
            .background(Color::TRANSPARENT)
            .content(Content::Flex)
            .child(
                PanelShell::new(dismiss.into_readable())
                    .duration(300)
                    .scale_from(1.0)
                    .on_dismiss(EventHandler::new(move |()| Platform::get().close_window(wid)))
                    .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::auto())
                        .corner_radius(20.)
                        .background(t.panel)
                        .border(Border::new().width(1.).fill(t.border))
                        .padding(12.)
                        .vertical()
                        .spacing(10.)
                        .content(Content::Flex)
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .main_align(Alignment::SpaceBetween)
                                .content(Content::Flex)
                                .child(
                                    label()
                                        .font_size(16.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(t.text)
                                        .text("Clipboard History"),
                                )
                                .child(
                                    rect()
                                        .horizontal()
                                        .spacing(8.)
                                        .content(Content::Flex)
                                        .child(ghost_button("Clear unpinned", {
                                            let mut it = items;
                                            move || {
                                                let _ = ipsea::clipboard::clear_history(CLIPBOARD_SOCKET_NAME);
                                                let mut cur = it.read().clone();
                                                cur.retain(|x| x.pinned);
                                                it.set(cur);
                                            }
                                        }))
                                        .child(small_close_button(move || dismiss.set(true))),
                                ),
                        )
                        .child(sidebar_search(filter, "Search clipboard…"))
                        .child({
                            let n = filtered.len();
                            let sc = n.min(8);
                            let dr = dismiss.into_readable();
                            let col =
                                rect().width(Size::fill()).vertical().spacing(6.).content(Content::Flex).children(
                                        filtered.into_iter().enumerate().map(|(i, it)| {
                                            let id = it.id;
                                            let txt = trunc(&it.text, 80);
                                            let pinned = it.pinned;
                                            StaggerRow::new(i, sc, dr.clone()).child(rect()
                                                .width(Size::fill())
                                                .padding((8., 10.))
                                                .corner_radius(10.)
                                                .background(t.bg)
                                                .border(Border::new().width(1.).fill(t.border))
                                                .horizontal()
                                                .cross_align(Alignment::Center)
                                                .spacing(8.)
                                                .content(Content::Flex)
                                                .child(
                                                    rect()
                                                        .width(Size::flex(1.))
                                                        .vertical()
                                                        .spacing(4.)
                                                        .content(Content::Flex)
                                                        .child(label().font_size(12.).color(t.text).text(txt.clone()))
                                                        .child(label().font_size(10.).color(t.text_dim).text(if pinned {
                                                            "Pinned"
                                                        } else {
                                                            ""
                                                        })),
                                                )
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .spacing(6.)
                                                        .content(Content::Flex)
                                                        .child(tool_button(
                                                            PIN,
                                                            if pinned { t.accent } else { t.text_dim },
                                                            {
                                                                move || {
                                                                    let _ = ipsea::clipboard::toggle_pin_item(
                                                                        CLIPBOARD_SOCKET_NAME,
                                                                        id,
                                                                    );
                                                                }
                                                            },
                                                        ))
                                                        .child(tool_button(COPY, t.text, {
                                                            move || {
                                                                let _ =
                                                                    ipsea::clipboard::select_item(CLIPBOARD_SOCKET_NAME, id);
                                                            }
                                                        }))
                                                        .child(tool_button(TRASH, t.accent_red, move || {
                                                            let _ = ipsea::clipboard::delete_item(
                                                                CLIPBOARD_SOCKET_NAME,
                                                                id,
                                                            );
                                                        })),
                                                )
                                                )
                                                .into_element()
                                        }),
                                );
                            rect().width(Size::fill()).height(Size::auto()).content(Content::Flex).child(
                                if n == 0 {
                                    empty_state(COPY, "No clipboard items", "Copy text to see it here").into_element()
                                } else if n > 4 {
                                    ScrollView::new()
                                        .width(Size::fill())
                                        .height(Size::px(240.))
                                        .child(col)
                                        .into_element()
                                } else {
                                    col.into_element()
                                },
                            )
                        }),
                ),
            )
            .into_element()
    }
}

fn now_secs() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

#[derive(Default)]
struct ClipboardStore {
    items: Vec<ClipboardItem>,
    next_id: u64,
}

impl ClipboardStore {
    fn add(&mut self, text: String) -> Option<ClipboardItem> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut owned = trimmed.to_string();
        if owned.chars().count() > 100_000 {
            owned = owned.chars().take(100_000).collect();
        }
        if self.items.first().is_some_and(|f| f.text == owned) {
            return None;
        }
        self.items.retain(|x| x.text != owned);
        self.next_id += 1;
        let item = ClipboardItem { id: self.next_id, text: owned, timestamp: now_secs(), pinned: false };
        self.items.insert(0, item.clone());
        self.items.truncate(100);
        Some(item)
    }
}

/// Starts the clipboard history IPC server (in-memory store) plus a
/// Wayland clipboard watcher. Called once from the overlay entry point;
/// without this no daemon answers clipboard requests and every action
/// silently fails or hangs on stale sockets.
pub fn start_clipboard_service() {
    let store = Arc::new(Mutex::new(ClipboardStore::default()));
    let broadcaster = ClipboardBroadcaster::new();

    let handler_store = store.clone();
    let handler_broadcaster = broadcaster.clone();
    let handler = move |req: ClipboardRequest, sender: std::sync::mpsc::Sender<ClipboardResponse>| {
        let reply = match req {
            ClipboardRequest::GetHistory => {
                let items = handler_store.lock().map(|s| s.items.clone()).unwrap_or_default();
                ClipboardResponse::History(items)
            }
            ClipboardRequest::Add { text } => match handler_store.lock().ok().and_then(|mut s| s.add(text)) {
                Some(item) => {
                    handler_broadcaster.broadcast(ClipboardEvent::Added(item.clone()));
                    ClipboardResponse::Item(item)
                }
                None => ClipboardResponse::Ok,
            },
            ClipboardRequest::Select { id } => {
                let item = handler_store.lock().ok().and_then(|mut s| {
                    let pos = s.items.iter().position(|x| x.id == id)?;
                    let mut item = s.items.remove(pos);
                    item.timestamp = now_secs();
                    s.items.insert(0, item.clone());
                    Some(item)
                });
                match item {
                    Some(item) => {
                        let _ = system::copy_text(&item.text);
                        handler_broadcaster.broadcast(ClipboardEvent::Selected(item));
                        ClipboardResponse::Ok
                    }
                    None => ClipboardResponse::Error(format!("clipboard item {id} not found")),
                }
            }
            ClipboardRequest::Delete { id } => {
                if let Ok(mut s) = handler_store.lock() {
                    s.items.retain(|x| x.id != id);
                }
                handler_broadcaster.broadcast(ClipboardEvent::Deleted(id));
                ClipboardResponse::Ok
            }
            ClipboardRequest::TogglePin { id } => {
                let pinned = handler_store
                    .lock()
                    .ok()
                    .map(|mut s| {
                        let mut pinned = false;
                        for x in s.items.iter_mut() {
                            if x.id == id {
                                x.pinned = !x.pinned;
                                pinned = x.pinned;
                            }
                        }
                        pinned
                    })
                    .unwrap_or(false);
                handler_broadcaster.broadcast(ClipboardEvent::Pinned { id, pinned });
                ClipboardResponse::Ok
            }
            ClipboardRequest::Clear => {
                if let Ok(mut s) = handler_store.lock() {
                    s.items.retain(|x| x.pinned);
                }
                handler_broadcaster.broadcast(ClipboardEvent::Cleared);
                ClipboardResponse::Ok
            }
            ClipboardRequest::ClearAll => {
                if let Ok(mut s) = handler_store.lock() {
                    s.items.clear();
                }
                handler_broadcaster.broadcast(ClipboardEvent::Cleared);
                ClipboardResponse::Ok
            }
            ClipboardRequest::Subscribe => return,
        };
        let _ = sender.send(reply);
    };

    let server_broadcaster = broadcaster.clone();
    std::thread::spawn(move || {
        let _ = ipsea::clipboard::start_clipboard_server(CLIPBOARD_SOCKET_NAME, server_broadcaster, handler);
    });

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let text = match system::paste_text() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if text.trim().is_empty() {
                continue;
            }
            if let Some(item) = store.lock().ok().and_then(|mut s| s.add(text)) {
                broadcaster.broadcast(ClipboardEvent::Added(item));
            }
        }
    });
}

pub fn clipboard_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let dismiss = use_state(|| false);
    let wid = Platform::window_id();
    use_hook(move || {
        let my_wid = wid;
        crate::state::spawn_panel_focus_watcher_guarded(
            &["overlay-clipboard", "overlay", "topbar"],
            "clipboard",
            dismiss,
            move || crate::state::is_clipboard_window_current(my_wid),
        );
    });
    use_hook(move || {
        let my_wid = wid;
        crate::state::clear_close_flag(&crate::state::CLOSE_CLIPBOARD);
        crate::state::spawn_close_flag_watcher_guarded(
            &crate::state::CLOSE_CLIPBOARD,
            dismiss,
            move || crate::state::is_clipboard_window_current(my_wid),
        );
    });
    ClipboardManager { dismiss }.into_element()
}

pub fn clipboard_window_config() -> WindowConfig {
    WindowConfig::new(clipboard_app)
        .with_title("clipboard")
        .with_app_id("overlay-clipboard")
        .with_size(520., 380.)
        .with_min_size(520., 340.)
        .with_max_size(520., 600.)
        .with_decorations(false)
        .with_transparency(true)
        .with_resizable(false)
        .with_background(Color::TRANSPARENT)
        .with_on_close(|ctx, _wid| {
            if let Some(cw) = ctx.global_contexts.try_get_context::<crate::ClipboardWindowId>()
                && let Ok(mut g) = cw.0.lock() {
                    *g = None;
                }
            CloseDecision::Close
        })
}


