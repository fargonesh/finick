use freya::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg_base: Color,
    pub bg_surface: Color,
    pub bg_sidebar: Color,
    pub bg_card: Color,
    pub bg_card_hover: Color,
    pub bg_hover: Color,
    pub bg_active: Color,
    pub bg_selected: Color,

    pub primary_accent: Color,
    pub accent_blue: Color,
    pub accent_blue_hover: Color,
    pub accent_red: Color,
    pub accent_green: Color,
    pub accent_orange: Color,
    pub accent_purple: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_disabled: Color,

    pub border_subtle: Color,
    pub border_card: Color,
    pub border_focus: Color,
}

pub const DARK_THEME: Theme = Theme {
    mode: ThemeMode::Dark,
    bg_base: Color::from_rgb(18, 18, 18),
    bg_surface: Color::from_rgb(26, 26, 26),
    bg_sidebar: Color::from_rgb(22, 22, 22),
    bg_card: Color::from_rgb(32, 32, 32),
    bg_card_hover: Color::from_rgb(42, 42, 42),
    bg_hover: Color::from_rgb(44, 44, 44),
    bg_active: Color::from_rgb(56, 56, 56),
    bg_selected: Color::from_rgb(40, 40, 40),

    primary_accent: Color::from_rgb(15, 163, 242),
    accent_blue: Color::from_rgb(15, 163, 242),
    accent_blue_hover: Color::from_rgb(35, 175, 250),
    accent_red: Color::from_rgb(242, 50, 50),
    accent_green: Color::from_rgb(50, 242, 50),
    accent_orange: Color::from_rgb(242, 163, 15),
    accent_purple: Color::from_rgb(163, 50, 242),

    text_primary: Color::from_rgb(240, 240, 240),
    text_secondary: Color::from_rgb(150, 150, 150),
    text_muted: Color::from_rgb(180, 180, 180),
    text_disabled: Color::from_rgb(100, 100, 100),

    border_subtle: Color::from_rgb(40, 40, 40),
    border_card: Color::from_rgb(48, 48, 48),
    border_focus: Color::from_rgb(15, 163, 242),
};

pub const LIGHT_THEME: Theme = Theme {
    mode: ThemeMode::Light,
    bg_base: Color::from_rgb(245, 245, 245),
    bg_surface: Color::from_rgb(255, 255, 255),
    bg_sidebar: Color::from_rgb(235, 235, 235),
    bg_card: Color::from_rgb(255, 255, 255),
    bg_card_hover: Color::from_rgb(240, 240, 240),
    bg_hover: Color::from_rgb(230, 230, 230),
    bg_active: Color::from_rgb(215, 215, 215),
    bg_selected: Color::from_rgb(205, 205, 205),

    primary_accent: Color::from_rgb(10, 120, 200),
    accent_blue: Color::from_rgb(10, 120, 200),
    accent_blue_hover: Color::from_rgb(15, 140, 220),
    accent_red: Color::from_rgb(220, 40, 40),
    accent_green: Color::from_rgb(40, 200, 40),
    accent_orange: Color::from_rgb(220, 140, 10),
    accent_purple: Color::from_rgb(140, 40, 220),

    text_primary: Color::from_rgb(20, 20, 20),
    text_secondary: Color::from_rgb(90, 90, 90),
    text_muted: Color::from_rgb(130, 130, 130),
    text_disabled: Color::from_rgb(180, 180, 180),

    border_subtle: Color::from_rgb(210, 210, 210),
    border_card: Color::from_rgb(200, 200, 200),
    border_focus: Color::from_rgb(10, 120, 200),
};

pub fn use_init_app_theme(theme: Theme) -> State<Theme> {
    let st = use_state(|| theme);
    // In Freya provide_context is usually per-component tree,
    // but when called at root it propagates.
    provide_context(st.clone());
    st
}

pub fn use_app_theme() -> Theme {
    if let Some(ctx) = try_consume_context::<State<Theme>>() { *ctx.read() } else { DARK_THEME }
}

pub fn use_app_theme_state() -> State<Theme> {
    consume_context::<State<Theme>>()
}
