use freya::animation::{
    AnimNum, Ease, Function, OnChange, OnCreation, use_animation_with_dependencies,
};
use freya::prelude::*;

use super::tokens::{DUR_EMPHASIS, motion_duration, should_skip};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum PageDirection {
    #[default]
    None,
    Forward,
    Back,
}

impl PageDirection {
    fn offset_sign(self) -> f32 {
        match self {
            PageDirection::None => 0.,
            PageDirection::Forward => 1.,
            PageDirection::Back => -1.,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct AnimatedPage {
    elements: Vec<Element>,
    key: DiffKey,
    route_key: u64,
    direction: PageDirection,
    duration: u64,
    offset_x: f32,
    reduced: bool,
}

impl AnimatedPage {
    pub fn new(route_key: u64) -> Self {
        Self {
            elements: Vec::new(),
            key: DiffKey::None,
            route_key,
            direction: PageDirection::Forward,
            duration: DUR_EMPHASIS,
            offset_x: 24.0,
            reduced: false,
        }
    }

    pub fn direction(mut self, direction: PageDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn offset_x(mut self, offset_x: f32) -> Self {
        self.offset_x = offset_x;
        self
    }

    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }
}

impl ChildrenExt for AnimatedPage {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }
}

impl KeyExt for AnimatedPage {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for AnimatedPage {
    fn render(&self) -> impl IntoElement {
        let duration = motion_duration(self.duration, self.reduced);
        let skip = should_skip(self.reduced, self.duration);
        let shift = self.offset_x * self.direction.offset_sign();

        let anim = use_animation_with_dependencies(&self.route_key, move |conf, _| {
            conf.on_creation(OnCreation::Run);
            conf.on_change(OnChange::Rerun);
            AnimNum::new(0., 1.)
                .time(duration.max(1))
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let t = if skip { 1. } else { anim.get().value() };
        rect()
            .opacity(t)
            .offset_x(shift * (1. - t))
            .children(self.elements.clone())
    }
}

