use std::collections::HashMap;

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

const CANVAS_W: f32 = 760.;
const CANVAS_H: f32 = 220.;
const MAX_SCALE: f32 = 0.12;
const DRAG_THRESHOLD: f32 = 3.;

fn overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

fn snap_position(
    name: &str,
    nx: i32,
    ny: i32,
    displays: &[DisplayMockItem],
    origin: &HashMap<String, (i32, i32)>,
) -> (i32, i32) {
    let (sw, sh) = displays
        .iter()
        .find(|d| d.name == name)
        .map(|d| (d.logical_w, d.logical_h))
        .unwrap_or((1920, 1080));
    let others: Vec<(i32, i32, i32, i32)> = displays
        .iter()
        .filter(|d| d.name != name)
        .map(|d| {
            let (ox, oy) = origin.get(&d.name).cloned().unwrap_or((d.x, d.y));
            (ox, oy, d.logical_w, d.logical_h)
        })
        .collect();
    let mut best: Option<(i64, i32, i32)> = None;
    for &(ox, oy, ow, oh) in &others {
        for (cx, cy) in [
            (ox + ow, oy),
            (ox - sw, oy),
            (ox, oy + oh),
            (ox, oy - sh),
        ] {
            if others.iter().any(|o| overlaps((cx, cy, sw, sh), *o)) {
                continue;
            }
            let dx = nx.saturating_sub(cx) as i64;
            let dy = ny.saturating_sub(cy) as i64;
            let dist = dx * dx + dy * dy;
            if best.map_or(true, |(current, _, _)| dist < current) {
                best = Some((dist, cx, cy));
            }
        }
    }
    best.map(|(_, x, y)| (x, y))
        .unwrap_or_else(|| origin.get(name).cloned().unwrap_or((0, 0)))
}

#[derive(PartialEq)]
pub struct DraggableDisplaysMock {
    pub displays: Vec<DisplayMockItem>,
    pub dragged_name: State<Option<String>>,
    pub locked: bool,
    pub on_select: Option<EventHandler<String>>,
    pub on_drag_end: Option<EventHandler<()>>,
    pub on_position: Option<EventHandler<(String, i32, i32)>>,
}

impl Component for DraggableDisplaysMock {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let drag_start: State<Option<(f32, f32)>> = use_state(|| None);
        let drag_origin: State<HashMap<String, (i32, i32)>> = use_state(Default::default);
        let drag_moved: State<bool> = use_state(|| false);
        let locked = self.locked;
        let is_dragging_any = !locked && self.dragged_name.read().is_some();
        let current_dragged = self.dragged_name.read().clone();
        let on_select = self.on_select.clone();
        let on_drag_end = self.on_drag_end.clone();
        let on_position = self.on_position.clone();
        if self.displays.is_empty() {
            return rect().width(Size::fill()).center().padding((18., 12.)).child(label().font_size(12.).color(t.text_dim).text("No displays detected")).into_element();
        }

        let min_x = self.displays.iter().map(|d| d.x).min().unwrap_or(0);
        let min_y = self.displays.iter().map(|d| d.y).min().unwrap_or(0);
        let max_x = self.displays.iter().map(|d| d.x + d.logical_w).max().unwrap_or(min_x + 1920);
        let max_y = self.displays.iter().map(|d| d.y + d.logical_h).max().unwrap_or(min_y + 1080);
        let max_h = self.displays.iter().map(|d| d.logical_h).max().unwrap_or(1080);
        let scale = (CANVAS_W / (max_x - min_x).max(1) as f32)
            .min(CANVAS_H / (max_y - min_y).max(1) as f32)
            .min((CANVAS_H - 18.) / max_h.max(1) as f32)
            .min(MAX_SCALE);

        rect()
            .width(Size::fill())
            .vertical()
            .spacing(8.)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(260.))
                    .corner_radius(RADIUS_SM)
                    .background(t.bg)
                    .border(Border::new().width(1.).fill(t.border))
                    .padding((12., 12.))
                    .on_global_pointer_press({
                        let mut dragged = self.dragged_name;
                        let mut start = drag_start;
                        let mut moved = drag_moved;
                        let on_end = on_drag_end.clone();
                        move |_| {
                            let active = dragged.read().clone();
                            if active.is_none() {
                                return;
                            }
                            let was_moved = *moved.read();
                            dragged.set(None);
                            start.set(None);
                            moved.set(false);
                            if was_moved {
                                if let Some(ref cb) = on_end {
                                    cb.call(());
                                }
                            }
                        }
                    })
                    .on_global_pointer_move({
                        let on_pos = on_position.clone();
                        let dragged = self.dragged_name;
                        let start = drag_start;
                        let origin = drag_origin;
                        let mut moved = drag_moved;
                        let displays_for_check = self.displays.clone();
                        move |e: Event<PointerEventData>| {
                            let active = dragged.read().clone();
                            if let Some(name) = active
                                && let Some((sx, sy)) = *start.read() {
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
                    })
                    .child(
                        rect()
                            .width(Size::px(CANVAS_W))
                            .height(Size::px(CANVAS_H))
                            .children(self.displays.iter().enumerate().map(|(idx, disp)| {
                                let is_dragged = current_dragged.as_deref() == Some(disp.name.as_str());
                                let is_selected = disp.is_selected;
                                let is_portrait = disp.transform == 1 || disp.transform == 3;
                                let card_w = (disp.logical_w as f32 * scale).round().max(56.);
                                let card_h = (disp.logical_h as f32 * scale).round().max(40.);
                                let border_color = if is_dragged || is_selected { t.accent } else { t.border };
                                let border_width = if is_selected || is_dragged { 2. } else { 1. };
                                let bg = if is_selected { t.bg_selected } else { t.panel_raised };
                                let card_cursor = if locked {
                                    CursorIcon::Default
                                } else if is_dragging_any {
                                    CursorIcon::Grabbing
                                } else {
                                    CursorIcon::Grab
                                };
                                let name_key = format!("disp-card-{}", disp.name);
                                let disp_name = disp.name.clone();
                                let disp_res = disp.resolution.clone();
                                let on_select_click = on_select.clone();
                                let mut dragged_state_down = self.dragged_name;
                                let mut start_down = drag_start;
                                let mut origin_down = drag_origin;
                                let mut moved_down = drag_moved;
                                let dname_down = disp_name.clone();
                                let displays_snapshot = self.displays.clone();
                                let left = ((disp.x - min_x) as f32 * scale).clamp(0., (CANVAS_W - card_w).max(0.));
                                let top = ((disp.y - min_y) as f32 * scale).clamp(0., (CANVAS_H - card_h).max(0.));
                                rect()
                                    .key(name_key)
                                    .position(Position::new_absolute().left(left).top(top))
                                    .width(Size::px(card_w))
                                    .height(Size::px(card_h + 16.))
                                    .vertical()
                                    .cross_align(Alignment::Center)
                                    .cursor(card_cursor)
                                    .on_pointer_down(move |e: Event<PointerEventData>| {
                                        if locked || !e.data().is_primary() {
                                            return;
                                        }
                                    })
                                    .on_pointer_enter(move |_| {
                                        let active = dragged_state_enter.read().clone();
                                        if let Some(from_name) = active
                                            && from_name != dname_enter {
                                                on_swap_clone.call((from_name, dname_enter.clone()));
                                            }
                                        })
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .height(Size::px(card_h))
                                            .corner_radius(RADIUS_SM)
                                            .background(bg)
                                            .border(Border::new().width(border_width).fill(border_color))
                                            .padding((4., 6.))
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
                                                        rect().padding((2., 5.)).corner_radius(4.).background(if is_selected { t.accent } else { t.track })
                                                            .child(label().font_size(9.).font_weight(FontWeight::BOLD).color(if is_selected { t.bg } else { t.text_dim }).text(format!("{}", idx + 1)))
                                                    )
                                                    .maybe_child(is_portrait.then(|| rect().padding((2., 5.)).corner_radius(4.).background(t.bg_selected).child(label().font_size(8.).color(t.accent).text(if disp.transform == 1 { "90°" } else { "270°" }))))
                                            )
                                            .child(
                                                rect().center().child(if disp.identify_active {
                                                    rect().padding((2., 4.)).corner_radius(4.).background(t.accent)
                                                        .child(label().font_size(9.).font_weight(FontWeight::BOLD).color(t.bg).text(format!("{} - {}", idx + 1, disp.name)))
                                                        .into_element()
                                                } else {
                                                    label().font_size(9.).color(t.text_dim).text(disp_res.clone()).into_element()
                                                })
                                            )
                                    )
                                    .child(
                                        rect().width(Size::fill()).height(Size::px(16.)).center()
                                            .child(label().font_size(10.).font_weight(if is_selected { FontWeight::MEDIUM } else { FontWeight::NORMAL }).color(if is_dragged { t.accent } else if is_selected { t.accent } else { t.text_dim }).text(if is_dragged { "Moving".to_string() } else { disp_name }))
                                    )
                            }))
                    )
            )
            .into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, x: i32, y: i32, w: i32, h: i32) -> DisplayMockItem {
        let mut display = DisplayMockItem::new(name, 72.);
        display.x = x;
        display.y = y;
        display.logical_w = w;
        display.logical_h = h;
        display
    }

    fn origin(displays: &[DisplayMockItem]) -> HashMap<String, (i32, i32)> {
        displays.iter().map(|d| (d.name.clone(), (d.x, d.y))).collect()
    }

    #[test]
    fn snaps_to_nearest_edge() {
        let displays = vec![item("DP-1", 0, 0, 1920, 1080), item("HDMI-1", 1920, 0, 1920, 1080)];
        let map = origin(&displays);
        assert_eq!(snap_position("HDMI-1", 1950, 40, &displays, &map), (1920, 0));
    }

    #[test]
    fn snaps_even_when_pointer_is_far() {
        let displays = vec![item("DP-1", 0, 0, 1920, 1080), item("HDMI-1", 1920, 0, 1920, 1080)];
        let map = origin(&displays);
        let (x, y) = snap_position("HDMI-1", 6000, 3000, &displays, &map);
        assert!(!overlaps((x, y, 1920, 1080), (0, 0, 1920, 1080)));
    }

    #[test]
    fn snaps_to_vertical_edge_below() {
        let displays = vec![item("DP-1", 0, 0, 1920, 1080), item("HDMI-1", 0, 1080, 1920, 1080)];
        let map = origin(&displays);
        assert_eq!(snap_position("HDMI-1", 80, 1100, &displays, &map), (0, 1080));
    }

    #[test]
    fn snapped_position_never_overlaps_another_display() {
        let displays = vec![
            item("DP-1", 0, 0, 1920, 1080),
            item("DP-2", 1920, 0, 1920, 1080),
            item("HDMI-1", 3840, 0, 1920, 1080),
        ];
        let map = origin(&displays);
        let (x, y) = snap_position("HDMI-1", 700, 400, &displays, &map);
        for d in displays.iter().filter(|d| d.name != "HDMI-1") {
            assert!(!overlaps((x, y, 1920, 1080), (d.x, d.y, d.logical_w, d.logical_h)));
        }
    }

    #[test]
    fn single_display_uses_origin() {
        let displays = vec![item("DP-1", 0, 0, 1920, 1080)];
        let map = origin(&displays);
        assert_eq!(snap_position("DP-1", 600, 300, &displays, &map), (0, 0));
    }
}

pub fn draggable_displays_mock(
    displays: Vec<DisplayMockItem>,
    dragged_name: State<Option<String>>,
    locked: bool,
    on_select: impl Into<Option<EventHandler<String>>>,
    on_drag_end: impl Into<Option<EventHandler<()>>>,
) -> impl IntoElement {
    DraggableDisplaysMock { displays, dragged_name, locked, on_select: on_select.into(), on_drag_end: on_drag_end.into(), on_position: None }
}

pub fn draggable_displays_mock_with_position(
    displays: Vec<DisplayMockItem>,
    dragged_name: State<Option<String>>,
    locked: bool,
    on_select: impl Into<Option<EventHandler<String>>>,
    on_drag_end: impl Into<Option<EventHandler<()>>>,
    on_position: impl Into<EventHandler<(String, i32, i32)>>,
) -> impl IntoElement {
    DraggableDisplaysMock { displays, dragged_name, locked, on_select: on_select.into(), on_drag_end: on_drag_end.into(), on_position: Some(on_position.into()) }
}
