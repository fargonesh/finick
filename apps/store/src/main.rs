#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use {
    freya::prelude::*,
    system::store::{StoreEntry, StoreSource},
    ui::*,
};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    Discover,
    Installed,
    Updates,
}

fn tab_title(tab: Tab) -> &'static str {
    match tab {
        Tab::Discover => "Discover",
        Tab::Installed => "Installed",
        Tab::Updates => "Updates",
    }
}

fn run_search(query: String, mut results: State<Vec<StoreEntry>>, mut busy: State<bool>, mut notice: State<String>) {
    busy.set(true);
    notice.set("Searching Flathub + nixpkgs…".to_string());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let hits = system::store::search_all(&query);
        let _ = tx.send(hits);
    });
    spawn(async move {
        if let Some(hits) = rx.recv().await {
            let n = hits.len();
            results.set(hits);
            busy.set(false);
            notice.set(if n == 0 { "No results — try another query".to_string() } else { format!("{n} results") });
        }
    });
}

fn refresh_installed(mut installed: State<Vec<StoreEntry>>) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let _ = tx.send(system::store::list_installed());
    });
    spawn(async move {
        if let Some(items) = rx.recv().await {
            installed.set(items);
        }
    });
}

fn refresh_updates(mut updates: State<Vec<system::store_flatpak::FlatpakApp>>) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let _ = tx.send(system::store_flatpak::pending_updates());
    });
    spawn(async move {
        if let Some(items) = rx.recv().await {
            updates.set(items);
        }
    });
}

fn do_install(entry: StoreEntry, mut notice: State<String>, mut busy: State<bool>, installed: State<Vec<StoreEntry>>) {
    busy.set(true);
    notice.set(format!("Installing {}…", entry.name));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let msg = match entry.source {
            StoreSource::Flathub => system::store_flatpak::install(&entry.id)
                .map(|m| {
                    let _ = system::store_hm::add_flatpak(&entry.id);
                    m
                })
                .unwrap_or_else(|e| e),
            StoreSource::Nixpkgs => {
                let attr = entry.id.trim_start_matches("nixpkgs#");
                system::store::install_nix_reproducible(attr).unwrap_or_else(|e| e)
            }
            StoreSource::System => "Already installed".to_string(),
        };
        let _ = tx.send(msg);
    });
    spawn(async move {
        if let Some(msg) = rx.recv().await {
            notice.set(msg);
            busy.set(false);
            refresh_installed(installed);
        }
    });
}

fn do_remove(entry: StoreEntry, mut notice: State<String>, mut busy: State<bool>, installed: State<Vec<StoreEntry>>) {
    busy.set(true);
    notice.set(format!("Removing {}…", entry.name));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let msg = match entry.source {
            StoreSource::Flathub => system::store_flatpak::remove(&entry.id)
                .map(|m| {
                    let _ = system::store_hm::remove_flatpak(&entry.id);
                    m
                })
                .unwrap_or_else(|e| e),
            StoreSource::Nixpkgs => {
                let attr = entry.id.trim_start_matches("nixpkgs#");
                system::store::remove_nix_reproducible(attr).unwrap_or_else(|e| e)
            }
            StoreSource::System => "System apps are managed by your OS image".to_string(),
        };
        let _ = tx.send(msg);
    });
    spawn(async move {
        if let Some(msg) = rx.recv().await {
            notice.set(msg);
            busy.set(false);
            refresh_installed(installed);
        }
    });
}

fn select_entry(
    entry: StoreEntry,
    mut selected: State<Option<StoreEntry>>,
    mut nix_detail: State<Option<system::store_nix::NixPackage>>,
    mut perms: State<Vec<system::store_flatpak::FlatpakPermission>>,
) {
    selected.set(Some(entry.clone()));
    nix_detail.set(None);
    perms.set(Vec::new());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let _ = tx.send(system::store::details_for(&entry));
    });
    spawn(async move {
        if let Some(d) = rx.recv().await {
            nix_detail.set(d.nix_meta);
            perms.set(d.flatpak_perms);
        }
    });
}

fn source_label(source: &StoreSource) -> &'static str {
    match source {
        StoreSource::Flathub => "Flathub",
        StoreSource::Nixpkgs => "nixpkgs",
        StoreSource::System => "System",
    }
}

fn app_initial(name: &str) -> String {
    name.trim().chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string())
}

fn app_row_icon(name: &str, active: bool) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .width(Size::px(38.))
        .height(Size::px(38.))
        .corner_radius(11.)
        .background(if active { t.panel } else { t.bg })
        .border(Border::new().width(1.).fill(t.border))
        .center()
        .child(
            label()
                .font_size(15.)
                .font_weight(FontWeight::BOLD)
                .color(if active { t.accent } else { t.text_dim })
                .text(app_initial(name)),
        )
}

fn store_app() -> impl IntoElement {
    let _theme = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let tab = use_state(|| Tab::Discover);
    let query = use_state(String::new);
    let results = use_state(Vec::<StoreEntry>::new);
    let installed = use_state(Vec::<StoreEntry>::new);
    let updates = use_state(Vec::<system::store_flatpak::FlatpakApp>::new);
    let notice = use_state(|| "Search Flathub and nixpkgs to get started".to_string());
    let busy = use_state(|| false);
    let hm_file = use_state(|| system::store_hm::resolve_home_manager_file().unwrap_or_default());
    let hm_imported = use_state(|| false);
    let selected = use_state(|| None::<StoreEntry>);
    let nix_detail = use_state(|| None::<system::store_nix::NixPackage>);
    let perms = use_state(Vec::<system::store_flatpak::FlatpakPermission>::new);

    {
        let installed = installed;
        let updates = updates;
        let hm_file = hm_file;
        let mut hm_imported = hm_imported;
        use_hook(move || {
            refresh_installed(installed);
            refresh_updates(updates);
            let f = hm_file.read().clone();
            if !f.is_empty() {
                hm_imported.set(system::store_hm::is_imported(&f));
            }
        });
    }

    let tab_val = *tab.read();
    let busy_val = *busy.read();
    let notice_val = notice.read().clone();
    let installed_count = installed.read().len();
    let updates_count = updates.read().len();
    let detail = selected.read().clone();

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .background(t.bg)
        .overflow(Overflow::Clip)
        .child(
            rect()
                .width(Size::px(232.))
                .height(Size::fill())
                .background(t.sidebar_bg)
                .border(Border::new().width(1.).fill(t.border))
                .padding((16., 12., 18., 12.))
                .vertical()
                .spacing(4.)
                .child(rect().cursor(CursorIcon::Pointer).child(brand_row("F", "Finick", "Apps")))
                .child(nav_group_label("Store"))
                .child({
                    let mut tab = tab;
                    nav_item(APPS, "Discover", tab_val == Tab::Discover, move || tab.set(Tab::Discover))
                })
                .child({
                    let mut tab = tab;
                    let installed = installed;
                    nav_item(FOLDER, "Installed", tab_val == Tab::Installed, move || {
                        tab.set(Tab::Installed);
                        refresh_installed(installed);
                    })
                })
                .child({
                    let mut tab = tab;
                    let updates = updates;
                    nav_item(
                        GENERAL,
                        format!("Updates{}", if updates_count > 0 { format!(" ({updates_count})") } else { String::new() }),
                        tab_val == Tab::Updates,
                        move || {
                            tab.set(Tab::Updates);
                            refresh_updates(updates);
                        },
                    )
                })
                .child(
                    ScrollView::new().width(Size::fill()).height(Size::fill()).child(
                        rect()
                            .width(Size::fill())
                            .vertical()
                            .spacing(6.)
                            .child(nav_group_label("Home Manager"))
                            .child(
                                tile()
                                    .child(tile_head(
                                        None,
                                        "Reproducible",
                                        Some(if *hm_imported.read() {
                                            status_chip("Linked", true, None).into_element()
                                        } else {
                                            status_chip("Not linked", false, None).into_element()
                                        }),
                                    ))
                                    .child(field_label(if hm_file.read().is_empty() {
                                        "No Home Manager file found".to_string()
                                    } else {
                                        hm_file.read().rsplit('/').next().unwrap_or("home.nix").to_string()
                                    })),
                            )
                            .maybe(!*hm_imported.read(), |el| {
                                el.child({
                                    let hm_file = hm_file;
                                    let hm_imported = hm_imported;
                                    let notice = notice;
                                    secondary_button("Link apps.nix", move || {
                                        let f = hm_file.read().clone();
                                        let mut notice = notice;
                                        let mut hm_imported = hm_imported;
                                        if f.is_empty() {
                                            notice.set("home-manager file not found — add imports manually".to_string());
                                            return;
                                        }
                                        match system::store_hm::ensure_imported(&f) {
                                            Ok(()) => {
                                                hm_imported.set(true);
                                                notice.set("apps.nix linked — Apply to rebuild".to_string());
                                            }
                                            Err(e) => notice.set(e),
                                        }
                                    })
                                })
                            })
                            .child({
                                let notice = notice;
                                let busy = busy;
                                primary_button(if *busy.read() { "Applying…" } else { "Apply changes" }, move || {
                                    let mut notice = notice;
                                    let mut busy = busy;
                                    busy.set(true);
                                    notice.set("Applying home-manager switch…".to_string());
                                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                                    std::thread::spawn(move || {
                                        let _ = tx.send(system::store_hm::apply_home_manager());
                                    });
                                    spawn(async move {
                                        if let Some(res) = rx.recv().await {
                                            match res {
                                                Ok(log) => notice.set(format!("Applied. {log}")),
                                                Err(e) => {
                                                    notice.set(format!("Switch failed, kept nix profile fallback: {e}"))
                                                }
                                            }
                                            busy.set(false);
                                        }
                                    });
                                })
                            }),
                    ),
                ),
        )
        .child(
            rect()
                .width(Size::flex(1.))
                .height(Size::fill())
                .vertical()
                .content(Content::Flex)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(56.))
                        .padding((0., 24.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::SpaceBetween)
                        .border(Border::new().width(1.).fill(t.border))
                        .background(t.panel)
                        .content(Content::Flex)
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(10.)
                                .content(Content::Flex)
                                .child({
                                    if detail.is_some() {
                                        let mut selected = selected;
                                        rect()
                                            .cursor(CursorIcon::Pointer)
                                            .padding((4., 8.))
                                            .corner_radius(RADIUS_PILL)
                                            .background(t.panel_raised)
                                            .border(Border::new().width(1.).fill(t.border))
                                            .on_press(move |_| selected.set(None))
                                            .child(
                                                label()
                                                    .font_size(12.5)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .color(t.text)
                                                    .text("‹ Back"),
                                            )
                                            .into_element()
                                    } else {
                                        rect().into_element()
                                    }
                                })
                                .child(
                                    label()
                                        .font_size(15.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(t.text)
                                        .text(if detail.is_some() { "Details" } else { tab_title(tab_val) }),
                                ),
                        )
                        .child(label().font_size(12.).color(t.text_dim).text(if tab_val == Tab::Installed {
                            format!("{installed_count} installed")
                        } else if tab_val == Tab::Updates {
                            format!("{updates_count} updates")
                        } else if busy_val {
                            "Searching…".to_string()
                        } else {
                            String::new()
                        })),
                )
                .child(rect().width(Size::flex(1.)).height(Size::fill()).content(Content::Flex).child(
                    ScrollView::new().width(Size::fill()).height(Size::fill()).child(
                        rect().width(Size::fill()).padding((30., 34., 64., 34.)).vertical().spacing(GAP).child({
                            if let Some(entry) = detail.clone() {
                                let mut selected = selected;
                                let nix_meta = nix_detail.read().clone();
                                let perm_list = perms.read().clone();
                                let mut perms = perms;
                                let notice_c = notice;
                                let busy_c = busy;
                                let installed_c = installed;
                                let entry_c = entry.clone();
                                let entry_p = entry.clone();
                                tile()
                                    .child(page_head(
                                        APPS,
                                        entry.name.clone(),
                                        format!("{} · {}", source_label(&entry.source), entry.id),
                                    ))
                                    .child(setting_row(
                                        "Summary",
                                        Some(if entry.summary.is_empty() {
                                            "No description available"
                                        } else {
                                            entry.summary.as_str()
                                        }),
                                        false,
                                        status_chip(source_label(&entry.source), entry.installed, None),
                                    ))
                                    .maybe_child(nix_meta.map(|m| {
                                        rect()
                                            .width(Size::fill())
                                            .vertical()
                                            .spacing(8.)
                                            .child(tile_head(None, "Package info", None::<String>))
                                            .child(field_label(format!(
                                                "version {} · license {}",
                                                if m.version.is_empty() { "rolling" } else { m.version.as_str() },
                                                if m.license.is_empty() { "nixpkgs" } else { m.license.as_str() }
                                            )))
                                            .maybe(!m.homepage.is_empty(), |el| {
                                                el.child(label().font_size(12.).color(t.text_dim).text(m.homepage.clone()))
                                            })
                                            .child(label().font_size(12.5).color(t.text).text(
                                                if m.long_description.is_empty() {
                                                    m.description
                                                } else {
                                                    m.long_description
                                                },
                                            ))
                                            .into_element()
                                    }))
                                    .maybe(!perm_list.is_empty(), |el| {
                                        el.child(tile_head(None, "Permissions", Some("Flatpak sandbox".to_string()))).child(
                                            rect().width(Size::fill()).vertical().spacing(2.).children(
                                                perm_list.iter().cloned().map(|p| {
                                                    let mut notice = notice_c;
                                                    let app_id = entry_p.id.clone();
                                                    let key = p.key.clone();
                                                    let toggled = !p.allowed;
                                                    setting_row(
                                                        p.label.clone(),
                                                        Some(p.key.clone()),
                                                        false,
                                                        secondary_button(
                                                            if p.allowed { "Revoke" } else { "Allow" },
                                                            move || {
                                                                match system::store_flatpak::set_permission(
                                                                    &app_id, &key, toggled,
                                                                ) {
                                                                    Ok(m) => notice.set(m),
                                                                    Err(e) => notice.set(e),
                                                                }
                                                                perms.set(system::store_flatpak::permissions(&app_id));
                                                            },
                                                        ),
                                                    )
                                                    .into_element()
                                                }),
                                            ),
                                        )
                                    })
                                    .child(rect().width(Size::fill()).horizontal().spacing(8.).content(Content::Flex).child(
                                        if entry.source == StoreSource::System {
                                            status_chip("Preinstalled", false, None).into_element()
                                        } else if entry.installed {
                                            danger_button("Remove", move || {
                                                do_remove(entry_c.clone(), notice_c, busy_c, installed_c);
                                                selected.set(None);
                                            })
                                            .into_element()
                                        } else {
                                            primary_button("Install", move || {
                                                do_install(entry_c.clone(), notice_c, busy_c, installed_c);
                                            })
                                            .into_element()
                                        },
                                    ))
                                    .into_element()
                            } else {
                                match tab_val {
                                    Tab::Discover => rect()
                                        .width(Size::fill())
                                        .vertical()
                                        .spacing(GAP)
                                        .child(page_head(
                                            APPS,
                                            "Discover",
                                            "Flathub sandboxed apps and nixpkgs, reproducibly",
                                        ))
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .horizontal()
                                                .content(Content::Flex)
                                                .cross_align(Alignment::Center)
                                                .spacing(10.)
                                                .child(
                                                    rect()
                                                        .width(Size::flex(1.))
                                                        .height(Size::px(44.))
                                                        .corner_radius(RADIUS_PILL)
                                                        .background(t.panel)
                                                        .border(Border::new().width(1.).fill(t.border))
                                                        .padding((0., 12.))
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .content(Content::Flex)
                                                        .spacing(8.)
                                                        .child(icon(SEARCH, 14., t.text_dim))
                                                        .child(
                                                            Input::new(query)
                                                                .width(Size::fill())
                                                                .background(Color::TRANSPARENT)
                                                                .border_fill(Color::TRANSPARENT)
                                                                .focus_background(Color::TRANSPARENT)
                                                                .focus_border_fill(Color::TRANSPARENT)
                                                                .placeholder("Search Flathub + nixpkgs…")
                                                                .on_submit({
                                                                    let query = query;
                                                                    let results = results;
                                                                    let busy = busy;
                                                                    let notice = notice;
                                                                    move |_| {
                                                                        run_search(
                                                                            query.read().clone(),
                                                                            results,
                                                                            busy,
                                                                            notice,
                                                                        );
                                                                    }
                                                                }),
                                                        ),
                                                )
                                                .child({
                                                    let query = query;
                                                    let results = results;
                                                    let busy = busy;
                                                    let notice = notice;
                                                    primary_button(if busy_val { "…" } else { "Search" }, move || {
                                                        if *busy.read() {
                                                            return;
                                                        }
                                                        run_search(query.read().clone(), results, busy, notice);
                                                    })
                                                }),
                                        )
                                        .child(label().font_size(12.).color(t.text_dim).text(notice_val))
                                        .child({
                                            let items = results.read().clone();
                                            if items.is_empty() {
                                                rect()
                                                    .width(Size::fill())
                                                    .padding((32., 0.))
                                                    .vertical()
                                                    .cross_align(Alignment::Center)
                                                    .spacing(8.)
                                                    .child(icon(APPS, 28., t.text_dim))
                                                    .child(
                                                        label()
                                                            .font_size(13.)
                                                            .color(t.text_dim)
                                                            .text("Search above to find apps to install"),
                                                    )
                                                    .into_element()
                                            } else {
                                                rect()
                                                    .width(Size::fill())
                                                    .vertical()
                                                    .spacing(8.)
                                                    .children(items.iter().cloned().map(|entry| {
                                                        let notice_c = notice;
                                                        let busy_c = busy;
                                                        let installed_c = installed;
                                                        let entry_c = entry.clone();
                                                        let entry_d = entry.clone();
                                                        let label_text =
                                                            if entry.installed { "Reinstall" } else { "Install" };
                                                        tile()
                                                            .child(setting_row(
                                                                entry.name.clone(),
                                                                Some(format!(
                                                                    "{} · {}",
                                                                    source_label(&entry.source),
                                                                    entry.id
                                                                )),
                                                                false,
                                                                rect()
                                                                    .horizontal()
                                                                    .spacing(8.)
                                                                    .cross_align(Alignment::Center)
                                                                    .content(Content::Flex)
                                                                    .child(app_row_icon(entry.name.as_str(), false))
                                                                    .child(status_chip(
                                                                        source_label(&entry.source),
                                                                        entry.installed,
                                                                        None,
                                                                    )),
                                                            ))
                                                            .child(
                                                                rect()
                                                                    .width(Size::fill())
                                                                    .horizontal()
                                                                    .content(Content::Flex)
                                                                    .main_align(Alignment::SpaceBetween)
                                                                    .cross_align(Alignment::Center)
                                                                    .child(label().font_size(12.).color(t.text_dim).text(
                                                                        if entry.summary.is_empty() {
                                                                            "No description".to_string()
                                                                        } else {
                                                                            entry.summary.clone()
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        rect()
                                                                            .horizontal()
                                                                            .spacing(8.)
                                                                            .content(Content::Flex)
                                                                            .child({
                                                                                let selected = selected;
                                                                                let nix_detail = nix_detail;
                                                                                let perms = perms;
                                                                                ghost_button("Details", move || {
                                                                                    select_entry(
                                                                                        entry_d.clone(),
                                                                                        selected,
                                                                                        nix_detail,
                                                                                        perms,
                                                                                    );
                                                                                })
                                                                            })
                                                                            .child(secondary_button(
                                                                                label_text,
                                                                                move || {
                                                                                    do_install(
                                                                                        entry_c.clone(),
                                                                                        notice_c,
                                                                                        busy_c,
                                                                                        installed_c,
                                                                                    );
                                                                                },
                                                                            )),
                                                                    ),
                                                            )
                                                            .into_element()
                                                    }))
                                                    .into_element()
                                            }
                                        })
                                        .into_element(),
                                    Tab::Installed => {
                                        let items = installed.read().clone();
                                        rect()
                                            .width(Size::fill())
                                            .vertical()
                                            .spacing(GAP)
                                            .child(page_head(
                                                FOLDER,
                                                "Installed",
                                                format!("{installed_count} applications on this system"),
                                            ))
                                            .child({
                                                if items.is_empty() {
                                                    rect()
                                                        .width(Size::fill())
                                                        .padding((32., 0.))
                                                        .vertical()
                                                        .cross_align(Alignment::Center)
                                                        .spacing(8.)
                                                        .child(icon(FOLDER, 28., t.text_dim))
                                                        .child(
                                                            label()
                                                                .font_size(13.)
                                                                .color(t.text_dim)
                                                                .text("Nothing installed yet"),
                                                        )
                                                        .into_element()
                                                } else {
                                                    rect()
                                                        .width(Size::fill())
                                                        .vertical()
                                                        .spacing(8.)
                                                        .children(items.iter().cloned().map(|entry| {
                                                            let notice_c = notice;
                                                            let busy_c = busy;
                                                            let installed_c = installed;
                                                            let entry_c = entry.clone();
                                                            let entry_d = entry.clone();
                                                            tile()
                                                                .child(setting_row(
                                                                    entry.name.clone(),
                                                                    Some(entry.id.clone()),
                                                                    false,
                                                                    rect()
                                                                        .horizontal()
                                                                        .spacing(8.)
                                                                        .cross_align(Alignment::Center)
                                                                        .content(Content::Flex)
                                                                        .child(app_row_icon(entry.name.as_str(), false))
                                                                        .child(status_chip(
                                                                            source_label(&entry.source),
                                                                            entry.installed,
                                                                            None,
                                                                        )),
                                                                ))
                                                                .child(
                                                                    rect()
                                                                        .width(Size::fill())
                                                                        .horizontal()
                                                                        .content(Content::Flex)
                                                                        .main_align(Alignment::SpaceBetween)
                                                                        .cross_align(Alignment::Center)
                                                                        .child(
                                                                            label().font_size(12.).color(t.text_dim).text(
                                                                                if entry.summary.is_empty() {
                                                                                    source_label(&entry.source).to_string()
                                                                                } else {
                                                                                    entry.summary.clone()
                                                                                },
                                                                            ),
                                                                        )
                                                                        .child(
                                                                            rect()
                                                                                .horizontal()
                                                                                .spacing(8.)
                                                                                .content(Content::Flex)
                                                                                .child({
                                                                                    let selected = selected;
                                                                                    let nix_detail = nix_detail;
                                                                                    let perms = perms;
                                                                                    ghost_button("Details", move || {
                                                                                        select_entry(
                                                                                            entry_d.clone(),
                                                                                            selected,
                                                                                            nix_detail,
                                                                                            perms,
                                                                                        );
                                                                                    })
                                                                                })
                                                                                .maybe(
                                                                                    entry.source != StoreSource::System,
                                                                                    |el| {
                                                                                        el.child(danger_button(
                                                                                            "Remove",
                                                                                            move || {
                                                                                                do_remove(
                                                                                                    entry_c.clone(),
                                                                                                    notice_c,
                                                                                                    busy_c,
                                                                                                    installed_c,
                                                                                                );
                                                                                            },
                                                                                        ))
                                                                                    },
                                                                                ),
                                                                        ),
                                                                )
                                                                .into_element()
                                                        }))
                                                        .into_element()
                                                }
                                            })
                                            .into_element()
                                    }
                                    Tab::Updates => {
                                        let items = updates.read().clone();
                                        rect()
                                            .width(Size::fill())
                                            .vertical()
                                            .spacing(GAP)
                                            .child(page_head(GENERAL, "Updates", "Keep sandboxed apps current"))
                                            .child(tile().child(setting_row(
                                                format!("{updates_count} Flatpak updates"),
                                                Some("Applies to user remotes only"),
                                                false,
                                                {
                                                    let notice = notice;
                                                    let updates = updates;
                                                    primary_button("Update all", move || {
                                                        let mut notice = notice;
                                                        match system::store_flatpak::update_all() {
                                                            Ok(log) => notice.set(log),
                                                            Err(e) => notice.set(e),
                                                        }
                                                        refresh_updates(updates);
                                                    })
                                                },
                                            )))
                                            .child({
                                                if items.is_empty() {
                                                    rect()
                                                        .width(Size::fill())
                                                        .padding((32., 0.))
                                                        .vertical()
                                                        .cross_align(Alignment::Center)
                                                        .spacing(8.)
                                                        .child(icon(GENERAL, 28., t.text_dim))
                                                        .child(
                                                            label()
                                                                .font_size(13.)
                                                                .color(t.text_dim)
                                                                .text("Everything is up to date"),
                                                        )
                                                        .into_element()
                                                } else {
                                                    rect()
                                                        .width(Size::fill())
                                                        .vertical()
                                                        .spacing(8.)
                                                        .children(items.iter().cloned().map(|fp| {
                                                            let mut notice_c = notice;
                                                            let updates_c = updates;
                                                            let app_id = fp.app_id.clone();
                                                            tile()
                                                                .child(setting_row(
                                                                    fp.name.clone(),
                                                                    Some(format!("{} · {}", fp.origin, fp.app_id)),
                                                                    false,
                                                                    secondary_button("Update", move || {
                                                                        match system::store_flatpak::update_single(&app_id) {
                                                                            Ok(m) => notice_c.set(m),
                                                                            Err(e) => notice_c.set(e),
                                                                        }
                                                                        refresh_updates(updates_c);
                                                                    }),
                                                                ))
                                                                .into_element()
                                                        }))
                                                        .into_element()
                                                }
                                            })
                                            .into_element()
                                    }
                                }
                            }
                        }),
                    ),
                )),
        )
        .into_element()
}

pub fn main() {
    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());
    launch(LaunchConfig::new().with_window(
        WindowConfig::new(store_app).with_title("Finick Apps").with_size(1080., 740.).with_min_size(640., 480.),
    ))
}
