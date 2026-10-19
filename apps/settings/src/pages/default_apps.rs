use {
    freya::prelude::*,
    system::{HyprlandBackend, SystemBackend},
    ui::*,
};

#[derive(PartialEq)]
pub struct DefaultApps;

impl Component for DefaultApps {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let cats = use_state(Vec::<system::DefaultAppCategoryInfo>::new);
        let is_loading = use_state(|| true);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut cs = cats;
            let mut ld = is_loading;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let info = HyprlandBackend.get_default_applications();
                let _ = tx.send(info);
            });

            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    cs.set(info);
                    ld.set(false);
                }
            });
        }

        let loading = *is_loading.read();
        let categories = cats.read().clone();

        FadeSlideIn::new().child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(page_head(APPS, "Default applications", "Choose default apps for common tasks"))
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            format!("Categories ({})", categories.len()),
                            Some(secondary_button(if loading { "Refreshing..." } else { "Refresh" }, {
                                let mut l = loaded;
                                let mut ld = is_loading;
                                move || {
                                    ld.set(true);
                                    l.set(false);
                                }
                            })),
                        ))
                        .child(tile_sub(if loading && categories.is_empty() {
                            "Loading default applications..."
                        } else {
                            "Pick which app opens each kind of file or link."
                        }))
                        .into_element(),
                )
                .child({
                    if loading && categories.is_empty() {
                        rect()
                            .width(Size::fill())
                            .padding(16.)
                            .center()
                            .child(label().font_size(14.).color(t.text_dim).text("Loading default applications..."))
                            .into_element()
                    } else if categories.is_empty() {
                        rect()
                            .width(Size::fill())
                            .padding(16.)
                            .center()
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_dim)
                                    .text("No default application categories found."),
                            )
                            .into_element()
                    } else {
                        rect()
                            .width(Size::fill())
                            .vertical()
                            .spacing(GAP)
                            .children(categories.into_iter().map(|c| {
                                let title = c.title.clone();
                                let desc = c.description.clone();
                                let cur_name = if c.current_name.is_empty() {
                                    "Not set".to_string()
                                } else {
                                    c.current_name.clone()
                                };
                                let cur_id = c.current_desktop_id.clone();
                                let cat_id = c.category_id.clone();
                                let apps = c.available_apps.clone();
                                let opts: Vec<DropdownOption> = if apps.is_empty() {
                                    vec![DropdownOption { label: cur_name.clone(), value: cur_id.clone() }]
                                } else {
                                    apps.iter()
                                        .map(|a| DropdownOption {
                                            label: a.name.clone(),
                                            value: a.desktop_id.clone(),
                                        })
                                        .collect()
                                };
                                let cur_val =
                                    if cur_id.is_empty() { cur_name.clone() } else { cur_id.clone() };
                                let label_val = if apps.iter().any(|a| a.desktop_id == cur_val) {
                                    apps.iter()
                                        .find(|a| a.desktop_id == cur_val)
                                        .map(|a| a.name.clone())
                                        .unwrap_or(cur_name.clone())
                                } else {
                                    cur_name.clone()
                                };
                                let cat_id2 = cat_id.clone();
                                let on_change = EventHandler::new(move |chosen: String| {
                                    let cat = cat_id2.clone();
                                    let mut cs = cats;
                                    let (tx, mut rx) =
                                        tokio::sync::mpsc::unbounded_channel();
                                    std::thread::spawn(move || {
                                        let _ = HyprlandBackend
                                            .set_default_application(&cat, &chosen);
                                        let info = HyprlandBackend.get_default_applications();
                                        let _ = tx.send(info);
                                    });
                                    freya::prelude::spawn(async move {
                                        if let Some(info) = rx.recv().await {
                                            cs.set(info);
                                        }
                                    });
                                });
                                tile()
                                    .child(tile_head(None, title.clone(), None::<Element>))
                                    .child(tile_sub(desc))
                                    .child(setting_row(
                                        format!("Current: {label_val}"),
                                        Some(format!("{cat_id}")),
                                        false,
                                        label()
                                            .font_size(11.)
                                            .color(t.text_dim)
                                            .text(cur_name.clone()),
                                    ))
                                    .child(dropdown_select(label_val, opts, on_change))
                                    .into_element()
                            }))
                            .into_element()
                    }
                }),
        )
    }
}
