use freya::prelude::*;
use crate::theme::{RADIUS_PILL, RADIUS_SM, use_app_theme};
use crate::components::{dialog_card, field_label, secondary_button, slider_row, small_close_button, tile_sub};
use crate::motion::FadeSlideIn;

pub fn normalize_hex(input: &str) -> String {
    let h = input.trim().trim_start_matches('#');
    if h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("#{}", h.to_ascii_uppercase())
    } else if h.len() == 3 && h.chars().all(|c| c.is_ascii_hexdigit()) {
        let expanded: String = h.chars().flat_map(|c| [c, c]).collect();
        format!("#{}", expanded.to_ascii_uppercase())
    } else {
        String::new()
    }
}

pub fn parse_hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let norm = normalize_hex(hex);
    if norm.len() != 7 {
        return None;
    }
    let r = u8::from_str_radix(&norm[1..3], 16).ok()?;
    let g = u8::from_str_radix(&norm[3..5], 16).ok()?;
    let b = u8::from_str_radix(&norm[5..7], 16).ok()?;
    Some((r, g, b))
}

pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let d = max - min;
    let mut h = if d == 0.0 {
        0.0
    } else if max == rf {
        60.0 * ((gf - bf) / d % 6.0)
    } else if max == gf {
        60.0 * ((bf - rf) / d + 2.0)
    } else {
        60.0 * ((rf - gf) / d + 4.0)
    };
    if h < 0.0 {
        h += 360.0;
    }
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (rf, gf, bf) = match (h as i32 / 60) % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((rf + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((gf + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((bf + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

pub fn hex_to_hsv(hex: &str) -> Option<(f32, f32, f32)> {
    parse_hex_to_rgb(hex).map(|(r, g, b)| rgb_to_hsv(r, g, b))
}

pub fn hsv_to_hex(h: f32, s: f32, v: f32) -> String {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    rgb_to_hex(r, g, b)
}

fn preview_color(hex: &str) -> Color {
    parse_hex_to_rgb(hex)
        .map(|(r, g, b)| Color::from_rgb(r, g, b))
        .unwrap_or(Color::from_rgb(0x1E, 0x1E, 0x2E))
}

const QUICK_PICK: &[&str] = &[
    "#5B5FE9", "#FF6952", "#E3A23D", "#2CA6A0", "#E85A88", "#7B7F87", "#4ECDC4", "#FFE66D",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorPickerTarget {
    Accent,
    Wallpaper,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ColorPickerRequest {
    pub target: ColorPickerTarget,
    pub initial_hex: String,
    pub title: String,
}

pub fn use_init_color_picker() -> State<Option<ColorPickerRequest>> {
    let st = use_state(|| None::<ColorPickerRequest>);
    provide_context(st);
    st
}

fn try_picker_request() -> Option<State<Option<ColorPickerRequest>>> {
    try_consume_context::<State<Option<ColorPickerRequest>>>()
}

pub fn open_color_picker(target: ColorPickerTarget, initial_hex: String, title: impl Into<String>) {
    if let Some(mut req) = try_picker_request() {
        req.set(Some(ColorPickerRequest { target, initial_hex, title: title.into() }));
    }
}

pub fn close_color_picker() {
    if let Some(mut req) = try_picker_request() {
        req.set(None);
    }
}

fn accent_hex_now() -> String {
    let c = use_app_theme().accent;
    rgb_to_hex(c.r(), c.g(), c.b())
}

/// Circle "+" trigger mirroring accent swatches. Render after the swatch row.
pub fn accent_add_circle(size: f32) -> impl IntoElement {
    let t = use_app_theme();
    let initial = accent_hex_now();
    rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .corner_radius(RADIUS_PILL)
        .background(t.panel_raised)
        .border(Border::new().width(1.).fill(t.border))
        .margin((0., 9., 0., 0.))
        .center()
        .cursor(CursorIcon::Pointer)
        .a11y_role(AccessibilityRole::Button)
        .a11y_focusable(true)
        .on_press(move |_| {
            open_color_picker(ColorPickerTarget::Accent, initial.clone(), "Custom accent colour");
        })
        .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_dim).text("+"))
}

/// Rounded-rect "+" trigger mirroring wallpaper defaults. Render as an extra flex cell.
pub fn wallpaper_add_rect(initial_hex: String) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .width(Size::flex(1.))
        .height(Size::px(44.))
        .corner_radius(RADIUS_SM)
        .background(t.panel_raised)
        .border(Border::new().width(1.).fill(t.border))
        .center()
        .cursor(CursorIcon::Pointer)
        .a11y_role(AccessibilityRole::Button)
        .a11y_focusable(true)
        .on_press(move |_| {
            open_color_picker(ColorPickerTarget::Wallpaper, initial_hex.clone(), "Custom background colour");
        })
        .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_dim).text("+"))
}

/// Neutral apply button: fixed text/panel pairing so it never shifts with the picked colour.
fn picker_apply_button(text: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    rect()
        .padding((10., 16.))
        .corner_radius(8.)
        .background(t.text)
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.panel).text(text_str))
}

#[derive(Clone, PartialEq)]
pub struct ColorPickerPanel {
    pub request: ColorPickerRequest,
    pub on_apply: EventHandler<(ColorPickerTarget, String)>,
    pub on_cancel: EventHandler<()>,
}

impl Component for ColorPickerPanel {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let req = self.request.clone();
        let on_apply = self.on_apply.clone();
        let on_cancel = self.on_cancel.clone();

        let init = normalize_hex(&req.initial_hex);
        let init = if init.is_empty() { "#5B5FE9".to_string() } else { init };
        let (init_h, init_s, init_v) = hex_to_hsv(&init).unwrap_or((231.0, 0.62, 0.91));

        let mut hex_state: State<String> = use_state(|| init.clone());
        let hue: State<f64> = use_state(|| init_h as f64);
        let sat: State<f64> = use_state(|| (init_s * 100.0) as f64);
        let bright: State<f64> = use_state(|| (init_v * 100.0) as f64);
        let error: State<Option<String>> = use_state(|| None);

        {
            let mut hue = hue;
            let mut sat = sat;
            let mut bright = bright;
            let mut error = error;
            use_side_effect(move || {
                let hex = hex_state.read().clone();
                if let Some((h, s, v)) = hex_to_hsv(&hex) {
                    hue.set(h as f64);
                    sat.set((s * 100.0) as f64);
                    bright.set((v * 100.0) as f64);
                    error.set(None);
                } else if hex.trim().len() >= 4 {
                    error.set(Some("Use format #RRGGBB".to_string()));
                } else {
                    error.set(None);
                }
            });
        }

        let current_hex = hex_state.read().clone();
        let preview = preview_color(&current_hex);
        let is_valid = !normalize_hex(&current_hex).is_empty();
        let (r, g, b) = parse_hex_to_rgb(&current_hex).unwrap_or((91, 95, 233));

        dialog_card(440.)
            .spacing(12.)
            .child(
                rect()
                    .horizontal()
                    .main_align(Alignment::SpaceBetween)
                    .cross_align(Alignment::Center)
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text).text(req.title.clone()))
                    .child({
                        let on_cancel = on_cancel.clone();
                        small_close_button(move || on_cancel.call(()))
                    }),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(12.)
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(56.))
                            .height(Size::px(56.))
                            .corner_radius(RADIUS_SM)
                            .background(preview)
                            .border(Border::new().width(1.).fill(t.border)),
                    )
                    .child(
                        rect().vertical().spacing(2.)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text)
                                    .text(if is_valid { current_hex.clone() } else { "Invalid colour".to_string() }),
                            )
                            .child(
                                label()
                                    .font_size(12.)
                                    .color(t.text_dim)
                                    .text(format!("R {r}  G {g}  B {b}")),
                            ),
                    ),
            )
            .child(
                rect().width(Size::fill()).vertical().spacing(4.).child(field_label("Hex")).child(
                    Input::new(hex_state).width(Size::fill()).placeholder("#RRGGBB"),
                ),
            )
            .maybe_child(error.read().clone().map(|msg| {
                label().font_size(12.).color(Color::from_rgb(235, 80, 80)).text(msg)
            }))
            .child(
                rect().width(Size::fill()).vertical().spacing(4.).child(field_label("Hue")).child({
                    let mut hue = hue;
                    let mut hex_state = hex_state;
                    let sat_v = *sat.read();
                    let bright_v = *bright.read();
                    slider_row(None, *hue.read() / 360.0 * 100.0, move |v| {
                        let h = (v / 100.0 * 360.0).clamp(0.0, 360.0);
                        hue.set(h);
                        hex_state.set(hsv_to_hex(h as f32, (sat_v / 100.0) as f32, (bright_v / 100.0) as f32));
                    })
                }),
            )
            .child(
                rect().width(Size::fill()).vertical().spacing(4.).child(field_label("Saturation")).child({
                    let mut sat = sat;
                    let mut hex_state = hex_state;
                    let hue_v = *hue.read();
                    let bright_v = *bright.read();
                    slider_row(None, *sat.read(), move |v| {
                        sat.set(v);
                        hex_state.set(hsv_to_hex(hue_v as f32, (v / 100.0) as f32, (bright_v / 100.0) as f32));
                    })
                }),
            )
            .child(
                rect().width(Size::fill()).vertical().spacing(4.).child(field_label("Brightness")).child({
                    let mut bright = bright;
                    let mut hex_state = hex_state;
                    let hue_v = *hue.read();
                    let sat_v = *sat.read();
                    slider_row(None, *bright.read(), move |v| {
                        bright.set(v);
                        hex_state.set(hsv_to_hex(hue_v as f32, (sat_v / 100.0) as f32, (v / 100.0) as f32));
                    })
                }),
            )
            .child(
                rect().width(Size::fill()).vertical().spacing(6.).child(field_label("Quick picks")).child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .spacing(8.)
                        .content(Content::Flex)
                        .children(QUICK_PICK.iter().map(|hex| {
                            let col = preview_color(hex);
                            let is_sel = current_hex.eq_ignore_ascii_case(hex);
                            let hex_owned = hex.to_string();
                            rect()
                                .width(Size::flex(1.))
                                .height(Size::px(32.))
                                .corner_radius(RADIUS_SM)
                                .background(col)
                                .border(
                                    Border::new()
                                        .width(if is_sel { 2. } else { 1. })
                                        .fill(if is_sel { t.accent } else { t.border }),
                                )
                                .cursor(CursorIcon::Pointer)
                                .a11y_role(AccessibilityRole::RadioButton)
                                .a11y_focusable(true)
                                .on_press(move |_| {
                                    hex_state.set(hex_owned.clone());
                                })
                        })),
                ),
            )
            .child(tile_sub("Pick with sliders or paste a hex value, then Apply."))
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .spacing(10.)
                    .main_align(Alignment::End)
                    .cross_align(Alignment::Center)
                    .content(Content::Flex)
                    .child({
                        let on_cancel = on_cancel.clone();
                        secondary_button("Cancel", move || on_cancel.call(()))
                    })
                    .child({
                        let hex = current_hex.clone();
                        let valid = is_valid;
                        let target = req.target;
                        picker_apply_button("Apply", move || {
                            if valid {
                                on_apply.call((target, normalize_hex(&hex)));
                            }
                        })
                    }),
            )
    }
}

#[derive(Clone, PartialEq)]
pub struct ColorPickerModal {
    pub on_apply: EventHandler<(ColorPickerTarget, String)>,
}

impl Component for ColorPickerModal {
    fn render(&self) -> impl IntoElement {
        let req_state = try_picker_request();
        let Some(req_state) = req_state else {
            return rect().into_element();
        };
        let req_opt = req_state.read().clone();
        let Some(req) = req_opt else {
            return rect().into_element();
        };
        let on_apply = self.on_apply.clone();
        let req_close = req_state;
        let req_close2 = req_state;

        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .on_press(move |_| {
                let mut r = req_close;
                r.set(None);
            })
            .child(
                rect()
                    .on_press(|e: Event<PressEventData>| {
                        e.stop_propagation();
                    })
                    .child(
                        FadeSlideIn::new().scale_from(0.97).child(ColorPickerPanel {
                            request: req,
                            on_apply,
                            on_cancel: (move |()| {
                                let mut r = req_close2;
                                r.set(None);
                            })
                            .into(),
                        }),
                    ),
            )
            .into_element()
    }
}
