use freya::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Accent {
    pub name: &'static str,
    pub hex: &'static str,
    pub color: Color,
}

pub const ACCENT_INDIGO: Accent = Accent {
    name: "Indigo",
    hex: "#5B5FE9",
    color: Color::from_rgb(0x5B, 0x5F, 0xE9),
};

pub const ACCENT_CORAL: Accent = Accent {
    name: "Coral",
    hex: "#FF6952",
    color: Color::from_rgb(0xFF, 0x69, 0x52),
};

pub const ACCENT_AMBER: Accent = Accent {
    name: "Amber",
    hex: "#E3A23D",
    color: Color::from_rgb(0xE3, 0xA2, 0x3D),
};

pub const ACCENT_TEAL: Accent = Accent {
    name: "Teal",
    hex: "#2CA6A0",
    color: Color::from_rgb(0x2C, 0xA6, 0xA0),
};

pub const ACCENT_ROSE: Accent = Accent {
    name: "Rose",
    hex: "#E85A88",
    color: Color::from_rgb(0xE8, 0x5A, 0x88),
};

pub const ACCENT_SLATE: Accent = Accent {
    name: "Slate",
    hex: "#7B7F87",
    color: Color::from_rgb(0x7B, 0x7F, 0x87),
};

pub const ACCENTS: [Accent; 6] = [
    ACCENT_INDIGO,
    ACCENT_CORAL,
    ACCENT_AMBER,
    ACCENT_TEAL,
    ACCENT_ROSE,
    ACCENT_SLATE,
];

// Design tokens matching ui_demo.html
pub const RADIUS_LG: f32 = 20.0;
pub const RADIUS_MD: f32 = 12.0;
pub const RADIUS_SM: f32 = 8.0;
pub const RADIUS_PILL: f32 = 999.0;
pub const GAP: f32 = 14.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AppTheme {
    pub mode: ThemeMode,

    // Core tokens from ui_demo.html
    pub bg: Color,
    pub panel: Color,
    pub panel_raised: Color,
    pub border: Color,
    pub text: Color,
    pub text_dim: Color,
    pub track: Color,
    pub sidebar_bg: Color,
    pub ring_cut: Color,
    pub accent: Color,
    pub accent_name: &'static str,

    // Compatibility aliases
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

pub type Theme = AppTheme;

impl AppTheme {
    /// Create a theme with a specific accent color applied
    pub fn with_accent(&self, accent: Accent) -> Self {
        let mut t = *self;
        t.accent = accent.color;
        t.accent_name = accent.name;
        t.primary_accent = accent.color;
        t.border_focus = accent.color;
        t
    }

    /// Create a theme with a specific mode (Light/Dark/Auto)
    pub fn with_mode(&self, mode: ThemeMode) -> Self {
        let base = match mode {
            ThemeMode::Light => LIGHT_THEME,
            ThemeMode::Dark | ThemeMode::Auto => DARK_THEME,
        };
        let mut t = base;
        t.mode = mode;
        t.accent = self.accent;
        t.accent_name = self.accent_name;
        t.primary_accent = self.primary_accent;
        t.border_focus = self.border_focus;
        t
    }
}

pub const DARK_THEME: AppTheme = AppTheme {
    mode: ThemeMode::Dark,

    // Exact values from html[data-theme="dark"]
    bg: Color::from_rgb(0x16, 0x17, 0x1A),
    panel: Color::from_rgb(0x1E, 0x1F, 0x23),
    panel_raised: Color::from_rgb(0x26, 0x27, 0x2C),
    border: Color::from_rgb(0x2B, 0x2C, 0x31),
    text: Color::from_rgb(0xF1, 0xF1, 0xEF),
    text_dim: Color::from_rgb(0x8B, 0x8C, 0x92),
    track: Color::from_rgb(0x30, 0x31, 0x36),
    sidebar_bg: Color::from_rgb(0x19, 0x19, 0x1D),
    ring_cut: Color::from_rgb(0x1E, 0x1F, 0x23),
    accent: Color::from_rgb(0x5B, 0x5F, 0xE9), // Indigo default
    accent_name: "Indigo",

    // Compatibility mappings
    bg_base: Color::from_rgb(0x16, 0x17, 0x1A),
    bg_surface: Color::from_rgb(0x1E, 0x1F, 0x23),
    bg_sidebar: Color::from_rgb(0x19, 0x19, 0x1D),
    bg_card: Color::from_rgb(0x1E, 0x1F, 0x23),
    bg_card_hover: Color::from_rgb(0x26, 0x27, 0x2C),
    bg_hover: Color::from_rgb(0x26, 0x27, 0x2C),
    bg_active: Color::from_rgb(0x2B, 0x2D, 0x3E), // subtle indigo-panel tint
    bg_selected: Color::from_rgb(0x2B, 0x2D, 0x3E),

    primary_accent: Color::from_rgb(0x5B, 0x5F, 0xE9),
    accent_blue: Color::from_rgb(0x5B, 0x5F, 0xE9),
    accent_blue_hover: Color::from_rgb(0x6E, 0x72, 0xF5),
    accent_red: Color::from_rgb(0xFF, 0x69, 0x52),
    accent_green: Color::from_rgb(0x2C, 0xA6, 0xA0),
    accent_orange: Color::from_rgb(0xE3, 0xA2, 0x3D),
    accent_purple: Color::from_rgb(0xE8, 0x5A, 0x88),

    text_primary: Color::from_rgb(0xF1, 0xF1, 0xEF),
    text_secondary: Color::from_rgb(0x8B, 0x8C, 0x92),
    text_muted: Color::from_rgb(0x6A, 0x6B, 0x72),
    text_disabled: Color::from_rgb(0x4A, 0x4B, 0x52),

    border_subtle: Color::from_rgb(0x2B, 0x2C, 0x31),
    border_card: Color::from_rgb(0x2B, 0x2C, 0x31),
    border_focus: Color::from_rgb(0x5B, 0x5F, 0xE9),
};

pub const LIGHT_THEME: AppTheme = AppTheme {
    mode: ThemeMode::Light,

    // Exact values from html[data-theme="light"]
    bg: Color::from_rgb(0xF1, 0xF1, 0xEE),
    panel: Color::from_rgb(0xFF, 0xFF, 0xFF),
    panel_raised: Color::from_rgb(0xFB, 0xFB, 0xFA),
    border: Color::from_rgb(0xE4, 0xE4, 0xDF),
    text: Color::from_rgb(0x1B, 0x1B, 0x1A),
    text_dim: Color::from_rgb(0x83, 0x83, 0x7D),
    track: Color::from_rgb(0xE8, 0xE8, 0xE3),
    sidebar_bg: Color::from_rgb(0xEA, 0xEA, 0xE6),
    ring_cut: Color::from_rgb(0xFF, 0xFF, 0xFF),
    accent: Color::from_rgb(0x5B, 0x5F, 0xE9), // Indigo default
    accent_name: "Indigo",

    // Compatibility mappings
    bg_base: Color::from_rgb(0xF1, 0xF1, 0xEE),
    bg_surface: Color::from_rgb(0xFF, 0xFF, 0xFF),
    bg_sidebar: Color::from_rgb(0xEA, 0xEA, 0xE6),
    bg_card: Color::from_rgb(0xFF, 0xFF, 0xFF),
    bg_card_hover: Color::from_rgb(0xFB, 0xFB, 0xFA),
    bg_hover: Color::from_rgb(0xF0, 0xF0, 0xEC),
    bg_active: Color::from_rgb(0xEA, 0xEB, 0xFD),
    bg_selected: Color::from_rgb(0xEA, 0xEB, 0xFD),

    primary_accent: Color::from_rgb(0x5B, 0x5F, 0xE9),
    accent_blue: Color::from_rgb(0x5B, 0x5F, 0xE9),
    accent_blue_hover: Color::from_rgb(0x4A, 0x4E, 0xD8),
    accent_red: Color::from_rgb(0xFF, 0x69, 0x52),
    accent_green: Color::from_rgb(0x2C, 0xA6, 0xA0),
    accent_orange: Color::from_rgb(0xE3, 0xA2, 0x3D),
    accent_purple: Color::from_rgb(0xE8, 0x5A, 0x88),

    text_primary: Color::from_rgb(0x1B, 0x1B, 0x1A),
    text_secondary: Color::from_rgb(0x83, 0x83, 0x7D),
    text_muted: Color::from_rgb(0xAA, 0xAA, 0xA4),
    text_disabled: Color::from_rgb(0xCC, 0xCC, 0xC6),

    border_subtle: Color::from_rgb(0xE4, 0xE4, 0xDF),
    border_card: Color::from_rgb(0xE4, 0xE4, 0xDF),
    border_focus: Color::from_rgb(0x5B, 0x5F, 0xE9),
};

pub fn use_init_app_theme(theme: AppTheme) -> State<AppTheme> {
    let st = use_state(|| theme);
    provide_context(st);
    st
}

pub fn use_app_theme() -> AppTheme {
    if let Some(ctx) = try_consume_context::<State<AppTheme>>() {
        *ctx.read()
    } else {
        DARK_THEME
    }
}

pub fn use_app_theme_state() -> State<AppTheme> {
    consume_context::<State<AppTheme>>()
}


