use crate::theme::{DARK_THEME, LIGHT_THEME, Theme, ThemeMode};
use freya::prelude::Color;
use std::{env, fs, path::PathBuf};

fn theme_file_path() -> PathBuf {
    env::home_dir().unwrap_or_default().join(".finick_theme")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ThemeFile { mode: String, accent: [u8; 3] }

pub fn get_theme() -> Theme {
    if let Ok(s) = fs::read_to_string(theme_file_path()) {
        // Try json first, fallback to legacy "mode;R,G,B"
        if let Ok(tf) = serde_json::from_str::<ThemeFile>(&s) {
            let mut theme = match tf.mode.as_str() {
                "light" => LIGHT_THEME,
                "auto" => { let mut t = DARK_THEME; t.mode = ThemeMode::Auto; t },
                _ => DARK_THEME,
            };
            let c = Color::from_rgb(tf.accent[0], tf.accent[1], tf.accent[2]);
            theme.accent = c; theme.primary_accent = c; theme.border_focus = c;
            return theme;
        }
        // legacy
        let parts: Vec<&str> = s.trim().split(';').collect();
        let mut theme = LIGHT_THEME;
        if parts.first().is_some_and(|p| *p == "auto") { theme = DARK_THEME; theme.mode = ThemeMode::Auto; }
        else if parts.first().is_some_and(|p| *p != "light") { theme = DARK_THEME; }
        if parts.len() > 1 {
            let c: Vec<&str> = parts[1].split(',').collect();
            if c.len()==3 && let (Ok(r),Ok(g),Ok(b))=(c[0].parse::<u8>(),c[1].parse::<u8>(),c[2].parse::<u8>()) {
                let col=Color::from_rgb(r,g,b); theme.accent=col; theme.primary_accent=col; theme.border_focus=col;
            }
        }
        return theme;
    }
    LIGHT_THEME
}

pub fn set_theme(theme: &Theme) {
    let mode = match theme.mode { ThemeMode::Light=>"light", ThemeMode::Dark=>"dark", ThemeMode::Auto=>"auto" }.to_string();
    let tf = ThemeFile { mode, accent: [theme.accent.r(), theme.accent.g(), theme.accent.b()] };
    let _ = fs::write(theme_file_path(), serde_json::to_string(&tf).unwrap_or_default());
}

