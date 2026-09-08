use freya::prelude::*;
use crate::theme::{RADIUS_SM, use_app_theme};

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayMockItem {
    pub name: String,
    pub description: String,
    pub resolution: String,
    pub height_px: f32,
    pub transform: u8,
    pub is_selected: bool,
}

impl DisplayMockItem {
    pub fn new(name: impl Into<String>, height_px: f32) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            resolution: String::new(),
            height_px,
            transform: 0,
            is_selected: false,
        }
    }
}

/// Static display monitor mock arrangement matching .displays-mock in ui_demo.html
pub fn displays_mock(displays: Vec<DisplayMockItem>) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::End)
        .spacing(14.)
        .margin((4., 0., 0., 0.))
        .content(Content::Flex)
        .children(displays.into_iter().map(move |disp| {
            rect()
                .width(Size::flex(1.))
                .vertical()
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(disp.height_px))
                        .corner_radius(RADIUS_SM)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .center()
                        .margin((6., 0., 0., 0.))
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_dim)
                                .text(disp.name),
                        ),
                )
        }))
}

/// Draggable monitors arrangement component.
/// Supports clicking/grabbing the display screen directly to reorder monitors with live swapping,
/// click to select for per-monitor configuration, and orientation previews.
#[derive(PartialEq)]
pub struct DraggableDisplaysMock {
    pub displays: Vec<DisplayMockItem>,
    pub dragged_name: State<Option<String>>,
    pub on_swap: EventHandler<(String, String)>,
    pub on_select: Option<EventHandler<String>>,
    pub on_drag_end: Option<EventHandler<()>>,
}

impl Component for DraggableDisplaysMock {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        if self.displays.is_empty() {
            return rect()
                .width(Size::fill())
                .center()
                .padding((18., 12.))
                .child(
                    label()
                        .font_size(12.)
                        .color(t.text_dim)
                        .text("No displays detected"),
                )
                .into_element();
        }

        let is_dragging_any = self.dragged_name.read().is_some();
        let current_dragged = self.dragged_name.read().clone();
        let on_swap = self.on_swap.clone();
        let on_select = self.on_select.clone();
        let on_drag_end = self.on_drag_end.clone();
        let mut dragged_state = self.dragged_name;

        rect()
            .width(Size::fill())
            .horizontal()
            .cross_align(Alignment::End)
            .spacing(16.)
            .margin((4., 0., 0., 0.))
            .content(Content::Flex)
            .on_global_pointer_press(move |_| {
                let active = dragged_state.read().clone();
                if active.is_some() {
                    dragged_state.set(None);
                    if let Some(ref cb) = on_drag_end {
                        cb.call(());
                    }
                }
            })
            .children(self.displays.iter().enumerate().map(|(idx, disp)| {
                let is_dragged = current_dragged.as_deref() == Some(disp.name.as_str());
                let is_selected = disp.is_selected;
                let is_portrait = disp.transform == 1 || disp.transform == 3;
                let screen_h = if is_portrait { 96. } else { 72. };

                let border_color = if is_dragged || is_selected {
                    t.accent
                } else {
                    t.border
                };
                let border_width = if is_selected || is_dragged { 2. } else { 1. };
                let bg = if is_selected {
                    t.bg_selected
                } else {
                    t.panel_raised
                };

                let card_cursor = if is_dragging_any {
                    CursorIcon::Grabbing
                } else {
                    CursorIcon::Grab
                };

                let name_key = format!("disp-card-{}", disp.name);
                let disp_name = disp.name.clone();
                let disp_res = disp.resolution.clone();
                let disp_desc = disp.description.clone();

                let on_swap_clone = on_swap.clone();
                let on_select_clone = on_select.clone();
                let mut dragged_state_down = self.dragged_name;
                let dragged_state_enter = self.dragged_name;
                let dname_down = disp_name.clone();
                let dname_enter = disp_name.clone();

                rect()
                    .key(name_key)
                    .width(Size::flex(1.))
                    .vertical()
                    .cross_align(Alignment::Center)
                    .cursor(card_cursor)
                    .on_pointer_down(move |e: Event<PointerEventData>| {
                        if e.data().is_primary() {
                            dragged_state_down.set(Some(dname_down.clone()));
                            if let Some(ref sel) = on_select_clone {
                                sel.call(dname_down.clone());
                            }
                        }
                    })
                    .on_pointer_enter(move |_| {
                        let active = dragged_state_enter.read().clone();
                        if let Some(from_name) = active {
                            if from_name != dname_enter {
                                on_swap_clone.call((from_name, dname_enter.clone()));
                            }
                        }
                    })
                    .child(
                        // Display monitor screen frame - fully interactive and draggable
                        rect()
                            .width(Size::fill())
                            .height(Size::px(screen_h))
                            .corner_radius(RADIUS_SM)
                            .background(bg)
                            .border(Border::new().width(border_width).fill(border_color))
                            .padding((6., 8.))
                            .vertical()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                // Top row inside screen: display number pill + portrait tag
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .main_align(Alignment::SpaceBetween)
                                    .cross_align(Alignment::Center)
                                    .child(
                                        rect()
                                            .padding((2., 6.))
                                            .corner_radius(4.)
                                            .background(if is_selected { t.accent } else { t.track })
                                            .child(
                                                label()
                                                    .font_size(9.)
                                                    .font_weight(FontWeight::BOLD)
                                                    .color(if is_selected { t.bg } else { t.text_dim })
                                                    .text(format!("{}", idx + 1)),
                                            ),
                                    )
                                    .maybe_child(is_portrait.then(|| {
                                        rect()
                                            .padding((2., 5.))
                                            .corner_radius(4.)
                                            .background(t.bg_selected)
                                            .child(
                                                label()
                                                    .font_size(8.)
                                                    .color(t.accent)
                                                    .text(if disp.transform == 1 { "90°" } else { "270°" }),
                                            )
                                    })),
                            )
                            .child(
                                // Center of screen: Monitor Name & Resolution
                                rect()
                                    .vertical()
                                    .cross_align(Alignment::Center)
                                    .spacing(2.)
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(if is_selected { t.text_primary } else { t.text })
                                            .text(disp_name.clone()),
                                    )
                                    .maybe_child((!disp_res.is_empty()).then(|| {
                                        label()
                                            .font_size(10.)
                                            .color(t.text_dim)
                                            .text(disp_res.clone())
                                    })),
                            )
                            .child(
                                // Bottom hint inside screen
                                rect()
                                    .center()
                                    .child(
                                        label()
                                            .font_size(9.)
                                            .color(if is_dragged { t.accent } else { t.text_disabled })
                                            .text(if is_dragged { "Moving" } else { "Drag to move" }),
                                    ),
                            ),
                    )
                    .child(
                        // Monitor stand neck
                        rect()
                            .width(Size::px(12.))
                            .height(Size::px(6.))
                            .background(t.border),
                    )
                    .child(
                        // Monitor stand base
                        rect()
                            .width(Size::px(36.))
                            .height(Size::px(3.))
                            .corner_radius(1.5)
                            .background(t.border),
                    )
                    .child(
                        // Monitor footer description or name
                        rect()
                            .width(Size::fill())
                            .center()
                            .margin((4., 0., 0., 0.))
                            .child(
                                label()
                                    .font_size(10.)
                                    .font_weight(if is_selected { FontWeight::MEDIUM } else { FontWeight::NORMAL })
                                    .color(if is_selected { t.accent } else { t.text_dim })
                                    .text(if !disp_desc.is_empty() {
                                        disp_desc.clone()
                                    } else {
                                        disp_name
                                    }),
                            ),
                    )
            }))
            .into_element()
    }
}

/// Full interactive draggable monitors mock with selection and drag end callbacks.
pub fn draggable_displays_mock(
    displays: Vec<DisplayMockItem>,
    dragged_name: State<Option<String>>,
    on_swap: impl Into<EventHandler<(String, String)>>,
    on_select: impl Into<Option<EventHandler<String>>>,
    on_drag_end: impl Into<Option<EventHandler<()>>>,
) -> impl IntoElement {
    DraggableDisplaysMock {
        displays,
        dragged_name,
        on_swap: on_swap.into(),
        on_select: on_select.into(),
        on_drag_end: on_drag_end.into(),
    }
}

