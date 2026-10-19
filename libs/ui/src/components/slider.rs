use freya::prelude::*;
use crate::theme::use_app_theme;
use crate::icons::{icon, SOUND};

pub fn slider_row(
    icon_svg: Option<&'static str>,
    value: f64,
    on_change: impl FnMut(f64) + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .maybe_child(icon_svg.map(|svg| icon(svg, 15., t.text_dim)))
        .child(
            rect()
                .width(Size::flex(1.))
                .content(Content::Flex)
                .child(Slider::new(on_change).value(value).theme(SliderThemePartial::new().background(t.track).thumb_inner_background(t.accent).thumb_background(Color::from_rgb(255,255,255)).border_fill(t.accent))),
        )
        .content(Content::Flex)
}

#[derive(Clone, PartialEq)]
struct MuteButton {
    muted: bool,
    on_toggle: EventHandler<bool>,
}

impl Component for MuteButton {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let is_hovered = *hovered.read();
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_toggle = self.on_toggle.clone();
        let is_muted = self.muted;
        let bg = if is_muted {
            if is_hovered { t.accent_blue_hover } else { t.accent }
        } else if is_hovered {
            t.track
        } else {
            t.panel_raised
        };

        rect()
            .width(Size::px(32.))
            .height(Size::px(32.))
            .corner_radius(999.)
            .background(bg)
            .border(Border::new().width(1.).fill(if is_muted { t.accent } else { t.border }))
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_role(AccessibilityRole::Button)
            .a11y_focusable(true)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_toggle.call(!is_muted))
            .child(icon(SOUND, 14., if is_muted { Color::WHITE } else { t.text_dim }))
    }
}

pub fn slider_row_with_mute(
    icon_svg: Option<&'static str>,
    value: f64,
    muted: bool,
    on_change: impl FnMut(f64) + 'static,
    mut on_toggle_mute: impl FnMut(bool) + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let is_muted = muted;
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .maybe_child(icon_svg.map(|svg| icon(svg, 15., t.text_dim)))
        .child(
            rect()
                .width(Size::flex(1.))
                .content(Content::Flex)
                .opacity(if is_muted { 0.45 } else { 1.0 })
                .child(Slider::new(on_change).value(value).theme(SliderThemePartial::new().background(t.track).thumb_inner_background(t.accent).thumb_background(Color::from_rgb(255,255,255)).border_fill(t.accent))),
        )
        .child(MuteButton {
            muted: is_muted,
            on_toggle: EventHandler::new(move |v: bool| on_toggle_mute(v)),
        })
        .content(Content::Flex)
}

pub fn input_level_pill(level: f64) -> impl IntoElement {
    let t = use_app_theme();
    let pct = level.clamp(0.0, 100.0) as f32;
    rect()
        .width(Size::fill())
        .height(Size::px(8.))
        .corner_radius(999.)
        .background(t.track)
        .child(rect().width(Size::percent(pct)).height(Size::fill()).corner_radius(999.).background(t.accent))
}

pub fn input_level(level: f64) -> impl IntoElement { input_level_pill(level) }
