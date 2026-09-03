#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use config::ty::App;
use freya::prelude::*;
use std::{env, path::PathBuf};
use ui::*;

#[derive(Clone, PartialEq, Debug)]
enum ItemType {
    File,
    Folder,
}

#[derive(Clone, PartialEq, Debug)]
struct Item {
    ty: ItemType,
    name: String,
    path: String,
    size: u64,
}

#[derive(Clone, PartialEq)]
enum ViewMode {
    List,
    Grid,
}

fn load_dir(path: String, mut items_state: State<Vec<Item>>) {
    items_state.set(Vec::new());

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    std::thread::spawn(move || {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let tx_clone = inner_tx.clone();

        let path_clone = path.clone();

        let res = ipsea::send_command(
            App::IndexService,
            &index::ty::Request::ListDir { path: path_clone },
            Some(move |res: index::ty::SearchResult| {
                let _ = tx_clone.send(Item {
                    ty: if res.is_dir { ItemType::Folder } else { ItemType::File },
                    name: res.name,
                    path: res.path,
                    size: res.size.unwrap_or(0),
                });
            }),
        );

        drop(inner_tx);

        let mut result: Vec<Item> = inner_rx.into_iter().collect();
        if result.is_empty() || res.is_err() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let md = entry.metadata().ok();
                    let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
                    result.push(Item {
                        ty: if is_dir { ItemType::Folder } else { ItemType::File },
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path().to_string_lossy().to_string(),
                        size,
                    });
                }
            }
        }

        result.sort_by(|a, b| {
            if a.ty == ItemType::Folder && b.ty == ItemType::File {
                std::cmp::Ordering::Less
            } else if a.ty == ItemType::File && b.ty == ItemType::Folder {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });
        let _ = tx.send(result);
    });

    spawn(async move {
        if let Some(items) = rx.recv().await {
            items_state.set(items);
        }
    });
}

fn perform_search(query: String, mut items_state: State<Vec<Item>>) {
    items_state.set(Vec::new());

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    std::thread::spawn(move || {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let tx_clone = inner_tx.clone();

        let _ = ipsea::send_command(
            App::IndexService,
            &index::ty::Request::Search { query },
            Some(move |res: index::ty::SearchResult| {
                let _ = tx_clone.send(Item {
                    ty: if res.is_dir { ItemType::Folder } else { ItemType::File },
                    name: res.name,
                    path: res.path,
                    size: res.size.unwrap_or(0),
                });
            }),
        );

        drop(inner_tx);

        let mut result: Vec<Item> = inner_rx.into_iter().collect();
        result.sort_by(|a, b| {
            if a.ty == ItemType::Folder && b.ty == ItemType::File {
                std::cmp::Ordering::Less
            } else if a.ty == ItemType::File && b.ty == ItemType::Folder {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });
        let _ = tx.send(result);
    });

    spawn(async move {
        if let Some(items) = rx.recv().await {
            items_state.set(items);
        }
    });
}

fn format_size(size: u64) -> String {
    if size == 0 {
        return String::new();
    }
    let kb = size / 1024;
    if kb < 1024 { format!("{} KB", kb) } else { format!("{:.1} MB", kb as f64 / 1024.0) }
}

fn app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let mut current_path = use_state(|| env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()));
    let items = use_state(|| Vec::<Item>::new());
    let mut selected_item = use_state(|| Option::<Item>::None);
    let mut view_mode = use_state(|| ViewMode::Grid);

    let mut show_uri_bar = use_state(|| false);
    let mut uri_input = use_state(|| String::new());
    let search_query = use_state(|| String::new());

    let cp = current_path.read().clone();
    let mut last_cp = use_state(|| cp.clone());
    if *last_cp.read() != cp {
        last_cp.set(cp.clone());
        uri_input.set(cp.clone());
    }

    let mut pinned = use_state(|| Vec::<(String, String)>::new());

    let mut loaded = use_state(|| false);
    if !*loaded.read() {
        loaded.set(true);
        load_dir(current_path.read().clone(), items.clone());
    }

    let sidebar = rect()
        .width(Size::px(240.))
        .height(Size::fill())
        .background(t.bg_sidebar)
        .border(Border::new().width(1.).fill(t.border_card))
        .padding(24.)
        .child(
            label()
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(t.text_muted)
                .margin((0., 0., 24., 0.))
                .text("PLACES"),
        )
        .child(sidebar_item(
            "Home",
            HOME,
            &current_path,
            env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items.clone(),
        ))
        .child(sidebar_item(
            "Documents",
            DOCUMENT,
            &current_path,
            env::home_dir().map(|v| v.join("Documents").to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items.clone(),
        ))
        .child(sidebar_item(
            "Downloads",
            DOWNLOAD,
            &current_path,
            env::home_dir().map(|v| v.join("Downloads").to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items.clone(),
        ))
        .child(sidebar_item(
            "Pictures",
            PICTURE,
            &current_path,
            env::home_dir().map(|v| v.join("Pictures").to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items.clone(),
        ))
        .child(if !pinned.read().is_empty() {
            let el: Element = rect()
                .margin((24., 0., 0., 0.))
                .child(
                    label()
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(t.text_muted)
                        .margin((0., 0., 16., 0.))
                        .text("PINNED"),
                )
                .children(
                    pinned
                        .read()
                        .iter()
                        .map(|(name, path)| sidebar_item(name, PIN, &current_path, path.clone(), items.clone())),
                )
                .into_element();
            el
        } else {
            let el: Element = rect().into_element();
            el
        });

    let path_str = current_path.read().clone();

    let top_bar = rect()
        .height(Size::px(80.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(24.)
        .background(t.bg_base)
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .cross_align(Alignment::Center)
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(t.bg_card)
                        .on_press({
                            let mut current = current_path.clone();
                            let items = items.clone();
                            move |_| {
                                let path = PathBuf::from(current.read().clone());
                                if let Some(parent) = path.parent() {
                                    let new_path = parent.to_string_lossy().to_string();
                                    current.set(new_path.clone());
                                    load_dir(new_path, items.clone());
                                }
                            }
                        })
                        .child(icon(ARROW_LEFT, 16., t.text_primary)),
                )
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(8.)
                        .child(Input::new(uri_input.clone()))
                        .child(
                            Button::new()
                                .on_press({
                                    let mut current = current_path.clone();
                                    let items = items.clone();
                                    let uri = uri_input.clone();
                                    move |_| {
                                        let target = uri.read().clone();
                                        current.set(target.clone());
                                        load_dir(target, items.clone());
                                    }
                                })
                                .child(label().text("Go").color(t.text_primary)),
                        ),
                ),
        )
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .main_align(Alignment::End)
                .spacing(8.)
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(4.)
                        .child(Input::new(search_query.clone()))
                        .child(
                            rect()
                                .padding(8.)
                                .corner_radius(8.)
                                .background(t.bg_card)
                                .on_press({
                                    let items = items.clone();
                                    let query = search_query.clone();
                                    move |_| {
                                        perform_search(query.read().clone(), items.clone());
                                    }
                                })
                                .child(icon(SEARCH, 16., t.text_primary)),
                        ),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(if *view_mode.read() == ViewMode::Grid { t.bg_active } else { t.bg_card })
                        .on_press({
                            let mut vm = view_mode.clone();
                            move |_| vm.set(ViewMode::Grid)
                        })
                        .child(icon(
                            LAYOUT_GRID,
                            16.,
                            if *view_mode.read() == ViewMode::Grid { t.text_primary } else { t.text_muted },
                        )),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(if *view_mode.read() == ViewMode::List { t.bg_active } else { t.bg_card })
                        .on_press({
                            let mut vm = view_mode.clone();
                            move |_| vm.set(ViewMode::List)
                        })
                        .child(icon(
                            LIST,
                            16.,
                            if *view_mode.read() == ViewMode::List { t.text_primary } else { t.text_muted },
                        )),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(t.primary_accent)
                        .on_press({
                            let path_state = current_path.clone();
                            let items_state = items.clone();
                            move |_| {
                                let target_path = PathBuf::from(path_state.read().clone()).join("New Folder");
                                let mut unique_path = target_path.clone();
                                let mut counter = 1;
                                while unique_path.exists() {
                                    unique_path =
                                        PathBuf::from(path_state.read().clone()).join(format!("New Folder ({})", counter));
                                    counter += 1;
                                }
                                let _ = std::fs::create_dir_all(&unique_path);
                                load_dir(path_state.read().clone(), items_state.clone());
                            }
                        })
                        .child(icon(PLUS, 16., t.bg_base)),
                ),
        );

    let is_grid = *view_mode.read() == ViewMode::Grid;
    let items_read = items.read().clone();

    let items_view = ScrollView::new().width(Size::fill()).height(Size::fill()).child(
        rect()
            .direction(if is_grid { Direction::Horizontal } else { Direction::Vertical })
            .width(Size::fill())
            .padding(24.)
            .children(items_read.into_iter().map(|item| {
                let mut path_state = current_path.clone();
                let items_state = items.clone();
                let mut sel_state = selected_item.clone();
                let path = item.path.clone();
                let is_folder = item.ty == ItemType::Folder;
                let is_selected = sel_state.read().as_ref().map_or(false, |s| s.path == path);
                let item_clone = item.clone();

                let bg = if is_selected { t.bg_selected } else { Color::TRANSPARENT };
                let i_svg = if is_folder { FOLDER } else { FILE };

                if is_grid {
                    rect()
                        .width(Size::px(120.))
                        .height(Size::px(140.))
                        .padding(12.)
                        .margin(8.)
                        .corner_radius(16.)
                        .background(bg)
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::Center)
                        .on_press(move |_| {
                            if is_folder && is_selected {
                                path_state.set(path.clone());
                                load_dir(path.clone(), items_state.clone());
                            }
                            sel_state.set(Some(item_clone.clone()));
                        })
                        .child(rect().margin((0., 0., 12., 0.)).child(icon(
                            i_svg,
                            48.,
                            if is_folder { t.primary_accent } else { t.text_secondary },
                        )))
                        .child(
                            label()
                                .font_size(13.)
                                .color(t.text_primary)
                                .text_align(TextAlign::Center)
                                .text(item.name.clone()),
                        )
                        .into_element()
                } else {
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .padding(12.)
                        .corner_radius(12.)
                        .background(bg)
                        .cross_align(Alignment::Center)
                        .on_press(move |_| {
                            if is_folder && is_selected {
                                path_state.set(path.clone());
                                load_dir(path.clone(), items_state.clone());
                            }
                            sel_state.set(Some(item_clone.clone()));
                        })
                        .child(rect().margin((0., 16., 0., 0.)).child(icon(
                            i_svg,
                            24.,
                            if is_folder { t.primary_accent } else { t.text_secondary },
                        )))
                        .child(
                            rect()
                                .width(Size::fill())
                                .child(label().font_size(14.).color(t.text_primary).text(item.name.clone())),
                        )
                        .child(
                            label()
                                .font_size(12.)
                                .color(t.text_muted)
                                .width(Size::px(80.))
                                .text_align(TextAlign::End)
                                .text(format_size(item.size)),
                        )
                        .into_element()
                }
            })),
    );

    let drawer: Element = if let Some(sel) = selected_item.read().clone() {
        let is_folder = sel.ty == ItemType::Folder;
        rect()
            .width(Size::px(300.))
            .height(Size::fill())
            .background(t.bg_surface)
            .padding(24.)
            .child(
                label()
                    .font_size(12.)
                    .font_weight(FontWeight::BOLD)
                    .color(t.text_muted)
                    .margin((0., 0., 24., 0.))
                    .text("PROPERTIES"),
            )
            .child({
                let p = sel.path.to_lowercase();
                if p.ends_with(".png")
                    || p.ends_with(".jpg")
                    || p.ends_with(".jpeg")
                    || p.ends_with(".gif")
                    || p.ends_with(".webp")
                {
                    let path_buf = std::path::PathBuf::from(sel.path.clone());
                    rect()
                        .width(Size::fill())
                        .height(Size::px(180.))
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::Center)
                        .margin((0., 0., 24., 0.))
                        .corner_radius(12.)
                        .background(t.bg_card)
                        .overflow(Overflow::Clip)
                        .child(ImageViewer::new(path_buf).width(Size::fill()).height(Size::fill()))
                        .into_element()
                } else {
                    rect()
                        .width(Size::fill())
                        .cross_align(Alignment::Center)
                        .margin((0., 0., 24., 0.))
                        .child(icon(
                            if is_folder { FOLDER } else { FILE },
                            64.,
                            if is_folder { t.primary_accent } else { t.text_secondary },
                        ))
                        .into_element()
                }
            })
            .child(
                label()
                    .font_size(18.)
                    .font_weight(FontWeight::BOLD)
                    .color(t.text_primary)
                    .margin((0., 0., 8., 0.))
                    .text(sel.name.clone()),
            )
            .child(label().font_size(12.).color(t.text_muted).margin((0., 0., 24., 0.)).text(format_size(sel.size)))
            .child(
                rect()
                    .margin((0., 0., 16., 0.))
                    .child(label().font_size(10.).color(t.text_muted).text("PATH"))
                    .child(label().font_size(12.).color(t.text_primary).text(sel.path.clone())),
            )
            .child(
                rect()
                    .margin((0., 0., 16., 0.))
                    .child(label().font_size(10.).color(t.text_muted).text("TYPE"))
                    .child(label().font_size(12.).color(t.text_primary).text(if is_folder { "Directory" } else { "File" })),
            )
            .child({
                let s_path = sel.path.clone();
                let mut path_state = current_path.clone();
                let items_state = items.clone();

                rect()
                    .margin((16., 0., 0., 0.))
                    .child(primary_button(if is_folder { "Open Directory" } else { "Open File" }, move || {
                        if is_folder {
                            path_state.set(s_path.clone());
                            load_dir(s_path.clone(), items_state.clone());
                        } else {
                            let _ = std::process::Command::new("xdg-open").arg(&s_path).spawn();
                        }
                    }))
                    .into_element()
            })
            .child({
                if is_folder {
                    let mut pin_state = pinned.clone();
                    let s_name = sel.name.clone();
                    let s_path = sel.path.clone();
                    let is_pinned = pin_state.read().iter().any(|(_, p)| p == &s_path);
                    let el: Element = rect()
                        .margin((16., 0., 0., 0.))
                        .child(secondary_button(
                            if is_pinned { "Unpin from Sidebar" } else { "Pin to Sidebar" },
                            move || {
                                let mut current = pin_state.read().clone();
                                if is_pinned {
                                    current.retain(|(_, p)| p != &s_path);
                                } else {
                                    current.push((s_name.clone(), s_path.clone()));
                                }
                                pin_state.set(current);
                            },
                        ))
                        .into_element();
                    el
                } else {
                    let el: Element = rect().into_element();
                    el
                }
            })
            .child({
                let s_path = sel.path.clone();
                let mut path_state = current_path.clone();
                let items_state = items.clone();
                let mut sel_state = selected_item.clone();

                rect()
                    .margin((16., 0., 0., 0.))
                    .child(danger_button("Delete", move || {
                        if is_folder {
                            let _ = std::fs::remove_dir_all(&s_path);
                        } else {
                            let _ = std::fs::remove_file(&s_path);
                        }
                        sel_state.set(None);
                        load_dir(path_state.read().clone(), items_state.clone());
                    }))
                    .into_element()
            })
            .into_element()
    } else {
        rect().width(Size::px(0.)).into_element()
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .height(Size::fill())
        .background(t.bg_base)
        .child(sidebar)
        .child(rect().vertical().expanded().child(top_bar).child(items_view))
        .child(drawer)
        .into_element()
}

fn sidebar_item(
    name: &str,
    icon_svg: &'static str,
    current_path: &State<String>,
    target_path: String,
    items_state: State<Vec<Item>>,
) -> Element {
    let t = use_app_theme();
    let mut cp = current_path.clone();
    let tp = target_path.clone();
    let is_active = *cp.read() == tp;

    let bg = if is_active { t.bg_active } else { Color::TRANSPARENT };

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .padding(10.)
        .corner_radius(12.)
        .background(bg)
        .margin((0., 0., 8., 0.))
        .on_press(move |_| {
            cp.set(tp.clone());
            load_dir(tp.clone(), items_state.clone());
        })
        .child(rect().margin((0., 12., 0., 0.)).child(icon(icon_svg, 16., t.text_primary)))
        .child(
            label()
                .font_size(14.)
                .color(t.text_primary)
                .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .text(name.to_string()),
        )
        .into_element()
}

fn main() {
    launch(LaunchConfig::new().with_window(WindowConfig::new(app).with_title("Files").with_size(1100., 700.)))
}
