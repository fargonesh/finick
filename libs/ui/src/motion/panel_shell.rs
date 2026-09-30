use freya::animation::{OnCreation, ReadAnimatedValue, use_animation};
use freya::prelude::*;

use super::fade_slide::AnimHoldFade;
use super::tokens::{DUR_STANDARD, SCALE_PANEL_FROM, motion_duration, should_skip};

#[derive(Clone, PartialEq)]
pub struct PanelShell {
    elements: Vec<Element>,
    key: DiffKey,
    dismiss: Readable<bool>,
    on_dismiss: Option<EventHandler<()>>,
    duration: u64,
    offset_y: f32,
    scale_from: f32,
    reduced: bool,
}

impl PanelShell {
    pub fn new(dismiss: Readable<bool>) -> Self {
        Self {
            elements: Vec::new(),
            key: DiffKey::None,
            dismiss,
            on_dismiss: None,
            duration: DUR_STANDARD,
            offset_y: -8.0,
            scale_from: SCALE_PANEL_FROM,
            reduced: false,
        }
    }

    pub fn on_dismiss(mut self, handler: impl Into<EventHandler<()>>) -> Self {
        self.on_dismiss = Some(handler.into());
        self
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
        self.scale_from = scale;
        self
    }

    pub fn reduced(mut self, reduced: bool) -> Self {
        self.reduced = reduced;
        self
    }
}

impl ChildrenExt for PanelShell {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.elements
    }
}

impl KeyExt for PanelShell {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for PanelShell {
    fn render(&self) -> impl IntoElement {
        let duration = motion_duration(self.duration, self.reduced);
        let offset_y = self.offset_y;
        let scale_from = self.scale_from;
        let skip = should_skip(self.reduced, self.duration);
        let dismiss = self.dismiss.clone();
        let on_dismiss = self.on_dismiss.clone();

        let mut anim = use_animation(move |conf| {
            if skip {
                conf.on_creation(OnCreation::Finish);
            } else {
                conf.on_creation(OnCreation::Run);
            }
            AnimHoldFade::new(0, duration)
        });
        let mut fired = use_state(|| false);
        let dismiss_a = dismiss.clone();

        use_side_effect(move || {
            if *dismiss_a.read() && !*fired.read() {
                if skip {
                    fired.set(true);
                } else {
                    anim.reverse();
                }
            }
        });

        use_side_effect(move || {
            let done = !*anim.is_running().read() && *anim.has_run_yet().read();
            if *dismiss.read() && done && !*fired.read() {
                fired.set(true);
                if let Some(cb) = on_dismiss.clone() {
                    cb.call(());
                }
            }
        });

        let t = if skip { 1. } else { anim.get().value() };
        rect()
            .opacity(t)
            .offset_y(offset_y * (1. - t))
            .scale(scale_from + (1. - scale_from) * t)
            .children(self.elements.clone())
    }
}
