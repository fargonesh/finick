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
    pub x: i32,
    pub y: i32,
    pub identify_active: bool,
    pub logical_w: i32,
    pub logical_h: i32,
}

impl DisplayMockItem {
    pub fn new(name: impl Into<String>, height_px: f32) -> Self {
        Self { name: name.into(), description: String::new(), resolution: String::new(), height_px, transform: 0, is_selected: false, x: 0, y: 0, identify_active: false, logical_w: 1920, logical_h: 1080 }
    }
}

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
                        .child(label().font_size(11.).color(t.text_dim).text(disp.name)),
                )
        }))
}

#[derive(PartialEq)]
pub struct DraggableDisplaysMock {
    pub displays: Vec<DisplayMockItem>,
    pub dragged_name: State<Option<String>>,
    pub on_swap: EventHandler<(String, String)>,
    pub on_select: Option<EventHandler<String>>,
    pub on_drag_end: Option<EventHandler<()>>,
    pub on_position: Option<EventHandler<(String, i32, i32)>>,
}

impl Component for DraggableDisplaysMock {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let drag_start: State<Option<(f32,f32)>> = use_state(|| None);
        let drag_origin: State<std::collections::HashMap<String,(i32,i32)>> = use_state(Default::default);
        let is_dragging_any = self.dragged_name.read().is_some();
        let current_dragged = self.dragged_name.read().clone();
        let on_swap = self.on_swap.clone();
        let on_select = self.on_select.clone();
        let on_drag_end = self.on_drag_end.clone();
        let on_position = self.on_position.clone();
        let mut dragged_state = self.dragged_name;
        if self.displays.is_empty() {
            return rect().width(Size::fill()).center().padding((18., 12.)).child(label().font_size(12.).color(t.text_dim).text("No displays detected")).into_element();
        }
        rect()
            .width(Size::fill())
            .vertical()
            .spacing(8.)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(140.))
                    .corner_radius(RADIUS_SM)
                    .background(t.bg)
                    .border(Border::new().width(1.).fill(t.border))
                    .padding((12., 12.))
                    .on_global_pointer_press(move |_| {
                        let active = dragged_state.read().clone();
                        if active.is_some() {
                            dragged_state.set(None);
                            if let Some(ref cb) = on_drag_end { cb.call(()); }
                        }
                    })
                    .on_global_pointer_move({
                        let on_pos = on_position.clone();
                        let dragged = self.dragged_name;
                        let start = drag_start;
                        let origin = drag_origin;
                        let displays_for_check = self.displays.clone();
                        move |e: Event<PointerEventData>| {
                            let active = dragged.read().clone();
                            if let Some(name) = active {
                                if let Some((sx, sy)) = *start.read() {
                                    let cur = e.data().global_location();
                                    let dx = cur.x as f32 - sx;
                                    let dy = cur.y as f32 - sy;
                                    if let Some(ref cb) = on_pos {
                                        let map = origin.read().clone();
                                        let (nx, ny) = if let Some((ox, oy)) = map.get(&name) {
                                            (ox + (dx * 6.0) as i32, oy + (dy * 6.0) as i32)
                                        } else {
                                            ((dx * 6.0) as i32, (dy * 6.0) as i32)
                                        };
                                        let self_item = displays_for_check.iter().find(|d| d.name==name);
                                        let (sw, sh) = self_item.map(|d| (d.logical_w, d.logical_h)).unwrap_or((1920,1080));
                                        let (mut snap_x, mut snap_y) = (nx, ny);
                                        if displays_for_check.len() > 1 {
                                            let mut best: Option<(i32,i32,i32)> = None;
                                            for d in &displays_for_check {
                                                if d.name == name { continue; }
                                                let (ox, oy) = map.get(&d.name).cloned().unwrap_or((d.x,d.y));
                                                let (ow, oh) = (d.logical_w, d.logical_h);
                                                for (cx, cy) in [(ox+ow, oy), (ox - sw, oy), (ox, oy+oh), (ox, oy - sh)] {
                                                    let dx = nx - cx;
                                                    let dy = ny - cy;
                                                    let dist = dx*dx + dy*dy;
                                                    if best.is_none() || dist < best.unwrap().0 { best = Some((dist, cx, cy)); }
                                                }
                                            }
                                            if let Some((_, bx, by)) = best {
                                                snap_x = bx;
                                                snap_y = by;
                                            }
                                        } else {
                                            snap_x = 0;
                                            snap_y = 0;
                                        }
                                        let max_x = 3000;
                                        let max_y = 2000;
                                        snap_x = snap_x.clamp(-max_x, max_x);
                                        snap_y = snap_y.clamp(-max_y, max_y);
                                        cb.call((name.clone(), snap_x, snap_y));
                                    }
                                    e.prevent_default();
                                }
                            }
                        }
                    })
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .horizontal()
                            .spacing(16.)
                            .cross_align(Alignment::Center)
                            .content(Content::Flex)
                            .children(self.displays.iter().enumerate().map(|(idx, disp)| {
                                let is_dragged = current_dragged.as_deref() == Some(disp.name.as_str());
                                let is_selected = disp.is_selected;
                                let is_portrait = disp.transform == 1 || disp.transform == 3;
                                let screen_h = if is_portrait { 96. } else { 72. };
                                let screen_w = if is_portrait { 72. } else { 110. };
                                let border_color = if is_dragged || is_selected { t.accent } else { t.border };
                                let border_width = if is_selected || is_dragged { 2. } else { 1. };
                                let bg = if is_selected { t.bg_selected } else { t.panel_raised };
                                let card_cursor = if is_dragging_any { CursorIcon::Grabbing } else { CursorIcon::Grab };
                                let name_key = format!("disp-card-{}", disp.name);
                                let disp_name = disp.name.clone();
                                let disp_res = disp.resolution.clone();
                                let on_swap_clone = on_swap.clone();
                                let on_select_clone = on_select.clone();
                                let mut dragged_state_down = self.dragged_name;
                                let dragged_state_enter = self.dragged_name;
                                let mut start_down = drag_start;
                                let mut origin_down = drag_origin;
                                let dname_down = disp_name.clone();
                                let dname_enter = disp_name.clone();
                                let displays_snapshot = self.displays.clone();
                                let x_off = (disp.x as f32 / 18.0).clamp(-32.0, 32.0);
                                let y_off = (disp.y as f32 / 18.0).clamp(-28.0, 28.0);
                                rect()
                                    .key(name_key)
                                    .width(Size::px(screen_w))
                                    .height(Size::px(screen_h + 28.))
                                    .margin((y_off, 0., 0., x_off))
                                    .vertical()
                                    .cross_align(Alignment::Center)
                                    .cursor(card_cursor)
                                    .on_pointer_down(move |e: Event<PointerEventData>| {
                                        if e.data().is_primary() {
                                            let gp = e.data().global_location();
                                            start_down.set(Some((gp.x as f32, gp.y as f32)));
                                            let mut map = std::collections::HashMap::new();
                                            for d in &displays_snapshot {
                                                map.insert(d.name.clone(), (d.x, d.y));
                                            }
                                            origin_down.set(map);
                                            dragged_state_down.set(Some(dname_down.clone()));
                                            if let Some(ref sel) = on_select_clone { sel.call(dname_down.clone()); }
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
                                                rect()
                                                    .width(Size::fill())
                                                    .horizontal()
                                                    .main_align(Alignment::SpaceBetween)
                                                    .cross_align(Alignment::Center)
                                                    .child(
                                                        rect().padding((2., 6.)).corner_radius(4.).background(if is_selected { t.accent } else { t.track })
                                                            .child(label().font_size(9.).font_weight(FontWeight::BOLD).color(if is_selected { t.bg } else { t.text_dim }).text(format!("{}", idx+1)))
                                                    )
                                                    .maybe_child(is_portrait.then(|| rect().padding((2.,5.)).corner_radius(4.).background(t.bg_selected).child(label().font_size(8.).color(t.accent).text(if disp.transform==1{"90°"}else{"270°"}))))
                                            )
                                            .child(
                                                rect().vertical().cross_align(Alignment::Center).spacing(2.)
                                                    .child(label().font_size(13.).font_weight(FontWeight::BOLD).color(if is_selected { t.text_primary } else { t.text }).text(disp_name.clone()))
                                                    .maybe_child((!disp_res.is_empty()).then(|| label().font_size(10.).color(t.text_dim).text(disp_res.clone())))
                                                    .maybe_child(disp.identify_active.then(|| rect().margin((4.,0.,0.,0.)).padding((4.,8.)).corner_radius(4.).background(t.accent).child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.bg).text(format!("{} - {}", idx+1, disp_name.clone())))))
                                            )
                                            .child(rect().center().child(label().font_size(9.).color(if is_dragged { t.accent } else { t.text_disabled }).text(if is_dragged {"Moving"} else {"Drag to move"})))
                                    )
                                    .child(
                                        rect().width(Size::fill()).center().margin((4.,0.,0.,0.))
                                            .child(label().font_size(10.).font_weight(if is_selected {FontWeight::MEDIUM} else {FontWeight::NORMAL}).color(if is_selected {t.accent} else {t.text_dim}).text(disp_name))
                                    )
                            }))
                    )
            )
            .into_element()
    }
}

pub fn draggable_displays_mock(
    displays: Vec<DisplayMockItem>,
    dragged_name: State<Option<String>>,
    on_swap: impl Into<EventHandler<(String, String)>>,
    on_select: impl Into<Option<EventHandler<String>>>,
    on_drag_end: impl Into<Option<EventHandler<()>>>,
) -> impl IntoElement {
    DraggableDisplaysMock { displays, dragged_name, on_swap: on_swap.into(), on_select: on_select.into(), on_drag_end: on_drag_end.into(), on_position: None }
}

pub fn draggable_displays_mock_with_position(
    displays: Vec<DisplayMockItem>,
    dragged_name: State<Option<String>>,
    on_swap: impl Into<EventHandler<(String, String)>>,
    on_select: impl Into<Option<EventHandler<String>>>,
    on_drag_end: impl Into<Option<EventHandler<()>>>,
    on_position: impl Into<EventHandler<(String,i32,i32)>>,
) -> impl IntoElement {
    DraggableDisplaysMock { displays, dragged_name, on_swap: on_swap.into(), on_select: on_select.into(), on_drag_end: on_drag_end.into(), on_position: Some(on_position.into()) }
}
