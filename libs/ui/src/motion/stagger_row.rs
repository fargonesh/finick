use freya::animation::{AnimDirection, AnimNum, AnimatedValue, Ease, Function, OnCreation, ReadAnimatedValue, use_animation};
use freya::prelude::*;

use super::fade_slide::stagger_delay;
use super::tokens::{should_skip};

pub const STAGGER_ENTER_STEP_MS: u64 = 32;
pub const STAGGER_ENTER_BASE_MS: u64 = 30;
pub const STAGGER_ENTER_CAP_MS: u64 = 224;
pub const STAGGER_EXIT_STEP_MS: u64 = 20;
pub const STAGGER_ROW_CAP: usize = 8;
pub const STAGGER_ROW_ENTER_MS: u64 = 260;
pub const STAGGER_ROW_EXIT_MS: u64 = 160;
pub const STAGGER_ROW_OFFSET_Y: f32 = -5.0;
pub const STAGGER_ROW_BLUR: f32 = 4.0;

pub fn stagger_exit_total_ms(count: usize) -> u64 {
    if count == 0 {
        return 0;
    }
    let capped = count.min(STAGGER_ROW_CAP);
    ((capped.saturating_sub(1) as u64) * STAGGER_EXIT_STEP_MS) + STAGGER_ROW_EXIT_MS
}

#[derive(Clone)]
pub struct AnimStagger {
    enter_delay_ms: u64,
    exit_delay_ms: u64,
    enter: AnimNum,
    exit: AnimNum,
    value: f32,
}

impl Default for AnimStagger {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl AnimStagger {
    pub fn new(enter_delay_ms: u64, exit_delay_ms: u64) -> Self {
        Self {
            enter_delay_ms,
            exit_delay_ms,
            enter: AnimNum::new(0., 1.)
                .time(STAGGER_ROW_ENTER_MS.max(1))
                .ease(Ease::Out)
                .function(Function::Cubic),
            exit: AnimNum::new(0., 1.)
                .time(STAGGER_ROW_EXIT_MS.max(1))
                .ease(Ease::Out)
                .function(Function::Cubic),
            value: 0.,
        }
    }
}

impl AnimatedValue for AnimStagger {
    fn prepare(&mut self, direction: AnimDirection) {
        self.enter.prepare(direction);
        self.exit.prepare(direction);
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
        let total = match direction {
            AnimDirection::Forward => self.enter_delay_ms as u128 + STAGGER_ROW_ENTER_MS as u128,
            AnimDirection::Reverse => self.exit_delay_ms as u128 + STAGGER_ROW_EXIT_MS as u128,
        };
        index >= total && self.value == target
    }

    fn advance(&mut self, index: u128, direction: AnimDirection) {
        match direction {
            AnimDirection::Forward => {
                let i = index.saturating_sub(self.enter_delay_ms as u128).min(STAGGER_ROW_ENTER_MS as u128);
                self.enter.advance(i, direction);
                self.value = self.enter.value();
            }
            AnimDirection::Reverse => {
                let i = index.saturating_sub(self.exit_delay_ms as u128).min(STAGGER_ROW_EXIT_MS as u128);
                self.exit.advance(i, direction);
                self.value = self.exit.value();
            }
        }
    }

    fn finish(&mut self, direction: AnimDirection) {
        match direction {
            AnimDirection::Forward => {
                self.enter.finish(direction);
                self.value = self.enter.value();
            }
            AnimDirection::Reverse => {
                self.exit.finish(direction);
                self.value = self.exit.value();
            }
        }
    }

    fn into_reversed(self) -> Self {
        self
    }
}

impl ReadAnimatedValue for AnimStagger {
    type Output = f32;
    fn value(&self) -> Self::Output {
        self.value
    }
}

#[derive(Clone, PartialEq)]
pub struct StaggerRow {
    elements: Vec<Element>,
    key: DiffKey,
    index: usize,
    total: usize,
    dismiss: Readable<bool>,
    reduced: bool,
}

impl StaggerRow {
    pub fn new(index: usize, total: usize, dismiss: Readable<bool>) -> Self {
        Self { elements: Vec::new(), key: DiffKey::None, index, total, dismiss, reduced: false }
    }

    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }
}

impl ChildrenExt for StaggerRow {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }
}

impl KeyExt for StaggerRow {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for StaggerRow {
    fn render(&self) -> impl IntoElement {
        let skip = should_skip(self.reduced, STAGGER_ROW_ENTER_MS);
        let cap = self.total.min(STAGGER_ROW_CAP);
        let enter_delay = stagger_delay(self.index.min(STAGGER_ROW_CAP - 1), STAGGER_ENTER_STEP_MS, STAGGER_ENTER_CAP_MS)
            + STAGGER_ENTER_BASE_MS;
        let exit_pos = self.index.min(cap.saturating_sub(1));
        let exit_delay = ((cap.saturating_sub(1).saturating_sub(exit_pos)) as u64) * STAGGER_EXIT_STEP_MS;
        let dismiss = self.dismiss.clone();

        let mut anim = use_animation(move |conf| {
            if skip {
                conf.on_creation(OnCreation::Finish);
            } else {
                conf.on_creation(OnCreation::Run);
            }
            AnimStagger::new(enter_delay, exit_delay)
        });
        let mut reversed = use_state(|| false);

        use_side_effect(move || {
            if *dismiss.read() && !*reversed.read() {
                reversed.set(true);
                if !skip {
                    anim.reverse();
                }
            }
        });

        let t = if skip { 1. } else { anim.get().value() };
        rect()
            .opacity(t)
            .offset_y(STAGGER_ROW_OFFSET_Y * (1. - t))
            .blur(STAGGER_ROW_BLUR * (1. - t))
            .children(self.elements.clone())
    }
}
