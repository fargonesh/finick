use {freya::prelude::*, system::scan_installed_apps, system::store::flatpak, ui::*};

#[derive(Clone, PartialEq)]
struct PermRow {
    key: String,
    label: String,
    allowed: bool,
}

fn load_perms(app_id: &str) -> Vec<PermRow> {
    flatpak::permissions(app_id)
        .into_iter()
        .map(|p| PermRow { key: p.key, label: p.label, allowed: p.allowed })
        .collect()
}

#[derive(PartialEq)]
pub struct Apps;

impl Component for Apps {
    fn render(&self) -> impl IntoElement {
        let query: State<String> = use_state(String::new);
        let selected: State<Option<String>> = use_state(|| None::<String>);
        let perms: State<Vec<PermRow>> = use_state(Vec::new);
        let busy: State<bool> = use_state(|| false);
        let err: State<Option<String>> = use_state(|| None);
        let apps: State<Vec<system::InstalledApp>> = use_state(Vec::new);
        let loading: State<bool> = use_state(|| true);

        use_hook(move || {
            let mut a = apps;
            let mut ld = loading;
            spawn(async move {
                let list = tokio::task::spawn_blocking(scan_installed_apps).await.unwrap_or_default();
                a.set(list);
                ld.set(false);
            });
        });

        let q = query.read().to_lowercase();
        let all = apps.read().clone();
        let filtered: Vec<system::InstalledApp> = all
            .into_iter()
            .filter(|a| {
                q.is_empty() || a.name.to_lowercase().contains(&q) || a.desktop_id.to_lowercase().contains(&q)
            })
            .take(30)
            .collect();
        let sel_id = selected.read().clone();
        let sel_app = sel_id
            .as_ref()
            .and_then(|id| filtered.iter().find(|a| &a.desktop_id == id).cloned())
            .or_else(|| filtered.first().cloned());
        let perm_list = perms.read().clone();
        let is_loading = *loading.read();
        let is_busy = *busy.read();
        let err_opt = err.read().clone();

        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(APPS, "Apps", "Installed apps, Flatpak permissions and default handlers."))
            .maybe_child(err_opt.map(|m| {
                rect()
                    .width(Size::fill())
                    .child(notice_pill(m, NoticeKind::Error))
                    .child(rect().margin((6., 0., 0., 0.)).child(ghost_button("Dismiss", {
                        let mut e = err;
                        move || e.set(None)
                    })))
            }))
            .maybe(is_busy, |el| el.child(tile_sub("Applying Flatpak override…")));

        let mut list_tile = tile().child(tile_head(
            None,
            format!("Installed ({})", filtered.len()),
            if is_loading { Some(status_chip("Loading", false, None)) } else { None },
        ));
        list_tile = list_tile.child(search_field(query.into(), "Search apps…", None));
        if is_loading && filtered.is_empty() {
            list_tile = list_tile.child(tile_sub("Reading installed applications…"));
        } else if filtered.is_empty() {
            list_tile = list_tile.child(empty_state(APPS, "No apps match", "Try a different search."));
        } else {
            for app in filtered.iter().take(12) {
                let id = app.desktop_id.clone();
                let is_sel = sel_id.as_deref() == Some(id.as_str());
                let mut sel = selected;
                let mut pm = perms;
                let aid = id.clone();
                list_tile = list_tile.child(setting_row(
                    app.name.clone(),
                    Some(app.desktop_id.clone()),
                    false,
                    secondary_button(if is_sel { "Selected" } else { "View" }, move || {
                        sel.set(Some(aid.clone()));
                        let aid2 = aid.clone();
                        spawn(async move {
                            let rows = tokio::task::spawn_blocking(move || load_perms(&aid2)).await.unwrap_or_default();
                            pm.set(rows);
                        });
                    }),
                ));
            }
        }
        base = base.child(list_tile);

        if let Some(app) = sel_app {
            let is_flatpak = app.desktop_id.contains('.') && flatpak::flatpak_available();
            let mut detail = tile().child(tile_head(
                None,
                format!("Permissions · {}", app.name),
                Some(status_chip(if is_flatpak { "Flatpak" } else { "System" }, is_flatpak, None)),
            ));
            if !is_flatpak {
                detail = detail.child(tile_sub("Unavailable: per-app revoke only applies to Flatpak apps. System packages have no override backend."));
            } else if perm_list.is_empty() {
                detail = detail.child(tile_sub("No Flatpak context entries found for this app. Nothing to allow or revoke."));
            } else {
                for p in perm_list.iter().take(10) {
                    let key = p.key.clone();
                    let aid = app.desktop_id.clone();
                    let allowed = p.allowed;
                    let mut pm = perms;
                    let mut b = busy;
                    let mut e = err;
                    detail = detail.child(setting_row(
                        p.label.clone(),
                        Some(p.key.clone()),
                        false,
                        pill_switch(allowed, move |v| {
                            b.set(true);
                            e.set(None);
                            let aid_c = aid.clone();
                            let key_c2 = key.clone();
                            let key_cmp = key.clone();
                            spawn(async move {
                                let res = tokio::task::spawn_blocking(move || {
                                    flatpak::set_permission(&aid_c, &key_c2, v)
                                })
                                .await
                                .unwrap_or(Err("Task failed".to_string()));
                                b.set(false);
                                match res {
                                    Ok(_) => {
                                        let mut rows = pm.read().clone();
                                        if let Some(r) = rows.iter_mut().find(|r| r.key == key_cmp) {
                                            r.allowed = v;
                                        }
                                        pm.set(rows);
                                    }
                                    Err(msg) => e.set(Some(msg)),
                                }
                            });
                        }),
                    ));
                }
            }
            detail = detail.child(tile_sub("Default handlers live in Default Apps. This panel never claims control it lacks."));
            detail = detail.child(setting_row(
                "Default handlers",
                Some("MIME defaults via xdg-mime".to_string()),
                true,
                secondary_button("Open Default Apps", move || {
                    let _ = std::process::Command::new("finick-settings").arg("default-apps").spawn();
                }),
            ));
            base = base.child(detail);
        }
        FadeSlideIn::new().child(base)
    }
}
