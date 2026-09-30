#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use {
    freya::prelude::*,
    serde_json::Value,
    std::collections::HashMap,
    system::{
        store::{StoreEntry, StoreSource},
        store_opts::OptField,
    },
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

/// Home Manager `programs.*` options card for a nix detail view.
/// Free fn like `app_row_icon`: states in, element out.
fn options_card(
    module: Option<String>,
    loading: bool,
    fields: Vec<OptField>,
    values: HashMap<String, Value>,
    editing: Option<String>,
    edit_path: State<Option<String>>,
    draft: State<String>,
    opt_values: State<HashMap<String, Value>>,
    notice: State<String>,
) -> Element {
    let t = use_app_theme();
    let mut card =
        rect().width(Size::fill()).vertical().spacing(8.).child(tile_head(None, "Home Manager options", module.clone()));
    if loading {
        return card.child(tile_sub("Resolving options schema (first run fetches HM sources, ~1 min)…")).into_element();
    }
    let Some(module) = module else {
        return card.child(tile_sub("No programs.* module for this package — declared via home.packages")).into_element();
    };
    let _ = module;
    if let Some(enable) = fields.iter().find(|f| f.name == "enable" && matches!(f.kind, system::store_opts::OptKind::Bool)) {
        let checked = opt_bool(enable, &values);
        let field = enable.clone();
        let notice = notice;
        let opt_values = opt_values;
        card = card.child(setting_row(
            "Enable module".to_string(),
            Some(field.path.clone()),
            false,
            pill_switch(checked, move |v| save_opt_direct(&field, Value::Bool(v), notice, opt_values)),
        ));
    }
    let editable: Vec<OptField> = fields
        .iter()
        .filter(|f| {
            f.name != "enable"
                && matches!(
                    f.kind,
                    system::store_opts::OptKind::Bool
                        | system::store_opts::OptKind::String
                        | system::store_opts::OptKind::Int
                        | system::store_opts::OptKind::Float
                        | system::store_opts::OptKind::Enum(_)
                )
        })
        .take(12)
        .cloned()
        .collect();
    let editable_n = editable.len();
    for field in editable {
        let path = field.path.clone();
        match &field.kind {
            system::store_opts::OptKind::Bool => {
                let checked = opt_bool(&field, &values);
                let notice = notice;
                let opt_values = opt_values;
                card = card.child(setting_row(
                    field.name.clone(),
                    opt_blurb(&field),
                    false,
                    pill_switch(checked, move |v| save_opt_direct(&field, Value::Bool(v), notice, opt_values)),
                ));
            }
            system::store_opts::OptKind::Enum(allowed)
                if allowed.len() <= 6 && editing.as_deref() != Some(path.as_str()) =>
            {
                let current = values
                    .get(&path)
                    .and_then(|v| v.as_str())
                    .or_else(|| field.default.as_ref().and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();
                let items: Vec<(String, String)> = allowed.iter().map(|v| (v.clone(), v.clone())).collect();
                let notice = notice;
                let opt_values = opt_values;
                card = card.child(setting_row(
                    field.name.clone(),
                    opt_blurb(&field),
                    false,
                    segmented_control_dynamic(items, current, move |v: String| {
                        save_opt_direct(&field, Value::String(v), notice, opt_values);
                    }),
                ));
            }
            _ => {
                if editing.as_deref() == Some(path.as_str()) {
                    let mut edit_path = edit_path;
                    let draft = draft;
                    let field_c = field.clone();
                    let field_s = field.clone();
                    let notice = notice;
                    let opt_values = opt_values;
                    let draft_v = draft;
                    card = card.child(field_label(format!("{} — {}", field.name, field.path))).child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .content(Content::Flex)
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .height(Size::px(40.))
                                    .corner_radius(RADIUS_PILL)
                                    .background(t.bg)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .padding((0., 12.))
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .content(Content::Flex)
                                    .child(
                                        Input::new(draft)
                                            .width(Size::fill())
                                            .background(Color::TRANSPARENT)
                                            .border_fill(Color::TRANSPARENT)
                                            .focus_background(Color::TRANSPARENT)
                                            .focus_border_fill(Color::TRANSPARENT)
                                            .on_submit(move |_| {
                                                save_opt_value(&field_s, &draft_v.read().clone(), notice, opt_values);
                                                edit_path.set(None);
                                            }),
                                    ),
                            )
                            .child(secondary_button("Save", move || {
                                save_opt_value(&field_c, &draft.read().clone(), notice, opt_values);
                                edit_path.set(None);
                            }))
                            .child({
                                let mut edit_path = edit_path;
                                ghost_button("Cancel", move || edit_path.set(None))
                            }),
                    );
                } else {
                    let shown = display_value(&field, &values);
                    let mut edit_path_c = edit_path;
                    let mut draft_c = draft;
                    let shown_c = shown.clone();
                    let field_c = field.clone();
                    card = card.child(setting_row(
                        field.name.clone(),
                        Some(format!("{} · {}", opt_blurb(&field).unwrap_or_default(), shown)),
                        false,
                        ghost_button("Edit", move || {
                            edit_path_c.set(Some(field_c.path.clone()));
                            draft_c.set(shown_c.clone());
                        }),
                    ));
                }
            }
        }
    }
    let readonly: Vec<OptField> =
        fields.iter().filter(|f| matches!(f.kind, system::store_opts::OptKind::Unsupported(_))).take(6).cloned().collect();
    let readonly_n = readonly.len();
    for field in readonly {
        let type_name = match &field.kind {
            system::store_opts::OptKind::Unsupported(t) => t.clone(),
            _ => String::new(),
        };
        card = card.child(setting_row(
            field.name.clone(),
            Some(format!("{type_name} · {}", opt_blurb(&field).unwrap_or_default())),
            false,
            label().font_size(12.).color(t.text_dim).text("read-only"),
        ));
    }
    let enable_count =
        fields.iter().filter(|f| f.name == "enable" && matches!(f.kind, system::store_opts::OptKind::Bool)).count();
    let hidden = fields.len().saturating_sub(enable_count + editable_n + readonly_n);
    if hidden > 0 {
        card = card.child(tile_sub(format!("+{hidden} more options — see the Home Manager manual")));
    }
    card.into_element()
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

enum JobEvent {
    Progress(String),
    Done(String),
}

/// Run a store job via finickd (or locally if the daemon is unreachable),
/// streaming progress into the notice line. One helper for all five op sites.
fn spawn_store_job(
    job: system::store::StoreJob,
    verb: String,
    mut notice: State<String>,
    mut busy: State<bool>,
    refresh: impl FnOnce() + 'static,
) {
    busy.set(true);
    notice.set(verb);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let mut progress = |ev: system::store::StoreProgress| {
            let line = match ev {
                system::store::StoreProgress::Started(d) => format!("{d}…"),
                system::store::StoreProgress::Log(l) => l,
            };
            let _ = tx.send(JobEvent::Progress(line));
        };
        let msg = system::store::run_job_ipc_or_local(job, &mut progress).unwrap_or_else(|e| e);
        let _ = tx.send(JobEvent::Done(msg));
    });
    spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                JobEvent::Progress(line) => notice.set(line),
                JobEvent::Done(msg) => {
                    notice.set(msg);
                    busy.set(false);
                    refresh();
                    break;
                }
            }
        }
    });
}

fn do_install(entry: StoreEntry, mut notice: State<String>, busy: State<bool>, installed: State<Vec<StoreEntry>>) {
    let job = match entry.source {
        StoreSource::Flathub => system::store::StoreJob::InstallFlatpak(entry.id.clone()),
        StoreSource::Nixpkgs => system::store::StoreJob::InstallNix(entry.id.trim_start_matches("nixpkgs#").to_string()),
        StoreSource::System => {
            notice.set("Already installed".to_string());
            return;
        }
    };
    spawn_store_job(job, format!("Installing {}…", entry.name), notice, busy, move || refresh_installed(installed));
}

fn do_remove(entry: StoreEntry, mut notice: State<String>, busy: State<bool>, installed: State<Vec<StoreEntry>>) {
    let job = match entry.source {
        StoreSource::Flathub => system::store::StoreJob::RemoveFlatpak(entry.id.clone()),
        StoreSource::Nixpkgs => system::store::StoreJob::RemoveNix(entry.id.trim_start_matches("nixpkgs#").to_string()),
        StoreSource::System => {
            notice.set("System apps are managed by your OS image".to_string());
            return;
        }
    };
    spawn_store_job(job, format!("Removing {}…", entry.name), notice, busy, move || refresh_installed(installed));
}

fn select_entry(
    entry: StoreEntry,
    mut selected: State<Option<StoreEntry>>,
    mut nix_detail: State<Option<system::store_nix::NixPackage>>,
    mut perms: State<Vec<system::store_flatpak::FlatpakPermission>>,
    mut opt_module: State<Option<String>>,
    mut opt_fields: State<Vec<OptField>>,
    mut opt_values: State<HashMap<String, Value>>,
    mut opt_loading: State<bool>,
) {
    selected.set(Some(entry.clone()));
    nix_detail.set(None);
    perms.set(Vec::new());
    opt_module.set(None);
    opt_fields.set(Vec::new());
    opt_values.set(HashMap::new());
    opt_loading.set(false);
    let is_nix = entry.source == StoreSource::Nixpkgs;
    let attr = entry.id.trim_start_matches("nixpkgs#").to_string();
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
    // Nix entries: resolve the programs.* module + schema off-thread
    // (first run downloads HM sources and takes ~1 min, then cached).
    if is_nix {
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
        opt_loading.set(true);
        std::thread::spawn(move || {
            let found = system::store_opts::module_for_attr(&attr);
            if let Some(module) = found.clone() {
                let _ = tx2.send((found, system::store_opts::module_schema(&module)));
            } else {
                let _ = tx2.send((None, Vec::new()));
            }
        });
        spawn(async move {
            if let Some((module, fields)) = rx2.recv().await {
                opt_module.set(module);
                if !fields.is_empty() {
                    opt_values.set(load_opt_values(&fields));
                }
                opt_fields.set(fields);
            }
            opt_loading.set(false);
        });
    }
}

/// Current declared values for the given schema paths.
fn load_opt_values(fields: &[OptField]) -> HashMap<String, Value> {
    let decls = system::store_hm::load_decls();
    fields.iter().filter_map(|f| decls.program_options.get(&f.path).cloned().map(|v| (f.path.clone(), v))).collect()
}

fn display_value(field: &OptField, values: &HashMap<String, Value>) -> String {
    if let Some(v) = values.get(&field.path).or(field.default.as_ref()) {
        match v {
            Value::Bool(b) => return b.to_string(),
            Value::Number(n) => return n.to_string(),
            Value::String(s) => return s.clone(),
            _ => {}
        }
    }
    "unset".to_string()
}

/// Persist a pre-parsed value (bool toggles, enum picks). Prunes defaults.
fn save_opt_direct(
    field: &OptField,
    value: Value,
    mut notice: State<String>,
    mut opt_values: State<HashMap<String, Value>>,
) {
    let to_store = if Some(&value) == field.default.as_ref() { None } else { Some(value) };
    match system::store_hm::set_program_option(&field.path, to_store) {
        Ok(_) => {
            opt_values.set(load_opt_values(std::slice::from_ref(field)));
            notice.set(format!("Saved {} — Apply to rebuild", field.path));
        }
        Err(e) => notice.set(e),
    }
}

/// Current bool for a field (declared value, else schema default, else false).
fn opt_bool(field: &OptField, values: &HashMap<String, Value>) -> bool {
    values
        .get(&field.path)
        .and_then(|v| v.as_bool())
        .or_else(|| field.default.as_ref().and_then(|v| v.as_bool()))
        .unwrap_or(false)
}

/// One-line description for a row subtitle.
fn opt_blurb(field: &OptField) -> Option<String> {
    let desc = field.description.lines().next().unwrap_or("").trim();
    let short: String = desc.chars().take(90).collect();
    if short.is_empty() { None } else { Some(short) }
}

/// Persist one field from the draft string. Errors go to the notice line.
fn save_opt_value(field: &OptField, draft: &str, mut notice: State<String>, mut opt_values: State<HashMap<String, Value>>) {
    let parsed: Option<Value> = match &field.kind {
        system::store_opts::OptKind::Bool => None,
        system::store_opts::OptKind::String => Some(Value::String(draft.to_string())),
        system::store_opts::OptKind::Int => match draft.trim().parse::<i64>() {
            Ok(n) => Some(n.into()),
            Err(_) => {
                notice.set(format!("{}: not a whole number", field.name));
                return;
            }
        },
        system::store_opts::OptKind::Float => match draft.trim().parse::<f64>() {
            Ok(n) => serde_json::Number::from_f64(n).map(Value::Number).or_else(|| {
                notice.set(format!("{}: not a number", field.name));
                None
            }),
            Err(_) => {
                notice.set(format!("{}: not a number", field.name));
                return;
            }
        },
        system::store_opts::OptKind::Enum(allowed) => {
            if allowed.iter().any(|a| a == draft.trim()) {
                Some(Value::String(draft.trim().to_string()))
            } else {
                notice.set(format!("{}: pick one of {}", field.name, allowed.join(", ")));
                return;
            }
        }
        system::store_opts::OptKind::Unsupported(_) => {
            notice.set(format!("{} is read-only here", field.name));
            return;
        }
    };
    // Prune values matching the module default to keep apps.nix clean.
    let to_store = match parsed {
        Some(v) if Some(&v) == field.default.as_ref() => None,
        other => other,
    };
    match system::store_hm::set_program_option(&field.path, to_store) {
        Ok(_) => {
            opt_values.set(load_opt_values(std::slice::from_ref(field)));
            notice.set(format!("Saved {} — Apply to rebuild", field.path));
        }
        Err(e) => notice.set(e),
    }
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
    let opt_module = use_state(|| None::<String>);
    let opt_fields = use_state(Vec::<OptField>::new);
    let opt_values = use_state(HashMap::<String, Value>::new);
    let opt_loading = use_state(|| false);
    let opt_edit_path = use_state(|| None::<String>);
    let opt_draft = use_state(String::new);

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

    responsive_view(780.0, move |compact| {
        let sidebar_w = if compact { 180. } else { 232. };
        let content_pad = if compact { (18., 16., 32., 16.) } else { (30., 34., 64., 34.) };
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .background(t.bg)
            .overflow(Overflow::Clip)
            .child(
                rect()
                    .width(Size::px(sidebar_w))
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
                            format!(
                                "Updates{}",
                                if updates_count > 0 { format!(" ({updates_count})") } else { String::new() }
                            ),
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
                                        spawn_store_job(
                                            system::store::StoreJob::ApplyHomeManager,
                                            "Applying home-manager switch…".to_string(),
                                            notice,
                                            busy,
                                            || {},
                                        )
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
                            rect().width(Size::fill()).padding(content_pad).vertical().spacing(GAP).child({
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
                                                    el.child(
                                                        label().font_size(12.).color(t.text_dim).text(m.homepage.clone()),
                                                    )
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
                                        .maybe(entry.source == StoreSource::Nixpkgs, |el| {
                                            el.child(options_card(
                                                opt_module.read().clone(),
                                                *opt_loading.read(),
                                                opt_fields.read().clone(),
                                                opt_values.read().clone(),
                                                opt_edit_path.read().clone(),
                                                opt_edit_path,
                                                opt_draft,
                                                opt_values,
                                                notice_c,
                                            ))
                                        })
                                        .maybe(!perm_list.is_empty(), |el| {
                                            el.child(tile_head(None, "Permissions", Some("Flatpak sandbox".to_string())))
                                                .child(rect().width(Size::fill()).vertical().spacing(2.).children(
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
                                                ))
                                        })
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .horizontal()
                                                .spacing(8.)
                                                .content(Content::Flex)
                                                .child(if entry.source == StoreSource::System {
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
                                                }),
                                        )
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
                                            .child(label().font_size(12.).color(t.text_dim).text(notice_val.clone()))
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
                                                                        .child(
                                                                            label().font_size(12.).color(t.text_dim).text(
                                                                                if entry.summary.is_empty() {
                                                                                    "No description".to_string()
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
                                                                                    let opt_module = opt_module;
                                                                                    let opt_fields = opt_fields;
                                                                                    let opt_values = opt_values;
                                                                                    ghost_button("Details", move || {
                                                                                        select_entry(
                                                                                            entry_d.clone(),
                                                                                            selected,
                                                                                            nix_detail,
                                                                                            perms,
                                                                                            opt_module,
                                                                                            opt_fields,
                                                                                            opt_values,
                                                                                            opt_loading,
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
                                                                                label()
                                                                                    .font_size(12.)
                                                                                    .color(t.text_dim)
                                                                                    .text(if entry.summary.is_empty() {
                                                                                        source_label(&entry.source)
                                                                                            .to_string()
                                                                                    } else {
                                                                                        entry.summary.clone()
                                                                                    }),
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
                                                                                        let opt_module = opt_module;
                                                                                        let opt_fields = opt_fields;
                                                                                        let opt_values = opt_values;
                                                                                        ghost_button("Details", move || {
                                                                                            select_entry(
                                                                                                entry_d.clone(),
                                                                                                selected,
                                                                                                nix_detail,
                                                                                                perms,
                                                                                                opt_module,
                                                                                                opt_fields,
                                                                                                opt_values,
                                                                                                opt_loading,
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
                                                        let busy = busy;
                                                        let updates = updates;
                                                        primary_button("Update all", move || {
                                                            spawn_store_job(
                                                                system::store::StoreJob::UpdateFlatpaks,
                                                                "Updating Flatpaks…".to_string(),
                                                                notice,
                                                                busy,
                                                                move || refresh_updates(updates),
                                                            )
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
                                                                let notice_c = notice;
                                                                let busy_c = busy;
                                                                let updates_c = updates;
                                                                let app_id = fp.app_id.clone();
                                                                let app_name = fp.name.clone();
                                                                tile()
                                                                    .child(setting_row(
                                                                        fp.name.clone(),
                                                                        Some(format!("{} · {}", fp.origin, fp.app_id)),
                                                                        false,
                                                                        secondary_button("Update", move || {
                                                                            spawn_store_job(
                                                                                system::store::StoreJob::UpdateFlatpak(
                                                                                    app_id.clone(),
                                                                                ),
                                                                                format!("Updating {app_name}…"),
                                                                                notice_c,
                                                                                busy_c,
                                                                                move || refresh_updates(updates_c),
                                                                            )
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
    })
}

pub fn main() {
    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());
    launch(LaunchConfig::new().with_window(
        WindowConfig::new(store_app).with_title("Finick Apps").with_size(1080., 740.).with_min_size(640., 480.),
    ))
}
