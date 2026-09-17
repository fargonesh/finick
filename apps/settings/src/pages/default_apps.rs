use freya::prelude::*;
use system::{HyprlandBackend, SystemBackend};
use ui::*;

#[derive(PartialEq)]
pub struct DefaultApps;

impl Component for DefaultApps {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let cats = use_state(|| HyprlandBackend.get_default_applications());
        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(APPS, "Default Applications", "Choose default apps for common tasks"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
                    .child({
                        let mut cs = cats;
                        rect()
                            .cursor(CursorIcon::Pointer)
                            .on_press(move |_| cs.set(HyprlandBackend.get_default_applications()))
                            .child(ghost_button("Refresh", || {}))
                            .into_element()
                    })
                    .children(cats.read().iter().map(|c| {
                        let title = c.title.clone();
                        let desc = c.description.clone();
                        let cur_name = if c.current_name.is_empty() { "Not set".to_string() } else { c.current_name.clone() };
                        let cur_id = c.current_desktop_id.clone();
                        let cat_id = c.category_id.clone();
                        let apps = c.available_apps.clone();
                        let opts: Vec<DropdownOption> = if apps.is_empty() {
                            vec![DropdownOption { label: cur_name.clone(), value: cur_id.clone() }]
                        } else {
                            apps.iter().map(|a| DropdownOption { label: a.name.clone(), value: a.desktop_id.clone() }).collect()
                        };
                        let cur_val = if cur_id.is_empty() { cur_name.clone() } else { cur_id.clone() };
                        let label_val = if apps.iter().any(|a| a.desktop_id == cur_val) {
                            apps.iter().find(|a| a.desktop_id == cur_val).map(|a| a.name.clone()).unwrap_or(cur_name.clone())
                        } else {
                            cur_name.clone()
                        };
                        let cat_id2 = cat_id.clone();
                        let mut cs = cats;
                        let on_change = EventHandler::new(move |chosen: String| {
                            let _ = HyprlandBackend.set_default_application(&cat_id2, &chosen);
                            cs.set(HyprlandBackend.get_default_applications());
                        });
                        tile()
                            .child(tile_head(None, title.clone(), None::<Element>))
                            .child(tile_sub(desc))
                            .child(setting_row(format!("Current: {label_val}"), Some(format!("{cat_id}")), false, label().font_size(11.).color(t.text_dim).text(cur_name.clone())))
                            .child(dropdown_select(label_val, opts, on_change))
                            .into_element()
                    })),
            )
    }
}
