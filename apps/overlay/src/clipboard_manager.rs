use freya::prelude::*;
use ipsea::clipboard::{CLIPBOARD_SOCKET_NAME, ClipboardItem, ClipboardEvent};
use ui::*;

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { let mut t: String = s.chars().take(n-1).collect(); t.push('…'); t }
}

#[derive(PartialEq)]
pub struct ClipboardManager;

impl Component for ClipboardManager {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let items = use_state(Vec::<ClipboardItem>::new);
        let filter = use_state(String::new);
        use_hook(move || {
            let mut it = items;
            spawn(async move {
                let initial = tokio::task::spawn_blocking(|| ipsea::clipboard::get_history(CLIPBOARD_SOCKET_NAME).unwrap_or_default()).await.unwrap_or_default();
                let mut sorted = initial;
                sorted.sort_by(|a,b| b.timestamp.cmp(&a.timestamp));
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
                                for x in cur.iter_mut() { if x.id==id { x.pinned = pinned; } }
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
            });
        });
        let query = filter.read().to_lowercase();
        let list = items.read().clone();
        let filtered: Vec<ClipboardItem> = list.into_iter().filter(|i| query.is_empty() || i.text.to_lowercase().contains(&query)).collect();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(6.)
            .background(Color::TRANSPARENT)
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
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
                            .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text).text("Clipboard History"))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .content(Content::Flex)
                                    .child(ghost_button("Clear unpinned", {
                                        let mut it = items;
                                        move || {
                                            let _ = ipsea::clipboard::clear_history(CLIPBOARD_SOCKET_NAME);
                                            let mut cur = it.read().clone(); cur.retain(|x| x.pinned); it.set(cur);
                                        }
                                    }))
                                    .child(
                                        rect()
                                            .width(Size::px(28.))
                                            .height(Size::px(28.))
                                            .corner_radius(999.)
                                            .background(t.panel_raised)
                                            .border(Border::new().width(1.).fill(t.border))
                                            .center()
                                            .cursor(CursorIcon::Pointer)
                                            .on_press(|_| Platform::get().close_window(Platform::window_id()))
                                            .child(label().font_size(12.).color(t.text_dim).text("✕")),
                                    ),
                            ),
                    )
                    .child(Input::new(filter).width(Size::fill()).placeholder("Search clipboard…"))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .content(Content::Flex)
                            .child(if filtered.is_empty() {
                                rect().width(Size::fill()).height(Size::fill()).center().child(label().font_size(12.).color(t.text_dim).text("No clipboard items")).into_element()
                            } else {
                                ScrollView::new().width(Size::fill()).height(Size::fill()).child(
                                    rect().width(Size::fill()).vertical().spacing(6.).children(filtered.into_iter().map(|it| {
                                        let id = it.id;
                                        let txt = trunc(&it.text, 80);
                                        let pinned = it.pinned;
                                        rect()
                                            .width(Size::fill())
                                            .padding((8.,10.))
                                            .corner_radius(10.)
                                            .background(t.bg)
                                            .border(Border::new().width(1.).fill(t.border))
                                            .horizontal()
                                            .cross_align(Alignment::Center)
                                            .spacing(8.)
                                            .content(Content::Flex)
                                            .child(rect().width(Size::flex(1.)).vertical().spacing(4.).content(Content::Flex).child(label().font_size(12.).color(t.text).text(txt.clone())).child(label().font_size(10.).color(t.text_dim).text(if pinned {"Pinned"} else {""})))
                                            .child(
                                                rect().horizontal().spacing(6.).content(Content::Flex)
                                                    .child(ghost_button(if pinned {"Unpin"} else {"Pin"}, {
                                                        let id = id;
                                                        move || { let _ = ipsea::clipboard::toggle_pin_item(CLIPBOARD_SOCKET_NAME, id); }
                                                    }))
                                                    .child(secondary_button("Copy", {
                                                        let id = id;
                                                        move || { let _ = ipsea::clipboard::select_item(CLIPBOARD_SOCKET_NAME, id); }
                                                    }))
                                                    .child(
                                                        rect().width(Size::px(28.)).height(Size::px(28.)).corner_radius(6.).background(t.panel_raised).border(Border::new().width(1.).fill(t.border)).center().cursor(CursorIcon::Pointer).on_press(move |_| { let _ = ipsea::clipboard::delete_item(CLIPBOARD_SOCKET_NAME, id); }).child(label().font_size(11.).color(t.accent_red).text("✕"))
                                                    ),
                                            )
                                            .into_element()
                                    }))
                                ).into_element()
                            }),
                    ),
            )
            .into_element()
    }
}

pub fn clipboard_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    use_hook(|| {
        let wid = Platform::window_id();
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
                        if s.contains("\"class\": \"overlay-clipboard\"") {
                            return false;
                        }
                        return !s.contains("\"title\": \"clipboard\"");
                    }
                    false
                })
                .await
                .unwrap_or(false);
                if should_close {
                    Platform::get().close_window(wid);
                    break;
                }
            }
        });
    });
    ClipboardManager.into_element()
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
        .with_on_close(|_ctx, _wid| {
            if let Some(cw) = GlobalContexts::get().try_get_context::<crate::ClipboardWindowId>() {
                if let Ok(mut g) = cw.0.lock() {
                    *g = None;
                }
            }
            CloseDecision::Close
        })
}

pub fn open_clipboard_manager() {
    let mon = crate::get_monitor_states().into_iter().find(|m| m.x == 0).unwrap_or(crate::MonitorState { name: "default".to_string(), x: 0, y: 0, width: 1920.0, height: 1080.0 });
    let w = 520;
    let h = 380;
    let px = mon.x + mon.width as i32 - w - 12;
    let py = mon.y + 44;
    let lua_rule = format!(
        r#"hl.window_rule({{ name = "finick-clipboard-position", match = {{ class = "^(overlay-clipboard)$" }}, monitor = "{mon}", float = true, pin = true, move = {{ {px}, {py} }}, size = {{ {w}, {h} }}, border_size = 0, no_shadow = true, no_anim = true, no_blur = true, rounding = 20 }})"#,
        mon = mon.name, px = px, py = py, w = w, h = h
    );
    let _ = std::process::Command::new("hyprctl").args(["eval", &lua_rule]).output();
    let cfg = clipboard_window_config();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(35)).await;
        let wid = Platform::get().launch_window(cfg).await;
        let _ = wid;
        tokio::spawn(async move {
            for delay in [30, 80] {
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                let lua = format!(
                    r#"
                    for _, w in ipairs(hl.get_windows()) do
                        if w.class == 'overlay-clipboard' or w.title == 'clipboard' then
                            local addr = 'address:' .. tostring(w.address)
                            hl.dispatch(hl.dsp.window.float({{ window = addr }}))
                            hl.dispatch(hl.dsp.window.pin({{ window = addr }}))
                            hl.dispatch(hl.dsp.window.move({{ window = addr, x = {px}, y = {py} }}))
                            hl.dispatch(hl.dsp.window.resize({{ window = addr, x = {w}, y = {h}, relative = false }}))
                            break
                        end
                    end
                    "#,
                    px = px, py = py, w = w, h = h
                );
                let _ = std::process::Command::new("hyprctl").args(["eval", &lua]).output();
            }
        });
    });
}
