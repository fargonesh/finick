use crate::theme::{DARK_THEME, LIGHT_THEME, Theme, ThemeMode};
use freya::prelude::Color;
use std::{env, fs, path::PathBuf};

fn theme_file_path() -> PathBuf {
    env::home_dir().unwrap_or_default().join(".finick_theme")
}

pub fn get_theme() -> Theme {
    let mut theme = LIGHT_THEME;
    if let Ok(s) = fs::read_to_string(theme_file_path()) {
        let parts: Vec<&str> = s.trim().split(';').collect();
        if !parts.is_empty() {
            if parts[0] == "light" {
                theme = LIGHT_THEME;
            } else if parts[0] == "auto" {
                theme = DARK_THEME;
                theme.mode = ThemeMode::Auto;
            }
        }
        if parts.len() > 1 {
            let colors: Vec<&str> = parts[1].split(',').collect();
            if colors.len() == 3
                && let (Ok(r), Ok(g), Ok(b)) = (colors[0].parse::<u8>(), colors[1].parse::<u8>(), colors[2].parse::<u8>()) {
                    let color = Color::from_rgb(r, g, b);
                    theme.primary_accent = color;
                    theme.accent = color;
                    theme.border_focus = color;
                }
        }
    }
    theme
}

pub fn set_theme(theme: &Theme) {
    let mode_str = match theme.mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
        ThemeMode::Auto => "auto",
    };
    let s = format!("{};{},{},{}", mode_str, theme.accent.r(), theme.accent.g(), theme.accent.b());
    let _ = fs::write(theme_file_path(), s);
}

