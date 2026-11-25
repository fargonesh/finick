use freya::animation::{
    AnimNum, Ease, Function, OnChange, OnCreation, use_animation_with_dependencies,
};
use freya::prelude::*;

use super::tokens::{DUR_FAST, motion_duration, should_skip};

#[derive(Clone, PartialEq)]
pub struct PressScale {
    elements: Vec<Element>,
    key: DiffKey,
    pressed_scale: f32,
    duration: u64,
    reduced: bool,
}

impl PressScale {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            key: DiffKey::None,
            pressed_scale: 0.97,
            duration: DUR_FAST,
            reduced: false,
        }
    }

    pub fn pressed_scale(mut self, scale: f32) -> Self {
        self.pressed_scale = scale;
        self
    }

    pub fn duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }
}

impl Default for PressScale {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildrenExt for PressScale {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }
}

impl KeyExt for PressScale {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for PressScale {
    fn render(&self) -> impl IntoElement {
        let pressed_scale = self.pressed_scale;
        let duration = motion_duration(self.duration, self.reduced);
        let skip = should_skip(self.reduced, self.duration);
        let mut pressed = use_state(|| false);

        let is_pressed = *pressed.read();
        let anim = use_animation_with_dependencies(&is_pressed, move |conf, p| {
            conf.on_creation(OnCreation::Finish);
            conf.on_change(OnChange::Rerun);
            if *p {
                AnimNum::new(1., pressed_scale)
                    .time(duration.max(1))
                    .ease(Ease::Out)
                    .function(Function::Quad)
            } else {
                AnimNum::new(pressed_scale, 1.)
                    .time(duration.max(1))
                    .ease(Ease::Out)
                    .function(Function::Quad)
            }
        });

        let s = if skip { 1. } else { anim.get().value() };
        rect()
            .scale(s)
            .on_pointer_down(move |_| pressed.set(true))
            .on_pointer_leave(move |_| pressed.set(false))
            .on_mouse_up(move |_| pressed.set(false))
            .children(self.elements.clone())
    }
}
