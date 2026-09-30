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

/// A custom switch toggle matching .switch in ui_demo.html
pub fn pill_switch(checked: bool, mut on_toggle: impl FnMut(bool) + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let track_bg = if checked { t.accent } else { t.track };

    rect()
        .width(Size::px(36.))
        .height(Size::px(21.))
        .corner_radius(RADIUS_PILL)
        .background(track_bg)
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_toggle(!checked))
        .child(SwitchThumb { checked })
}
