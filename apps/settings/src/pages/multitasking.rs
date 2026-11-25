use {crate::state::SettingsStore, freya::prelude::*, ipsea::settings::SettingKey, system::HyprlandBackend, ui::*};

fn hot_key() -> SettingKey {
    SettingKey::Custom("multitasking.hot_corner".to_string())
}
fn tiling_key() -> SettingKey {
    SettingKey::Custom("multitasking.edge_tiling".to_string())
}
fn gaps_key() -> SettingKey {
    SettingKey::Custom("multitasking.gaps".to_string())
}
fn ws_key() -> SettingKey {
    SettingKey::Custom("multitasking.workspaces".to_string())
}

fn apply_hypr(lua: &str, keyword: &str, val: &str) -> Result<(), String> {
    HyprlandBackend::hyprctl_set_config(lua, keyword, val)
}

#[derive(PartialEq)]
pub struct Multitasking {
    pub store: SettingsStore,
}

impl Component for Multitasking {
    fn render(&self) -> impl IntoElement {
        let store = self.store;
        let hot: State<bool> = use_state(|| false);
        let tiling: State<bool> = use_state(|| true);
        let gaps: State<f64> = use_state(|| 12.0);
        let ws_count: State<usize> = use_state(|| 4);
        let err: State<Option<String>> = use_state(|| None);

        let err_opt = err.read().clone();
        let base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(
                LAYOUT_GRID,
                "Multitasking",
                "Hot corner, workspaces and edge tiling. Applies live, persists via the daemon.",
            ))
            .maybe_child(err_opt.map(|m| {
                rect()
                    .width(Size::fill())
                    .child(notice_pill(m, NoticeKind::Error))
                    .child(rect().margin((6., 0., 0., 0.)).child(ghost_button("Dismiss", {
                        let mut e = err;
                        move || e.set(None)
                    })))
            }))
            .child(
                tile()
                    .child(tile_head(None, "Hot corner", Some(status_chip(if *hot.read() { "On" } else { "Off" }, *hot.read(), None))))
                    .child(tile_sub("Top-left corner opens the overview. Maps to misc:enable_hot_corners."))
                    .child(setting_row(
                        "Hot corner",
                        Some("Apply live; denial shows here and rolls back.".to_string()),
                        false,
                        {
                            let mut h = hot;
                            let e = err;
                            let s = store;
                            pill_switch(*hot.read(), move |v| {
                                h.set(v);
                                let val = if v { "true" } else { "false" };
                                let lua = format!("hl.config({{ misc = {{ enable_hot_corners = {val} }} }})");
                                let mut h = h;
                                let mut e = e;
                                spawn(async move {
                                    let res = tokio::task::spawn_blocking(move || {
                                        apply_hypr(&lua, "misc:enable_hot_corners", val)
                                    })
                                    .await
                                    .unwrap_or(Err("Task failed".to_string()));
                                    match res {
                                        Ok(()) => s.set(hot_key(), v),
                                        Err(msg) => {
                                            h.set(!v);
                                            e.set(Some(msg));
                                        }
                                    }
                                });
                            })
                        },
                    )),
            )
            .child(
                tile()
                    .child(tile_head(None, "Workspaces", None::<String>))
                    .child(field_label("Workspace count"))
                    .child({
                        let mut w = ws_count;
                        let s = store;
                        segmented_control(vec![("2", 2usize), ("4", 4usize), ("6", 6usize), ("9", 9usize)], *ws_count.read(), move |v| {
                            w.set(v);
                            s.set(ws_key(), v as i64);
                        })
                    })
                    .child(tile_sub("Finick keeps N named workspaces; switch with Super+1…9. Count persists via the daemon.")),
            )
            .child(
                tile()
                    .child(tile_head(None, "Edge tiling", Some(status_chip(if *tiling.read() { "On" } else { "Off" }, *tiling.read(), None))))
                    .child(tile_sub("Drag windows to screen edges to tile. Maps to general:snap:enabled."))
                    .child(setting_row(
                        "Snap to edges",
                        Some("Apply live via general:snap:enabled.".to_string()),
                        false,
                        {
                            let mut t = tiling;
                            let e = err;
                            let s = store;
                            pill_switch(*tiling.read(), move |v| {
                                t.set(v);
                                let val = if v { "true" } else { "false" };
                                let lua = format!("hl.config({{ general = {{ snap = {{ enabled = {val} }} }} }})");
                                let mut t = t;
                                let mut e = e;
                                spawn(async move {
                                    let res = tokio::task::spawn_blocking(move || {
                                        apply_hypr(&lua, "general:snap:enabled", val)
                                    })
                                    .await
                                    .unwrap_or(Err("Task failed".to_string()));
                                    match res {
                                        Ok(()) => s.set(tiling_key(), v),
                                        Err(msg) => {
                                            t.set(!v);
                                            e.set(Some(msg));
                                        }
                                    }
                                });
                            })
                        },
                    ))
                    .child(rect().margin((10., 0., 0., 0.)).child(field_label(format!("Workspace gap · {} px", *gaps.read() as i32))))
                    .child({
                        let mut g = gaps;
                        let s = store;
                        let mut e = err;
                        slider_row(None, *gaps.read(), move |v| {
                            g.set(v);
                            let n = v as i32;
                            let lua = format!("hl.config({{ general = {{ gaps_out = {n} }} }})");
                            spawn(async move {
                                let res = tokio::task::spawn_blocking(move || {
                                    apply_hypr(&lua, "general:gaps_out", &n.to_string())
                                })
                                .await
                                .unwrap_or(Err("Task failed".to_string()));
                                match res {
                                    Ok(()) => s.set(gaps_key(), v),
                                    Err(msg) => e.set(Some(msg)),
                                }
                            });
                        })
                    }),
            );
        FadeSlideIn::new().child(base)
    }
}
