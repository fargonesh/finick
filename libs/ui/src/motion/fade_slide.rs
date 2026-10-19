use freya::animation::{
    AnimNum, AnimatedValue, AnimDirection, Ease, Function, OnCreation, ReadAnimatedValue,
    use_animation,
};
use freya::prelude::*;

use super::tokens::{DUR_STANDARD, motion_duration, should_skip};

#[derive(Clone, PartialEq)]
pub struct AnimHoldFade {
    delay_ms: u64,
    duration_ms: u64,
    fade: AnimNum,
    value: f32,
}

impl Default for AnimHoldFade {
    fn default() -> Self {
        Self::new(0, DUR_STANDARD)
    }
}

impl AnimHoldFade {
    pub fn new(delay_ms: u64, duration_ms: u64) -> Self {
        let duration_ms = duration_ms.max(1);
        Self {
            delay_ms,
            duration_ms,
            fade: AnimNum::new(0., 1.)
                .time(duration_ms)
                .ease(Ease::Out)
                .function(Function::Expo),
            value: 0.,
        }
    }

    fn total_ms(&self) -> u128 {
        self.delay_ms as u128 + self.duration_ms as u128
    }
}

impl AnimatedValue for AnimHoldFade {
    fn prepare(&mut self, direction: AnimDirection) {
        self.fade.prepare(direction);
        self.value = match direction {
            AnimDirection::Forward => 0.,
            AnimDirection::Reverse => 1.,
        };
    }

    fn is_finished(&self, index: u128, direction: AnimDirection) -> bool {
        let target = match direction {
            AnimDirection::Forward => 1.,
            AnimDirection::Reverse => 0.,
        };
        index >= self.total_ms() && self.value == target
    }

    fn advance(&mut self, index: u128, direction: AnimDirection) {
        let fade_index = index.saturating_sub(self.delay_ms as u128).min(self.duration_ms as u128);
        self.fade.advance(fade_index, direction);
        self.value = self.fade.value();
    }

    fn finish(&mut self, direction: AnimDirection) {
        self.fade.finish(direction);
        self.value = self.fade.value();
    }

    fn into_reversed(self) -> Self {
        Self {
            delay_ms: self.delay_ms,
            duration_ms: self.duration_ms,
            fade: self.fade.into_reversed(),
            value: self.value,
        }
    }
}

impl ReadAnimatedValue for AnimHoldFade {
    type Output = f32;
    fn value(&self) -> Self::Output {
        self.value
    }
}

#[derive(Clone, PartialEq)]
pub struct FadeSlideIn {
    elements: Vec<Element>,
    key: DiffKey,
    duration: u64,
    offset_y: f32,
    scale_from: Option<f32>,
    delay: u64,
    reduced: bool,
}

impl FadeSlideIn {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            key: DiffKey::None,
            duration: DUR_STANDARD,
            offset_y: -8.0,
            scale_from: None,
            delay: 0,
            reduced: false,
        }
    }

    pub fn duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn offset_y(mut self, offset_y: f32) -> Self {
        self.offset_y = offset_y;
        self
    }

    pub fn scale_from(mut self, scale: f32) -> Self {
        self.scale_from = Some(scale);
        self
    }

    pub fn delay(mut self, delay: u64) -> Self {
        self.delay = delay;
        self
    }

    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }
}

impl Default for FadeSlideIn {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildrenExt for FadeSlideIn {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }
}

impl KeyExt for FadeSlideIn {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for FadeSlideIn {
    fn render(&self) -> impl IntoElement {
        let duration = motion_duration(self.duration, self.reduced);
        let delay = if self.reduced { 0 } else { self.delay };
        let offset_y = self.offset_y;
        let scale_from = self.scale_from;
        let skip = should_skip(self.reduced, self.duration);

        let anim = use_animation(move |conf| {
            if skip {
                conf.on_creation(OnCreation::Finish);
            } else {
                conf.on_creation(OnCreation::Run);
            }
            AnimHoldFade::new(delay, duration)
        });

        let t = anim.get().value();
        let mut el = rect()
            .opacity(t)
            .offset_y(offset_y * (1. - t))
            .children(self.elements.clone());
        if let Some(s) = scale_from {
            el = el.scale(s + (1. - s) * t);
        }
        el
    }
}

#[derive(Clone, PartialEq)]
pub struct FadeIn {
    elements: Vec<Element>,
    key: DiffKey,
    duration: u64,
    delay: u64,
    reduced: bool,
}

impl FadeIn {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            key: DiffKey::None,
            duration: DUR_STANDARD,
            delay: 0,
            reduced: false,
        }
    }

    pub fn duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn delay(mut self, delay: u64) -> Self {
        self.delay = delay;
        self
    }

    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }
}

impl Default for FadeIn {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildrenExt for FadeIn {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }
}

impl KeyExt for FadeIn {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for FadeIn {
    fn render(&self) -> impl IntoElement {
        FadeSlideIn {
            elements: self.elements.clone(),
            key: self.key.clone(),
            duration: self.duration,
            offset_y: 0.0,
            scale_from: None,
            delay: self.delay,
            reduced: self.reduced,
        }
    }
}

pub fn stagger_delay(index: usize, step_ms: u64, max_ms: u64) -> u64 {
    ((index as u64) * step_ms).min(max_ms)
}

