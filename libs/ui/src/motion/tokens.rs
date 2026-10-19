use freya::animation::{Ease, Function};

pub const DUR_FAST: u64 = 120;
pub const DUR_STANDARD: u64 = 170;
pub const DUR_EMPHASIS: u64 = 240;

pub const OFFSET_PANEL_Y: f32 = 8.0;
pub const OFFSET_PAGE_X: f32 = 24.0;
pub const SCALE_PANEL_FROM: f32 = 0.97;

pub const EASE_STANDARD: Ease = Ease::Out;
pub const FUNCTION_STANDARD: Function = Function::Expo;
pub const FUNCTION_PAGE: Function = Function::Cubic;

pub fn motion_duration(base: u64, reduced: bool) -> u64 {
    if reduced { 0 } else { base }
}

pub fn should_skip(reduced: bool, duration: u64) -> bool {
    reduced || duration == 0
}
