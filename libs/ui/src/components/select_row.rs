use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_SM};
use crate::icons::{icon, CHEVRON_DOWN};

#[derive(Clone, PartialEq)]
pub struct DropdownOption { pub label: String, pub value: String }

#[derive(PartialEq, Clone)]
pub struct DropdownSelect {
    pub selected: String,
    pub options: Vec<DropdownOption>,
    pub on_select: EventHandler<String>,
}

impl Component for DropdownSelect {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut is_open = use_state(|| false);
        let open_val = *is_open.read();
        let selected_val = self.selected.clone();
        let on_select = self.on_select.clone();

        // Selected label to display
        let display_label = self
            .options
            .iter()
            .find(|o| o.value == self.selected || o.label == self.selected)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| {
                if self.selected.is_empty() {
                    "Select an option…".to_string()
                } else {
                    self.selected.clone()
                }
            });

        rect()
            .width(Size::fill())
            .vertical()
            .margin((8., 0., 0., 0.))
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .main_align(Alignment::SpaceBetween)
                    .padding((8., 12.))
                    .background(if open_val { t.bg_hover } else { t.panel_raised })
                    .border(Border::new().width(1.).fill(if open_val { t.accent } else { t.border }))
                    .corner_radius(RADIUS_SM)
                    .cursor(CursorIcon::Pointer)
                    .on_press(move |_| {
                        is_open.set(!open_val);
                    })
                    .child(label().font_size(12.5).color(t.text).text(display_label))
                    .child(icon(CHEVRON_DOWN, 13., if open_val { t.accent } else { t.text_dim })),
            )
            .maybe(open_val, move |el| {
                let is_open_close = is_open;
                let on_select = on_select.clone();
                let opts = self.options.clone();
                let sel_val = selected_val.clone();
                el.child(
                    rect()
                        .width(Size::fill())
                        .vertical()
                        .margin((4., 0., 0., 0.))
                        .padding(4.)
                        .background(t.panel)
                        .border(Border::new().width(1.).fill(t.border))
                        .corner_radius(RADIUS_SM)
                        .shadow(Shadow::new().blur(10.).color(Color::from_argb(64, 0, 0, 0)))
                        .children(opts.into_iter().map(move |opt| {
                            let is_item_selected = opt.value == sel_val || opt.label == sel_val;
                            let opt_val = opt.value.clone();
                            let on_select_item = on_select.clone();
                            let mut is_open_item = is_open_close;

                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .main_align(Alignment::SpaceBetween)
                                .padding((6., 10.))
                                .corner_radius(RADIUS_SM)
                                .background(if is_item_selected { t.bg_active } else { Color::TRANSPARENT })
                                .cursor(CursorIcon::Pointer)
                                .on_press(move |_| {
                                    on_select_item.call(opt_val.clone());
                                    is_open_item.set(false);
                                })
                                .child(
                                    label()
                                        .font_size(12.5)
                                        .color(if is_item_selected { t.accent } else { t.text })
                                        .text(opt.label),
                                )
                                .maybe(is_item_selected, |r| {
                                    r.child(label().font_size(12.).color(t.accent).text("✓"))
                                })
                        })),
                )
            })
    }
}

pub fn dropdown_select(
    selected: String,
    options: Vec<DropdownOption>,
    on_select: EventHandler<String>,
) -> impl IntoElement {
    DropdownSelect {
        selected,
        options,
        on_select,
    }
}

pub fn select_row(
    label_text: impl Into<String>,
    on_press: Option<EventHandler<()>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let label_str = label_text.into();
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .main_align(Alignment::SpaceBetween)
        .padding((9., 11.))
        .background(t.panel_raised)
        .border(Border::new().width(1.).fill(t.border))
        .corner_radius(RADIUS_SM)
        .margin((10., 0., 0., 0.))
        .cursor(CursorIcon::Pointer)
        .map(on_press, |el, cb| {
            el.a11y_role(AccessibilityRole::ComboBox)
                .a11y_focusable(true)
                .on_press(move |_| cb.call(()))
        })
        .child(label().font_size(12.5).color(t.text).text(label_str))
        .child(icon(CHEVRON_DOWN, 13., t.text_dim))
}
