use freya::prelude::*;
use system::{HyprlandBackend, KeyboardShortcut, SystemBackend};
use ui::*;

#[derive(PartialEq)]
pub struct Shortcuts;

impl Component for Shortcuts {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let shortcuts = use_state(|| HyprlandBackend.get_keyboard_shortcuts());
        let add_open = use_state(|| false);
        let new_name = use_state(String::new);
        let new_mods = use_state(String::new);
        let new_key = use_state(String::new);
        let new_cmd = use_state(String::new);
        let filter = use_state(String::new);
        let query = filter.read().to_lowercase();
        let list = shortcuts.read().clone();
        let filtered: Vec<KeyboardShortcut> = list
            .into_iter()
            .filter(|s| {
                if query.is_empty() {
                    true
                } else {
                    s.name.to_lowercase().contains(&query) || s.key.to_lowercase().contains(&query) || s.category.to_lowercase().contains(&query)
                }
            })
            .collect();
        let mut grouped: std::collections::BTreeMap<String, Vec<KeyboardShortcut>> = std::collections::BTreeMap::new();
        for s in filtered {
            grouped.entry(s.category.clone()).or_default().push(s);
        }
        rect()
            .width(Size::fill())
            .vertical()
            .child(page_head(KEYBOARD, "Keyboard Shortcuts", "View and manage keybindings"))
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(GAP)
                    .child(
                        tile()
                            .child(tile_head(None, "Search & Add", Some({
                                let mut ao = add_open;
                                ghost_button(if *ao.read() { "Close" } else { "Add Shortcut" }, move || ao.set(!*ao.read()))
                            })))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.)
                                    .content(Content::Flex)
                                    .child(field_label("Filter"))
                                    .child(Input::new(filter).width(Size::flex(1.)).placeholder("Filter by name or key")),
                            )
                            .maybe_child((*add_open.read()).then(|| {
                                let t = t;
                                rect()
                                    .width(Size::fill())
                                    .vertical()
                                    .spacing(8.)
                                    .margin((10., 0., 0., 0.))
                                    .padding(12.)
                                    .background(t.panel_raised)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .corner_radius(12.)
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .horizontal()
                                            .spacing(8.)
                                            .content(Content::Flex)
                                            .child(rect().width(Size::flex(1.)).vertical().spacing(4.).child(field_label("Name")).child(Input::new(new_name).width(Size::fill()).placeholder("e.g. Open Terminal")))
                                            .child(rect().width(Size::flex(1.)).vertical().spacing(4.).child(field_label("Mods (Super Ctrl Alt Shift)")).child(Input::new(new_mods).width(Size::fill()).placeholder("Super"))),
                                    )
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .horizontal()
                                            .spacing(8.)
                                            .content(Content::Flex)
                                            .child(rect().width(Size::flex(1.)).vertical().spacing(4.).child(field_label("Key")).child(Input::new(new_key).width(Size::fill()).placeholder("T")))
                                            .child(rect().width(Size::flex(1.)).vertical().spacing(4.).child(field_label("Command")).child(Input::new(new_cmd).width(Size::fill()).placeholder("ghostty"))),
                                    )
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .horizontal()
                                            .main_align(Alignment::End)
                                            .content(Content::Flex)
                                            .spacing(8.)
                                            .child(secondary_button("Cancel", {
                                                let mut ao = add_open;
                                                move || ao.set(false)
                                            }))
                                            .child({
                                                let mut sc = shortcuts;
                                                let mut ao = add_open;
                                                let mut nm = new_name;
                                                let mut md = new_mods;
                                                let mut ky = new_key;
                                                let mut cm = new_cmd;
                                                primary_button("Add", move || {
                                                    let name = nm.read().trim().to_string();
                                                    let key = ky.read().trim().to_string();
                                                    let mods_raw = md.read().trim().to_string();
                                                    let cmd = cm.read().trim().to_string();
                                                    if name.is_empty() || key.is_empty() || cmd.is_empty() {
                                                        return;
                                                    }
                                                    let mods: Vec<String> = mods_raw.split(|c| c == ',' || c == ' ' || c == '+').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
                                                    let mods = if mods.is_empty() { vec!["Super".to_string()] } else { mods };
                                                    let id = format!("custom_{}_{}", mods.join("_"), key);
                                                    let scut = KeyboardShortcut { id, name, mods, key, command: cmd, category: "Custom".to_string(), is_custom: true };
                                                    let _ = HyprlandBackend.add_custom_shortcut(&scut);
                                                    let updated = HyprlandBackend.get_keyboard_shortcuts();
                                                    sc.set(updated);
                                                    nm.set(String::new());
                                                    md.set(String::new());
                                                    ky.set(String::new());
                                                    cm.set(String::new());
                                                    ao.set(false);
                                                })
                                            }),
                                    )
                                    .into_element()
                            })),
                    )
                    .children({
                        let mut els: Vec<Element> = Vec::new();
                        for (cat, items) in grouped {
                            let mut base = tile().child(tile_head(None, cat.clone(), None::<Element>));
                            for s in items {
                                let sid = s.id.clone();
                                let mods_txt = s.mods.join(" + ");
                                let key_txt = s.key.clone();
                                let combo = if mods_txt.is_empty() { key_txt.clone() } else { format!("{mods_txt} + {key_txt}") };
                                let s_is_custom = s.is_custom;
                                base = base.child(setting_row(
                                    s.name.clone(),
                                    Some(s.command.clone()),
                                    false,
                                    rect()
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .spacing(8.)
                                        .content(Content::Flex)
                                        .child(
                                            rect()
                                                .padding((4., 8.))
                                                .corner_radius(6.)
                                                .background(t.panel_raised)
                                                .border(Border::new().width(1.).fill(t.border))
                                                .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text).text(combo)),
                                        )
                                        .maybe(s_is_custom, |el| {
                                            let mut sc = shortcuts;
                                            let id2 = sid.clone();
                                            el.child(
                                                rect()
                                                    .padding((4., 6.))
                                                    .corner_radius(6.)
                                                    .background(t.panel_raised)
                                                    .border(Border::new().width(1.).fill(t.border))
                                                    .cursor(CursorIcon::Pointer)
                                                    .on_press(move |_| {
                                                        let _ = HyprlandBackend.remove_custom_shortcut(&id2);
                                                        let updated = HyprlandBackend.get_keyboard_shortcuts();
                                                        sc.set(updated);
                                                    })
                                                    .child(label().font_size(10.).color(t.accent_red).text("Remove")),
                                            )
                                        }),
                                ));
                            }
                            els.push(base.into_element());
                        }
                        if els.is_empty() {
                            els.push(tile().child(tile_sub("No shortcuts match filter")).into_element());
                        }
                        els
                    }),
            )
    }
}
