#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use {config::ty::App, freya::prelude::*, std::process::Command, ui::*};

fn sanitize_exec(exec: &str) -> String {
    exec.split_whitespace().filter(|p| !p.starts_with('%')).collect::<Vec<_>>().join(" ")
}

fn launch_item(r: &index::ty::SearchResult) {
    if r.is_desktop && r.path.ends_with(".desktop") {
        if let Ok(content) = std::fs::read_to_string(&r.path) {
            if let Some(line) = content.lines().find(|l| l.starts_with("Exec=")) {
                let raw = line.trim_start_matches("Exec=").trim();
                let cmd = sanitize_exec(raw);
                if !cmd.is_empty() {
                    let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &cmd]).spawn();
                    std::process::exit(0);
                }
            }
        }
        let _ = Command::new("xdg-open").arg(&r.path).spawn();
    } else if r.is_dir {
        let _ = Command::new("xdg-open").arg(&r.path).spawn();
    } else if r.is_executable {
        let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &r.path]).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(&r.path).spawn();
    }
    std::process::exit(0);
}

fn search(query: String, mut results: State<Vec<index::ty::SearchResult>>, mut selected: State<usize>) {
    if query.trim().is_empty() {
        results.set(Vec::new());
        selected.set(0);
        return;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let inner_tx2 = inner_tx.clone();
        let q = query.clone();
        let res = ipsea::send_command(
            App::IndexService,
            &index::ty::Request::Search { query: q },
            Some(move |r: index::ty::SearchResult| {
                let _ = inner_tx2.send(r);
            }),
        );
        drop(inner_tx);
        let mut items: Vec<index::ty::SearchResult> = inner_rx.into_iter().collect();
        if res.is_err() {}
        items.truncate(50);
        let _ = tx.send(items);
    });
    spawn(async move {
        if let Some(items) = rx.recv().await {
            selected.set(0);
            results.set(items);
        }
    });
}

fn kind_icon(r: &index::ty::SearchResult) -> &'static str {
    if r.is_desktop {
        APPS
    } else if r.is_dir {
        FOLDER
    } else if r.is_executable {
        TERMINAL
    } else {
        FILE
    }
}

fn kind_label(r: &index::ty::SearchResult) -> &'static str {
    if r.is_desktop {
        "App"
    } else if r.is_dir {
        "Folder"
    } else if r.is_executable {
        "Exec"
    } else {
        "File"
    }
}

fn launcher_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let query = use_state(String::new);
    let results = use_state(Vec::<index::ty::SearchResult>::new);
    let selected = use_state(|| 0usize);
    let input_id = use_hook(AccessibilityId::new_unique);
    let _input_focus = use_focus(input_id);
    use_hook(move || {
        let id = input_id;
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            id.request_focus();
        });
    });
    let q_val = query.read().clone();
    let r_side = results;
    let s_side = selected;
    use_side_effect(move || {
        let q = q_val.clone();
        search(q, r_side, s_side);
    });
    let results_len = results.read().len();
    let sel = *selected.read();
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(Color::from_argb(90, 0, 0, 0))
        .center()
        .content(Content::Flex)
        .on_global_key_down({
            let mut sel_state = selected;
            let q_state = query;
            let res_state = results;
            move |e: Event<KeyboardEventData>| {
                let len = res_state.read().len();
                let cur = *sel_state.read();
                match e.data().key.clone() {
                    Key::Named(NamedKey::Escape) => std::process::exit(0),
                    Key::Named(NamedKey::ArrowDown) => {
                        if len > 0 {
                            sel_state.set((cur + 1) % len)
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if len > 0 {
                            sel_state.set(if cur == 0 { len - 1 } else { cur - 1 })
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(item) = res_state.read().get(cur).cloned() {
                            launch_item(&item);
                        } else if !q_state.read().trim().is_empty() {
                            let q = q_state.read().trim().to_string();
                            let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &q]).spawn();
                            std::process::exit(0);
                        }
                    }
                    _ => {}
                }
            }
        })
        .child(
            rect()
                .width(Size::px(640.))
                .height(Size::px(460.))
                .corner_radius(20.)
                .background(t.panel)
                .border(Border::new().width(1.).fill(t.border))
                .padding(14.)
                .spacing(10.)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(48.))
                        .corner_radius(12.)
                        .background(t.bg)
                        .border(Border::new().width(1.).fill(t.border))
                        .padding((0., 12.))
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .horizontal()
                        .spacing(10.)
                        .child(icon(SEARCH, 18., t.text_dim))
                        .child(Input::new(query).width(Size::fill()).placeholder("Search apps, files…").on_submit({
                            let res_state = results;
                            let mut sel_state = selected;
                            let q_state = query;
                            move |_| {
                                if let Some(item) = res_state.read().get(*sel_state.read()).cloned() {
                                    launch_item(&item);
                                } else if !q_state.read().trim().is_empty() {
                                    let q = q_state.read().trim().to_string();
                                    let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &q]).spawn();
                                    std::process::exit(0);
                                }
                            }
                        }))
                        .maybe(!query.read().is_empty(), |el| {
                            let mut q = query;
                            el.child(
                                rect()
                                    .width(Size::px(28.))
                                    .height(Size::px(28.))
                                    .corner_radius(999.)
                                    .background(t.panel_raised)
                                    .center()
                                    .cursor(CursorIcon::Pointer)
                                    .on_press(move |_| q.set(String::new()))
                                    .child(label().font_size(12.).color(t.text_dim).text("✕")),
                            )
                        }),
                )
                .child(rect().width(Size::fill()).height(Size::fill()).content(Content::Flex).child(if results_len == 0 {
                    rect()
                        .width(Size::fill())
                        .height(Size::fill())
                        .center()
                        .content(Content::Flex)
                        .child(
                            rect()
                                .vertical()
                                .cross_align(Alignment::Center)
                                .spacing(8.)
                                .child(icon(APPS, 28., t.text_muted))
                                .child(label().font_size(13.).color(t.text_muted).text(if query.read().trim().is_empty() {
                                    "Type to search apps & files"
                                } else {
                                    "No results — press Enter to run as command"
                                })),
                        )
                        .into_element()
                } else {
                    ScrollView::new()
                        .width(Size::fill())
                        .height(Size::fill())
                        .child(rect().width(Size::fill()).vertical().spacing(4.).children(
                            results.read().iter().enumerate().map(|(idx, item)| {
                                let is_sel = idx == sel;
                                let bg = if is_sel { t.bg_active } else { Color::TRANSPARENT };
                                let border = if is_sel { t.accent } else { Color::TRANSPARENT };
                                let item_for_press = item.clone();
                                let item_for_icon = item.clone();
                                let item_for_label = item.clone();
                                rect()
                                    .width(Size::fill())
                                    .height(Size::px(56.))
                                    .corner_radius(12.)
                                    .background(bg)
                                    .border(Border::new().width(1.).fill(border))
                                    .padding((8., 12.))
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .content(Content::Flex)
                                    .spacing(12.)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press(move |_| launch_item(&item_for_press))
                                    .child(
                                        rect()
                                            .width(Size::px(36.))
                                            .height(Size::px(36.))
                                            .corner_radius(10.)
                                            .background(if is_sel { t.panel } else { t.bg })
                                            .border(Border::new().width(1.).fill(t.border))
                                            .center()
                                            .child(icon(
                                                kind_icon(&item_for_icon),
                                                18.,
                                                if is_sel { t.accent } else { t.text_dim },
                                            )),
                                    )
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .vertical()
                                            .content(Content::Flex)
                                            .child(
                                                label()
                                                    .font_size(13.)
                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                    .color(t.text)
                                                    .text(item_for_label.name.clone()),
                                            )
                                            .child(
                                                label().font_size(11.).color(t.text_dim).text(item_for_label.path.clone()),
                                            ),
                                    )
                                    .child(
                                        rect()
                                            .padding((4., 8.))
                                            .corner_radius(999.)
                                            .background(if is_sel { t.accent } else { t.panel_raised })
                                            .child(
                                                label()
                                                    .font_size(10.)
                                                    .font_weight(FontWeight::BOLD)
                                                    .color(if is_sel { t.panel } else { t.text_muted })
                                                    .text(kind_label(item).to_string()),
                                            ),
                                    )
                                    .into_element()
                            }),
                        ))
                        .into_element()
                }))
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .main_align(Alignment::SpaceBetween)
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .child(label().font_size(11.).color(t.text_muted).text("↑↓ Navigate  ⏎ Launch  Esc Close"))
                        .child(label().font_size(11.).color(t.text_muted).text(format!("{} results", results_len))),
                ),
        )
        .into_element()
}

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(launcher_app)
                .with_title("launcher")
                .with_app_id("launcher")
                .with_size(700., 540.)
                .with_min_size(640., 460.)
                .with_decorations(false)
                .with_transparency(true)
                .with_background(Color::TRANSPARENT),
        ),
    );
}
