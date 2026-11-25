use {crate::state::SettingsStore, freya::prelude::*, ipsea::settings::SettingKey, ui::*};

fn prov_key(id: &str) -> SettingKey {
    SettingKey::Custom(format!("search.provider.{id}"))
}
fn excl_key(i: usize) -> SettingKey {
    SettingKey::Custom(format!("search.exclude.{i}"))
}

fn index_db_present() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    for cand in [
        format!("{home}/.local/share/finick/index.db"),
        format!("{home}/.config/finick/index.db"),
    ] {
        if std::path::Path::new(&cand).exists() {
            return true;
        }
    }
    if let Ok(root) = std::env::var("FINICK_ROOT") {
        if std::path::Path::new(&format!("{root}/index.db")).exists() {
            return true;
        }
    }
    false
}

#[derive(Clone, PartialEq)]
struct Provider {
    id: &'static str,
    title: &'static str,
    hint: &'static str,
}

const PROVIDERS: &[Provider] = &[
    Provider { id: "apps", title: "Applications", hint: "Desktop entries and store apps" },
    Provider { id: "files", title: "Files", hint: "Indexed files and folders" },
    Provider { id: "calc", title: "Calculator", hint: "= expressions in launcher" },
    Provider { id: "cmds", title: "Commands", hint: ": finick commands and ! shell" },
];

#[derive(PartialEq)]
pub struct Search {
    pub store: SettingsStore,
}

impl Component for Search {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let apps_on: State<bool> = use_state(|| true);
        let files_on: State<bool> = use_state(|| true);
        let calc_on: State<bool> = use_state(|| true);
        let cmds_on: State<bool> = use_state(|| true);
        let excludes: State<Vec<String>> = use_state(|| vec!["/tmp".to_string(), "/proc".to_string()]);
        let new_loc: State<String> = use_state(String::new);
        let index_ok: State<Option<bool>> = use_state(|| None);

        use_hook(move || {
            let mut ok = index_ok;
            spawn(async move {
                let present = tokio::task::spawn_blocking(index_db_present).await.unwrap_or(false);
                ok.set(Some(present));
            });
        });

        let states = [apps_on, files_on, calc_on, cmds_on];
        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(
                SEARCH,
                "Search",
                "Launcher providers and indexed locations. Stored via the settings daemon.",
            ));
        let mut prov_tile = tile().child(tile_head(None, "Providers", Some(status_chip("Live", true, None))));
        prov_tile = prov_tile.child(tile_sub("Disabling a provider hides its rows in the launcher on next search."));
        for (i, p) in PROVIDERS.iter().enumerate() {
            let mut st = states[i];
            let on = *st.read();
            let s = store;
            let id = p.id;
            prov_tile = prov_tile.child(setting_row(
                p.title.to_string(),
                Some(p.hint.to_string()),
                i > 0,
                pill_switch(on, move |v| {
                    st.set(v);
                    s.set(prov_key(id), v);
                }),
            ));
        }
        base = base.child(prov_tile);

        let has_index = index_ok.read().clone().unwrap_or(false);
        let indexing_known = index_ok.read().is_some();
        let mut loc_tile = tile().child(tile_head(
            None,
            "Index locations",
            Some(status_chip(
                if !indexing_known {
                    "Checking"
                } else if has_index {
                    "Live"
                } else {
                    "Unavailable"
                },
                has_index,
                None,
            )),
        ));
        loc_tile = loc_tile.child(tile_sub(if !indexing_known {
            "Looking for the launcher index…".to_string()
        } else if has_index {
            "Excluded paths are skipped on the next launcher index.".to_string()
        } else {
            "Unavailable: no launcher index.db found, so excludes have nothing to act on yet.".to_string()
        }));
        let excluded: Vec<String> = excludes.read().clone();
        for (i, path) in excluded.iter().enumerate() {
            let path_owned = path.clone();
            let mut ex = excludes;
            let s = store;
            loc_tile = loc_tile.child(setting_row(
                path_owned.clone(),
                Some("Excluded".to_string()),
                false,
                secondary_button("Include", move || {
                    let mut v = ex.read().clone();
                    v.retain(|p| p != &path_owned);
                    ex.set(v);
                    s.set(excl_key(i), String::new());
                }),
            ));
        }
        loc_tile = loc_tile.child(rect().margin((10., 0., 0., 0.)).child(field_label("Exclude a path")));
        loc_tile = loc_tile.child(search_field(new_loc.into(), "/path/to/exclude", None));
        loc_tile = loc_tile.child(rect().margin((8., 0., 0., 0.)).child({
            let mut ex = excludes;
            let mut nl = new_loc;
            let s = store;
            secondary_button("Exclude path", move || {
                let p = nl.read().trim().to_string();
                if p.is_empty() {
                    return;
                }
                let mut v = ex.read().clone();
                if !v.contains(&p) {
                    v.push(p.clone());
                    s.set(excl_key(v.len()), p);
                }
                ex.set(v);
                nl.set(String::new());
            })
        }));
        base = base.child(loc_tile);
        FadeSlideIn::new().child(base)
    }
}
