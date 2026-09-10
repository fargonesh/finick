#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use {
    config::ty::App,
    freya::prelude::*,
    std::{env, path::PathBuf},
    ui::*,
};

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

#[derive(Clone, Copy, PartialEq)]
enum IconSize {
    Small,
    Medium,
    Large,
}

impl IconSize {
    fn px(self) -> f32 {
        match self {
            Self::Small => 24.,
            Self::Medium => 32.,
            Self::Large => 48.,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Small => "S",
            Self::Medium => "M",
            Self::Large => "L",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Small => Self::Medium,
            Self::Medium => Self::Large,
            Self::Large => Self::Small,
        }
    }
}

fn places_path() -> PathBuf {
    let base = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
    base.join(".config").join("finick").join("places.json")
}

fn load_pinned() -> Vec<(String, String)> {
    let p = places_path();
    if let Ok(s) = std::fs::read_to_string(p) {
        if let Ok(v) = serde_json::from_str::<Vec<(String, String)>>(&s) {
            return v;
        }
    }
    Vec::new()
}

fn save_pinned(v: &[(String, String)]) {
    let p = places_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(v) {
        let _ = std::fs::write(p, s);
    }
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
        if (result.is_empty() || res.is_err())
            && let Ok(entries) = std::fs::read_dir(&path)
        {
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

fn do_paste(clip: Option<(String, bool)>, dest_dir: String, items: State<Vec<Item>>) {
    if let Some((src, is_cut)) = clip {
        let src_path = PathBuf::from(&src);
        let name = src_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("file".to_string());
        let dest = PathBuf::from(&dest_dir).join(&name);
        if is_cut {
            let _ = std::fs::rename(&src, &dest);
        } else {
            if src_path.is_dir() {
                let _ = copy_dir_recursive(&src_path, &dest);
            } else {
                let _ = std::fs::copy(&src, &dest);
            }
        }
        load_dir(dest_dir, items);
    }
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let ty = e.file_type()?;
        let dst_path = dst.join(e.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&e.path(), &dst_path)?;
        } else {
            std::fs::copy(e.path(), dst_path)?;
        }
    }
    Ok(())
}

fn app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let current_path = use_state(|| env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()));
    let items = use_state(Vec::<Item>::new);
    let selected_item = use_state(|| Option::<Item>::None);
    let view_mode = use_state(|| ViewMode::Grid);
    let icon_size = use_state(|| IconSize::Large);
    let uri_input = use_state(String::new);
    let search_query = use_state(String::new);
    let pinned = use_state(load_pinned);
    let show_add_place = use_state(|| false);
    let add_place_name = use_state(String::new);
    let rename_target = use_state(|| Option::<Item>::None);
    let rename_input = use_state(String::new);
    let clipboard = use_state(|| Option::<(String, bool)>::None);
    let show_rename = use_state(|| false);
    let show_sidebar = use_state(|| true);
    let last_click: State<Option<(String, std::time::Instant)>> = use_state(|| None);
    let drag_start: State<Option<(f64, f64)>> = use_state(|| None);
    let drag_current: State<Option<(f64, f64)>> = use_state(|| None);
    let last_cp = use_state(|| env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()));
    let loaded = use_state(|| false);

    let cp = current_path.read().clone();
    if *last_cp.read() != cp {
        let mut lcp = last_cp;
        let mut uin = uri_input;
        lcp.set(cp.clone());
        uin.set(cp.clone());
    }

    if !*loaded.read() {
        let mut l = loaded;
        l.set(true);
        load_dir(current_path.read().clone(), items);
    }

    let pinned_val = pinned.read().clone();
    if !pinned_val.is_empty() {
        let _ = save_pinned(&pinned_val);
    }

    let sidebar = rect()
        .width(Size::px(220.))
        .height(Size::fill())
        .background(t.bg_sidebar)
        .border(Border::new().width(1.).fill(t.border_card))
        .padding(12.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .main_align(Alignment::SpaceBetween)
                .margin((0., 0., 12., 0.))
                .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("PLACES"))
                .child(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((2., 6.))
                        .corner_radius(6.)
                        .on_press({
                            let mut s = show_sidebar;
                            move |_| s.set(false)
                        })
                        .child(label().font_size(10.).color(t.text_muted).text("Collapse")),
                ),
        )
        .child(sidebar_entry(
            "Home",
            HOME,
            &current_path,
            env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items,
            pinned,
            clipboard,
        ))
        .child(sidebar_entry(
            "Documents",
            DOCUMENT,
            &current_path,
            env::home_dir().map(|v| v.join("Documents").to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items,
            pinned,
            clipboard,
        ))
        .child(sidebar_entry(
            "Downloads",
            DOWNLOAD,
            &current_path,
            env::home_dir().map(|v| v.join("Downloads").to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items,
            pinned,
            clipboard,
        ))
        .child(sidebar_entry(
            "Pictures",
            PICTURE,
            &current_path,
            env::home_dir().map(|v| v.join("Pictures").to_str().unwrap().to_string()).unwrap_or("/".to_string()),
            items,
            pinned,
            clipboard,
        ))
        .child(if !pinned.read().is_empty() {
            rect()
                .margin((12., 0., 0., 0.))
                .content(Content::Flex)
                .child(
                    label()
                        .font_size(11.)
                        .font_weight(FontWeight::BOLD)
                        .color(t.text_muted)
                        .margin((0., 0., 8., 0.))
                        .text("PINNED"),
                )
                .children(
                    pinned
                        .read()
                        .iter()
                        .map(|(name, path)| sidebar_entry(name, PIN, &current_path, path.clone(), items, pinned, clipboard)),
                )
                .into_element()
        } else {
            rect().into_element()
        })
        .child(
            rect().margin((12., 0., 0., 0.)).child(
                rect()
                    .width(Size::fill())
                    .padding(8.)
                    .corner_radius(8.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .cursor(CursorIcon::Pointer)
                    .on_press({
                        let mut show = show_add_place;
                        let mut name_st = add_place_name;
                        let cp2 = current_path.read().clone();
                        move |_| {
                            let base = PathBuf::from(&cp2)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or("Place".to_string());
                            name_st.set(base);
                            show.set(true);
                        }
                    })
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(6.)
                            .content(Content::Flex)
                            .child(icon(PLUS, 12., t.text_primary))
                            .child(label().font_size(12.).color(t.text_primary).text("Add Place")),
                    ),
            ),
        );

    let top_bar = rect()
        .height(Size::px(56.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .padding(10.)
        .spacing(8.)
        .background(t.bg_base)
        .border(Border::new().width(1.).fill(t.border_card))
        .child(
            rect()
                .padding(8.)
                .corner_radius(8.)
                .background(if *show_sidebar.read() { t.bg_card } else { t.bg_surface })
                .border(Border::new().width(1.).fill(t.border_card))
                .cursor(CursorIcon::Pointer)
                .on_press({
                    let mut s = show_sidebar;
                    move |_| s.set(!*s.read())
                })
                .child(icon(SIDEBAR, 16., t.text_primary)),
        )
        .child(
            rect()
                .padding(8.)
                .corner_radius(8.)
                .background(t.bg_card)
                .border(Border::new().width(1.).fill(t.border_card))
                .cursor(CursorIcon::Pointer)
                .on_press({
                    let mut current = current_path;
                    let items = items;
                    move |_| {
                        let path = PathBuf::from(current.read().clone());
                        if let Some(parent) = path.parent() {
                            let new_path = parent.to_string_lossy().to_string();
                            current.set(new_path.clone());
                            load_dir(new_path, items);
                        }
                    }
                })
                .child(icon(ARROW_LEFT, 16., t.text_primary)),
        )
        .child(
            rect()
                .width(Size::flex(1.))
                .height(Size::px(36.))
                .cross_align(Alignment::Center)
                .content(Content::Flex)
                .background(t.bg_card)
                .border(Border::new().width(1.).fill(t.border_card))
                .corner_radius(8.)
                .padding((2., 10.))
                .child(Input::new(uri_input).width(Size::fill()).flat().placeholder("Path").on_submit({
                    let mut current = current_path;
                    let items = items;
                    move |val: String| {
                        current.set(val.clone());
                        load_dir(val, items);
                    }
                })),
        )
        .child(
            rect()
                .horizontal()
                .main_align(Alignment::End)
                .cross_align(Alignment::Center)
                .content(Content::Flex)
                .spacing(6.)
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .spacing(4.)
                        .child(Input::new(search_query).width(Size::px(140.)).placeholder("Search").on_submit({
                            let items = items;
                            move |val: String| {
                                perform_search(val, items);
                            }
                        }))
                        .child(
                            rect()
                                .padding(8.)
                                .corner_radius(8.)
                                .background(t.bg_card)
                                .cursor(CursorIcon::Pointer)
                                .on_press({
                                    let items = items;
                                    let query = search_query;
                                    move |_| {
                                        perform_search(query.read().clone(), items);
                                    }
                                })
                                .child(icon(SEARCH, 14., t.text_primary)),
                        ),
                )
                .child(
                    rect()
                        .padding(6.)
                        .corner_radius(8.)
                        .background(t.bg_card)
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut s = icon_size;
                            move |_| s.set(s.read().next())
                        })
                        .child(label().font_size(11.).color(t.text_primary).text(icon_size.read().label().to_string())),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(if *view_mode.read() == ViewMode::Grid { t.bg_active } else { t.bg_card })
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut vm = view_mode;
                            move |_| vm.set(ViewMode::Grid)
                        })
                        .child(icon(
                            LAYOUT_GRID,
                            14.,
                            if *view_mode.read() == ViewMode::Grid { t.text_primary } else { t.text_muted },
                        )),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(if *view_mode.read() == ViewMode::List { t.bg_active } else { t.bg_card })
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut vm = view_mode;
                            move |_| vm.set(ViewMode::List)
                        })
                        .child(icon(
                            LIST,
                            14.,
                            if *view_mode.read() == ViewMode::List { t.text_primary } else { t.text_muted },
                        )),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(t.primary_accent)
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let path_state = current_path;
                            let items_state = items;
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
                                load_dir(path_state.read().clone(), items_state);
                            }
                        })
                        .child(icon(PLUS, 14., t.bg_base)),
                ),
        );

    let is_grid = *view_mode.read() == ViewMode::Grid;
    let items_read = items.read().clone();
    let icon_px = icon_size.read().px();

    let drag_box: Element = if let (Some((x1, y1)), Some((x2, y2))) = (*drag_start.read(), *drag_current.read()) {
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);
        let w = max_x - min_x;
        let h = max_y - min_y;
        if w > 3.0 || h > 3.0 {
            rect()
                .position(Position::new_absolute().top(min_y as f32).left(min_x as f32))
                .width(Size::px(w as f32))
                .height(Size::px(h as f32))
                .background(Color::from_argb(40, 50, 120, 240))
                .border(Border::new().width(1.).fill(Color::from_argb(180, 50, 120, 240)))
                .corner_radius(2.)
                .into_element()
        } else {
            rect().into_element()
        }
    } else {
        rect().into_element()
    };

    let items_view = ScrollView::new().width(Size::fill()).height(Size::fill()).child(
        rect()
            .direction(if is_grid { Direction::Horizontal } else { Direction::Vertical })
            .width(Size::fill())
            .padding(12.)
            .content(if is_grid { Content::Wrap { wrap_spacing: Some(6.0) } } else { Content::Flex })
            .on_pointer_down({
                let mut ds = drag_start;
                let mut dc = drag_current;
                let mut sel = selected_item;
                move |e: Event<PointerEventData>| {
                    let p = e.data().element_location();
                    ds.set(Some((p.x as f64, p.y as f64)));
                    dc.set(Some((p.x as f64, p.y as f64)));
                    sel.set(None);
                }
            })
            .on_global_pointer_move({
                let ds = drag_start;
                let mut dc = drag_current;
                move |e: Event<PointerEventData>| {
                    if ds.read().is_some() {
                        let p = e.data().element_location();
                        dc.set(Some((p.x as f64, p.y as f64)));
                    }
                }
            })
            .on_global_pointer_press({
                let mut ds = drag_start;
                let mut dc = drag_current;
                move |_| {
                    ds.set(None);
                    dc.set(None);
                }
            })
            .child(drag_box)
            .children(items_read.into_iter().map(|item| {
                let path_state = current_path;
                let items_state = items;
                let sel_state = selected_item;
                let mut rename_t = rename_target;
                let mut rename_in = rename_input;
                let mut show_rn = show_rename;
                let path = item.path.clone();
                let is_folder = item.ty == ItemType::Folder;
                let is_selected = sel_state.read().as_ref().is_some_and(|s| s.path == path);
                let item_clone = item.clone();
                let item_for_menu = item.clone();
                let bg = if is_selected { t.bg_selected } else { Color::TRANSPARENT };
                let i_svg = file_icon_for(&item.name, is_folder);
                let is_img = !is_folder && is_image_ext(&item.name);

                let ctx_menu = {
                    let path_c = item_for_menu.path.clone();
                    let name_c = item_for_menu.name.clone();
                    let is_folder_c = is_folder;
                    let cp_c = current_path;
                    let items_c = items;
                    let sel_c = selected_item;
                    let mut clip_c = clipboard;
                    Menu::new()
                        .child(
                            MenuButton::new()
                                .on_press({
                                    let pc = path_c.clone();
                                    let mut ps = path_state;
                                    let is2 = is_folder_c;
                                    move |_| {
                                        if is2 {
                                            ps.set(pc.clone());
                                            load_dir(pc.clone(), items_c);
                                        } else {
                                            let _ = std::process::Command::new("xdg-open").arg(&pc).spawn();
                                        }
                                    }
                                })
                                .child(if is_folder_c { "Open" } else { "Open File" }),
                        )
                        .child(
                            MenuButton::new()
                                .on_press({
                                    let pc = path_c.clone();
                                    move |_| {
                                        clip_c.set(Some((pc.clone(), true)));
                                    }
                                })
                                .child("Cut"),
                        )
                        .child(
                            MenuButton::new()
                                .on_press({
                                    let pc = path_c.clone();
                                    move |_| {
                                        clip_c.set(Some((pc.clone(), false)));
                                    }
                                })
                                .child("Copy"),
                        )
                        .child(
                            MenuButton::new()
                                .on_press({
                                    let pc = if is_folder_c { path_c.clone() } else { cp_c.read().clone() };
                                    let clip_v = clip_c.read().clone();
                                    move |_| {
                                        do_paste(clip_v.clone(), pc.clone(), items_c);
                                        if clip_v.as_ref().is_some_and(|(_, k)| *k) {
                                            clip_c.set(None);
                                        }
                                    }
                                })
                                .child("Paste"),
                        )
                        .child(
                            MenuButton::new()
                                .on_press(move |_| {
                                    rename_in.set(name_c.clone());
                                    rename_t.set(Some(item_for_menu.clone()));
                                    show_rn.set(true);
                                })
                                .child("Rename"),
                        )
                        .child(
                            MenuButton::new()
                                .on_press({
                                    let pc = path_c.clone();
                                    let mut sc = sel_c;
                                    move |_| {
                                        if is_folder_c {
                                            let _ = std::fs::remove_dir_all(&pc);
                                        } else {
                                            let _ = std::fs::remove_file(&pc);
                                        }
                                        sc.set(None);
                                        load_dir(cp_c.read().clone(), items_c);
                                    }
                                })
                                .child("Delete"),
                        )
                };

                if is_grid {
                    let thumb: Element = if is_img {
                        rect()
                            .width(Size::px(icon_px + 16.))
                            .height(Size::px(icon_px + 16.))
                            .corner_radius(8.)
                            .overflow(Overflow::Clip)
                            .background(t.bg_card)
                            .center()
                            .child(
                                ImageViewer::new(PathBuf::from(path.clone()))
                                    .width(Size::px(icon_px + 12.))
                                    .height(Size::px(icon_px + 12.)),
                            )
                            .into_element()
                    } else {
                        rect()
                            .width(Size::px(icon_px + 16.))
                            .height(Size::px(icon_px + 16.))
                            .center()
                            .child(icon(i_svg, icon_px, if is_folder { t.primary_accent } else { t.text_secondary }))
                            .into_element()
                    };
                    rect()
                        .vertical()
                        .width(Size::px(120.))
                        .height(Size::px(140.))
                        .padding(8.)
                        .spacing(6.)
                        .margin(6.)
                        .corner_radius(12.)
                        .background(bg)
                        .cursor(CursorIcon::Pointer)
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::Center)
                        .content(Content::Flex)
                        .on_pointer_enter({
                            let ds = drag_start;
                            let mut ss = sel_state;
                            let it = item_clone.clone();
                            move |_| {
                                if ds.read().is_some() {
                                    ss.set(Some(it.clone()));
                                }
                            }
                        })
                        .on_press({
                            let mut lc = last_click;
                            let mut ps = path_state;
                            let mut ss = sel_state;
                            let p = path.clone();
                            let it = item_clone.clone();
                            let is_f = is_folder;
                            move |_| {
                                let now = std::time::Instant::now();
                                let is_double = if let Some((ref lp, lt)) = *lc.read() {
                                    lp == &p && now.duration_since(lt).as_millis() < 450
                                } else {
                                    false
                                };
                                if is_double {
                                    lc.set(None);
                                    if is_f {
                                        ps.set(p.clone());
                                        load_dir(p.clone(), items_state);
                                    } else {
                                        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
                                    }
                                } else {
                                    lc.set(Some((p.clone(), now)));
                                    ss.set(Some(it.clone()));
                                }
                            }
                        })
                        .on_secondary_down({
                            let mut ss = sel_state;
                            let it = item_clone.clone();
                            let m = ctx_menu.clone();
                            move |_| {
                                ss.set(Some(it.clone()));
                                ContextMenu::open_from_down(m.clone());
                            }
                        })
                        .child(thumb)
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_primary)
                                .text_align(TextAlign::Center)
                                .text(item.name.clone()),
                        )
                        .into_element()
                } else {
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .padding(8.)
                        .corner_radius(8.)
                        .background(bg)
                        .cursor(CursorIcon::Pointer)
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .spacing(8.)
                        .on_pointer_enter({
                            let ds = drag_start;
                            let mut ss = sel_state;
                            let it = item_clone.clone();
                            move |_| {
                                if ds.read().is_some() {
                                    ss.set(Some(it.clone()));
                                }
                            }
                        })
                        .on_press({
                            let mut lc = last_click;
                            let mut ps = path_state;
                            let mut ss = sel_state;
                            let p = path.clone();
                            let it = item_clone.clone();
                            let is_f = is_folder;
                            move |_| {
                                let now = std::time::Instant::now();
                                let is_double = if let Some((ref lp, lt)) = *lc.read() {
                                    lp == &p && now.duration_since(lt).as_millis() < 450
                                } else {
                                    false
                                };
                                if is_double {
                                    lc.set(None);
                                    if is_f {
                                        ps.set(p.clone());
                                        load_dir(p.clone(), items_state);
                                    } else {
                                        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
                                    }
                                } else {
                                    lc.set(Some((p.clone(), now)));
                                    ss.set(Some(it.clone()));
                                }
                            }
                        })
                        .on_secondary_down({
                            let mut ss = sel_state;
                            let it = item_clone.clone();
                            let m = ctx_menu.clone();
                            move |_| {
                                ss.set(Some(it.clone()));
                                ContextMenu::open_from_down(m.clone());
                            }
                        })
                        .child(icon(i_svg, 20., if is_folder { t.primary_accent } else { t.text_secondary }))
                        .child(
                            rect()
                                .width(Size::fill())
                                .content(Content::Flex)
                                .child(label().font_size(13.).color(t.text_primary).text(item.name.clone())),
                        )
                        .child(
                            label()
                                .font_size(11.)
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
        let is_img_sel = !is_folder && is_image_ext(&sel.name);
        rect()
            .width(Size::px(280.))
            .height(Size::fill())
            .background(t.bg_surface)
            .padding(16.)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .main_align(Alignment::SpaceBetween)
                    .cross_align(Alignment::Center)
                    .margin((0., 0., 12., 0.))
                    .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("PROPERTIES"))
                    .child({
                        let mut sel_state = selected_item;
                        rect()
                            .cursor(CursorIcon::Pointer)
                            .padding((2., 6.))
                            .corner_radius(4.)
                            .background(t.bg_card)
                            .on_press(move |_| sel_state.set(None))
                            .child(label().font_size(11.).color(t.text_muted).text("✕"))
                    }),
            )
            .child({
                if is_img_sel {
                    let path_buf = PathBuf::from(sel.path.clone());
                    rect()
                        .width(Size::fill())
                        .height(Size::px(160.))
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::Center)
                        .content(Content::Flex)
                        .margin((0., 0., 16., 0.))
                        .corner_radius(12.)
                        .background(t.bg_card)
                        .overflow(Overflow::Clip)
                        .child(ImageViewer::new(path_buf).width(Size::fill()).height(Size::fill()))
                        .into_element()
                } else {
                    rect()
                        .width(Size::fill())
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .margin((0., 0., 16., 0.))
                        .child(icon(
                            file_icon_for(&sel.name, is_folder),
                            48.,
                            if is_folder { t.primary_accent } else { t.text_secondary },
                        ))
                        .into_element()
                }
            })
            .child(
                label()
                    .font_size(16.)
                    .font_weight(FontWeight::BOLD)
                    .color(t.text_primary)
                    .margin((0., 0., 4., 0.))
                    .text(sel.name.clone()),
            )
            .child(label().font_size(11.).color(t.text_muted).margin((0., 0., 12., 0.)).text(format_size(sel.size)))
            .child(
                rect()
                    .margin((0., 0., 12., 0.))
                    .child(label().font_size(10.).color(t.text_muted).text("PATH"))
                    .child(label().font_size(11.).color(t.text_primary).text(sel.path.clone())),
            )
            .child(
                rect()
                    .margin((0., 0., 12., 0.))
                    .child(label().font_size(10.).color(t.text_muted).text("TYPE"))
                    .child(label().font_size(11.).color(t.text_primary).text(if is_folder { "Directory" } else { "File" })),
            )
            .child({
                let s_path = sel.path.clone();
                let mut path_state = current_path;
                let items_state = items;
                rect()
                    .margin((12., 0., 0., 0.))
                    .child(primary_button(if is_folder { "Open Directory" } else { "Open File" }, move || {
                        if is_folder {
                            path_state.set(s_path.clone());
                            load_dir(s_path.clone(), items_state);
                        } else {
                            let _ = std::process::Command::new("xdg-open").arg(&s_path).spawn();
                        }
                    }))
                    .into_element()
            })
            .child({
                if is_folder {
                    let mut pin_state = pinned;
                    let s_name = sel.name.clone();
                    let s_path = sel.path.clone();
                    let is_pinned = pin_state.read().iter().any(|(_, p)| p == &s_path);
                    rect()
                        .margin((8., 0., 0., 0.))
                        .child(secondary_button(
                            if is_pinned { "Unpin from Sidebar" } else { "Pin to Sidebar" },
                            move || {
                                let mut current = pin_state.read().clone();
                                if is_pinned {
                                    current.retain(|(_, p)| p != &s_path);
                                } else {
                                    current.push((s_name.clone(), s_path.clone()));
                                }
                                save_pinned(&current);
                                pin_state.set(current);
                            },
                        ))
                        .into_element()
                } else {
                    rect().into_element()
                }
            })
            .child({
                let mut clip = clipboard;
                let s_path = sel.path.clone();
                rect()
                    .horizontal()
                    .spacing(8.)
                    .margin((8., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(secondary_button("Cut", {
                                let p = s_path.clone();
                                move || clip.set(Some((p.clone(), true)))
                            }))
                            .content(Content::Flex),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(secondary_button("Copy", {
                                let p = s_path.clone();
                                move || clip.set(Some((p.clone(), false)))
                            }))
                            .content(Content::Flex),
                    )
                    .into_element()
            })
            .child({
                let s_path = sel.path.clone();
                let path_state = current_path;
                let items_state = items;
                let mut sel_state = selected_item;
                let mut rt = rename_target;
                let mut ri = rename_input;
                let mut sr = show_rename;
                let name_c = sel.name.clone();
                rect()
                    .horizontal()
                    .spacing(8.)
                    .margin((8., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(secondary_button("Rename", move || {
                                ri.set(name_c.clone());
                                rt.set(Some(sel.clone()));
                                sr.set(true);
                            }))
                            .content(Content::Flex),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(danger_button("Delete", move || {
                                if is_folder {
                                    let _ = std::fs::remove_dir_all(&s_path);
                                } else {
                                    let _ = std::fs::remove_file(&s_path);
                                }
                                sel_state.set(None);
                                load_dir(path_state.read().clone(), items_state);
                            }))
                            .content(Content::Flex),
                    )
                    .into_element()
            })
            .child({
                let clip_val = clipboard.read().clone();
                if clip_val.is_some() {
                    let mut clip = clipboard;
                    let dest = current_path.read().clone();
                    let items_state = items;
                    rect()
                        .margin((8., 0., 0., 0.))
                        .child(secondary_button("Paste Here", move || {
                            do_paste(clip_val.clone(), dest.clone(), items_state);
                            if clip_val.as_ref().is_some_and(|(_, k)| *k) {
                                clip.set(None);
                            }
                        }))
                        .into_element()
                } else {
                    rect().into_element()
                }
            })
            .into_element()
    } else {
        rect().width(Size::px(0.)).into_element()
    };

    let add_place_modal: Element = if *show_add_place.read() {
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(360.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.bg_surface)
                    .spacing(12.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Add Place"))
                    .child(Input::new(add_place_name).width(Size::fill()).placeholder("Name"))
                    .child(label().font_size(11.).color(t.text_muted).text(current_path.read().clone()))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .on_press({
                                        let mut s = show_add_place;
                                        move |_| s.set(false)
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Cancel")),
                            )
                            .child(
                                rect()
                                    .padding((8., 12.))
                                    .corner_radius(8.)
                                    .background(t.primary_accent)
                                    .on_press({
                                        let mut s = show_add_place;
                                        let mut pin = pinned;
                                        let name_s = add_place_name;
                                        let path_s = current_path;
                                        move |_| {
                                            let n = name_s.read().trim().to_string();
                                            if !n.is_empty() {
                                                let mut v = pin.read().clone();
                                                v.push((n, path_s.read().clone()));
                                                save_pinned(&v);
                                                pin.set(v);
                                            }
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.bg_base).text("Add")),
                            ),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    let rename_modal: Element = if *show_rename.read() {
        let _target = rename_target.read().clone();
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(360.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.bg_surface)
                    .spacing(12.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Rename"))
                    .child(Input::new(rename_input).width(Size::fill()).placeholder("New name").on_submit({
                        let mut show = show_rename;
                        let tgt = rename_target;
                        let inp = rename_input;
                        let cp = current_path;
                        let items_s = items;
                        let mut sel = selected_item;
                        move |val: String| {
                            if let Some(it) = tgt.read().clone() {
                                let new_path = PathBuf::from(&it.path)
                                    .parent()
                                    .unwrap_or(PathBuf::from("/").as_path())
                                    .join(val.trim());
                                let _ = std::fs::rename(&it.path, &new_path);
                                sel.set(None);
                                load_dir(cp.read().clone(), items_s);
                            }
                            let _ = inp;
                            show.set(false);
                        }
                    }))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .on_press({
                                        let mut s = show_rename;
                                        move |_| s.set(false)
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Cancel")),
                            )
                            .child(
                                rect()
                                    .padding((8., 12.))
                                    .corner_radius(8.)
                                    .background(t.primary_accent)
                                    .on_press({
                                        let mut s = show_rename;
                                        let tgt = rename_target;
                                        let inp = rename_input;
                                        let cp = current_path;
                                        let items_s = items;
                                        let mut sel = selected_item;
                                        move |_| {
                                            if let Some(it) = tgt.read().clone() {
                                                let new_name = inp.read().trim().to_string();
                                                if !new_name.is_empty() {
                                                    let new_path = PathBuf::from(&it.path)
                                                        .parent()
                                                        .unwrap_or(PathBuf::from("/").as_path())
                                                        .join(&new_name);
                                                    let _ = std::fs::rename(&it.path, &new_path);
                                                    sel.set(None);
                                                    load_dir(cp.read().clone(), items_s);
                                                }
                                            }
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.bg_base).text("Rename")),
                            ),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(t.bg_base)
        .on_global_key_down({
            let mut clip = clipboard;
            let mut sel = selected_item;
            let cp2 = current_path;
            let items2 = items;
            let mut rename_t = rename_target;
            let mut rename_in = rename_input;
            let mut show_rn = show_rename;
            move |e: Event<KeyboardEventData>| {
                if e.data().modifiers.ctrl() {
                    match e.data().key {
                        Key::Character(ref s) if s == "c" => {
                            if let Some(it) = sel.read().clone() {
                                clip.set(Some((it.path, false)));
                            }
                        }
                        Key::Character(ref s) if s == "x" => {
                            if let Some(it) = sel.read().clone() {
                                clip.set(Some((it.path, true)));
                            }
                        }
                        Key::Character(ref s) if s == "v" => {
                            do_paste(clip.read().clone(), cp2.read().clone(), items2);
                            if clip.read().as_ref().is_some_and(|(_, is_cut)| *is_cut) {
                                clip.set(None);
                            }
                        }
                        _ => {}
                    }
                } else {
                    match &e.data().key {
                        Key::Named(NamedKey::F2) => {
                            if let Some(it) = sel.read().clone() {
                                rename_in.set(it.name.clone());
                                rename_t.set(Some(it));
                                show_rn.set(true);
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            if let Some(it) = sel.read().clone() {
                                if it.ty == ItemType::Folder {
                                    let _ = std::fs::remove_dir_all(&it.path);
                                } else {
                                    let _ = std::fs::remove_file(&it.path);
                                }
                                sel.set(None);
                                load_dir(cp2.read().clone(), items2);
                            }
                        }
                        _ => {}
                    }
                }
            }
        })
        .child(ContextMenuViewer::new())
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::fill())
                .content(Content::Flex)
                .background(t.bg_base)
                .maybe(*show_sidebar.read(), |el| el.child(sidebar))
                .child(
                    rect()
                        .vertical()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .content(Content::Flex)
                        .child(top_bar)
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::fill())
                                .content(Content::Flex)
                                .on_secondary_down({
                                    let cp_c = current_path.read().clone();
                                    let clip_v = clipboard.read().clone();
                                    let items_c = items;
                                    let mut clip = clipboard;
                                    move |_| {
                                        if let Some((..)) = clip_v.clone() {
                                            let dest = cp_c.clone();
                                            let cv = clip_v.clone();
                                            ContextMenu::open_from_down(
                                                Menu::new().child(
                                                    MenuButton::new()
                                                        .on_press({
                                                            let cv2 = cv.clone();
                                                            let dest2 = dest.clone();
                                                            move |_| {
                                                                do_paste(cv2.clone(), dest2.clone(), items_c);
                                                                if cv2.as_ref().is_some_and(|(_, k)| *k) {
                                                                    clip.set(None);
                                                                }
                                                            }
                                                        })
                                                        .child("Paste"),
                                                ),
                                            );
                                        }
                                    }
                                })
                                .child(items_view),
                        ),
                )
                .maybe_child(selected_item.read().is_some().then(|| drawer)),
        )
        .child(add_place_modal)
        .child(rename_modal)
        .into_element()
}

fn sidebar_entry(
    name: &str,
    icon_svg: &'static str,
    current_path: &State<String>,
    target_path: String,
    items_state: State<Vec<Item>>,
    pinned: State<Vec<(String, String)>>,
    _clipboard: State<Option<(String, bool)>>,
) -> Element {
    let t = use_app_theme();
    let mut cp = *current_path;
    let tp = target_path.clone();
    let is_active = *cp.read() == tp;
    let bg = if is_active { t.bg_active } else { Color::TRANSPARENT };
    let name_owned = name.to_string();
    let tp_for_menu = tp.clone();
    let ctx = Menu::new()
        .child(
            MenuButton::new()
                .on_press({
                    let tp2 = tp_for_menu.clone();
                    let mut cp2 = cp;
                    move |_| {
                        cp2.set(tp2.clone());
                        load_dir(tp2.clone(), items_state);
                    }
                })
                .child("Open"),
        )
        .child(
            MenuButton::new()
                .on_press({
                    let tp2 = tp_for_menu.clone();
                    let name2 = name_owned.clone();
                    let mut pin = pinned;
                    move |_| {
                        let mut v = pin.read().clone();
                        if v.iter().any(|(_, p)| p == &tp2) {
                            v.retain(|(_, p)| p != &tp2);
                        } else {
                            v.push((name2.clone(), tp2.clone()));
                        }
                        save_pinned(&v);
                        pin.set(v);
                    }
                })
                .child(if pinned.read().iter().any(|(_, p)| p == &tp_for_menu) { "Unpin" } else { "Pin" }),
        );
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .spacing(8.)
        .padding(8.)
        .corner_radius(8.)
        .background(bg)
        .margin((0., 0., 4., 0.))
        .on_press(move |_| {
            cp.set(tp.clone());
            load_dir(tp.clone(), items_state);
        })
        .on_secondary_down(move |_| ContextMenu::open_from_down(ctx.clone()))
        .child(icon(icon_svg, 14., t.text_primary))
        .child(
            label()
                .font_size(13.)
                .color(t.text_primary)
                .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .text(name_owned.clone()),
        )
        .into_element()
}

fn main() { launch(LaunchConfig::new().with_window(WindowConfig::new(app).with_title("Files").with_size(1100., 700.))) }
