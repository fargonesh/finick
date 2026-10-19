use freya::animation::{
    AnimNum, Ease, Function, OnChange, OnCreation, use_animation_with_dependencies,
};
use freya::prelude::*;
use crate::motion::tokens::DUR_FAST;
use crate::theme::{use_app_theme, RADIUS_PILL};

const THUMB_OFF: f32 = 2.5;
const THUMB_ON: f32 = 17.5;

#[derive(Clone, PartialEq)]
struct SwitchThumb {
    checked: bool,
}

impl Component for SwitchThumb {
    fn render(&self) -> impl IntoElement {
        let checked = self.checked;
        let anim = use_animation_with_dependencies(&checked, move |conf, c| {
            conf.on_creation(OnCreation::Finish);
            conf.on_change(OnChange::Rerun);
            if *c {
                AnimNum::new(THUMB_OFF, THUMB_ON)
                    .time(DUR_FAST)
                    .ease(Ease::Out)
                    .function(Function::Quad)
            } else {
                AnimNum::new(THUMB_ON, THUMB_OFF)
                    .time(DUR_FAST)
                    .ease(Ease::Out)
                    .function(Function::Quad)
            }
        });
        let left = anim.get().value();
        rect()
            .width(Size::px(16.))
            .height(Size::px(16.))
            .corner_radius(RADIUS_PILL)
            .background(Color::from_rgb(255, 255, 255))
            .margin((2.5, 0., 0., left))
    }
}

#[derive(Clone, PartialEq)]
struct PillSwitchInner {
    checked: bool,
    on_toggle: EventHandler<bool>,
}

impl Component for PillSwitchInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_toggle = self.on_toggle.clone();
        let checked = self.checked;

        let track_bg = if checked {
            if is_hovered { t.accent_blue_hover } else { t.accent }
        } else if is_hovered {
            t.border
        } else {
            t.track
        };

        rect()
            .width(Size::px(36.))
            .height(Size::px(21.))
            .corner_radius(RADIUS_PILL)
            .background(track_bg)
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 0. })
                    .fill(t.accent),
            )
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Switch)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_toggle.call(!checked))
            .child(SwitchThumb { checked })
    }
}

/// A custom switch toggle matching .switch in ui_demo.html
pub fn pill_switch(checked: bool, mut on_toggle: impl FnMut(bool) + 'static) -> impl IntoElement {
    PillSwitchInner {
        checked,
        on_toggle: EventHandler::new(move |v: bool| on_toggle(v)),
    }
}
