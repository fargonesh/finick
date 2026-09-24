use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyboardShortcut {
    pub id: String,
    pub name: String,
    pub mods: Vec<String>,
    pub key: String,
    pub command: String,
    pub category: String,
    pub is_custom: bool,
}

fn get_finick_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|p| p.join("finick"))
}

fn get_shortcuts_file() -> Option<PathBuf> {
    get_finick_config_dir().map(|p| p.join("shortcuts.json"))
}

pub fn load_custom_shortcuts() -> Vec<KeyboardShortcut> {
    if let Some(path) = get_shortcuts_file() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<KeyboardShortcut>>(&content) {
                return list;
            }
        }
    }
    Vec::new()
}

pub fn save_custom_shortcuts(shortcuts: &[KeyboardShortcut]) -> Result<(), String> {
    let dir = get_finick_config_dir().ok_or_else(|| "Could not locate config directory".to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create finick config dir: {e}"))?;
    let path = dir.join("shortcuts.json");
    let json = serde_json::to_string_pretty(shortcuts)
        .map_err(|e| format!("Failed to serialize shortcuts: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write shortcuts file: {e}"))?;
    Ok(())
}

fn normalize_mods(mods_raw: &str) -> Vec<String> {
    let clean = mods_raw.replace('$', "");
    let mut parts: Vec<String> = clean
        .split(|c: char| c == '_' || c == ' ' || c == '+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| match s.to_uppercase().as_str() {
            "SUPER" | "MAINMOD" | "MOD4" => "Super".to_string(),
            "SHIFT" => "Shift".to_string(),
            "CTRL" | "CONTROL" => "Ctrl".to_string(),
            "ALT" | "MOD1" => "Alt".to_string(),
            other => other.to_string(),
        })
        .collect();

    // Standard ordering: Super, Ctrl, Alt, Shift
    parts.sort_by_key(|m| match m.as_str() {
        "Super" => 0,
        "Ctrl" => 1,
        "Alt" => 2,
        "Shift" => 3,
        _ => 4,
    });
    parts.dedup();
    parts
}

fn normalize_key(key_raw: &str) -> String {
    let key = key_raw.trim();
    match key {
        "Return" => "Enter".to_string(),
        "space" | "Space" => "Space".to_string(),
        "left" => "Left".to_string(),
        "right" => "Right".to_string(),
        "up" => "Up".to_string(),
        "down" => "Down".to_string(),
        "mouse:272" => "Left Click".to_string(),
        "mouse:273" => "Right Click".to_string(),
        "mouse_down" => "Scroll Down".to_string(),
        "mouse_up" => "Scroll Up".to_string(),
        "XF86AudioRaiseVolume" => "Volume Up".to_string(),
        "XF86AudioLowerVolume" => "Volume Down".to_string(),
        "XF86AudioMute" => "Audio Mute".to_string(),
        "XF86AudioMicMute" => "Mic Mute".to_string(),
        "XF86MonBrightnessUp" => "Brightness +".to_string(),
        "XF86MonBrightnessDown" => "Brightness -".to_string(),
        "XF86AudioNext" => "Next Track".to_string(),
        "XF86AudioPrev" => "Prev Track".to_string(),
        "XF86AudioPlay" | "XF86AudioPause" => "Play / Pause".to_string(),
        other => other.to_string(),
    }
}

fn categorize_shortcut(command: &str) -> (&'static str, String) {
    let cmd = command.trim();
    if cmd.contains("workspace") {
        let name = if let Some((_, ws)) = cmd.split_once("movetoworkspace,") {
            format!("Move Window to Workspace {}", ws.trim())
        } else if let Some((_, ws)) = cmd.split_once("workspace,") {
            format!("Switch to Workspace {}", ws.trim())
        } else {
            "Workspace Navigation".to_string()
        };
        ("Workspaces", name)
    } else if cmd.contains("killactive") {
        ("Window Management", "Close Active Window".to_string())
    } else if cmd.contains("movefocus") {
        let dir = cmd.split(',').nth(1).unwrap_or("").trim();
        let dir_str = match dir {
            "l" => "Left",
            "r" => "Right",
            "u" => "Up",
            "d" => "Down",
            _ => dir,
        };
        ("Window Management", format!("Focus Window {dir_str}"))
    } else if cmd.contains("movewindow") {
        ("Window Management", "Move Window".to_string())
    } else if cmd.contains("resizewindow") {
        ("Window Management", "Resize Window".to_string())
    } else if cmd.contains("togglefloating") {
        ("Window Management", "Toggle Floating Window".to_string())
    } else if cmd.contains("fullscreen") {
        ("Window Management", "Toggle Fullscreen".to_string())
    } else if cmd.contains("wpctl set-volume") {
        if cmd.contains('+') {
            ("Media & Audio", "Raise Volume".to_string())
        } else {
            ("Media & Audio", "Lower Volume".to_string())
        }
    } else if cmd.contains("wpctl set-mute") {
        if cmd.contains("SOURCE") {
            ("Media & Audio", "Mute / Unmute Microphone".to_string())
        } else {
            ("Media & Audio", "Mute / Unmute Audio".to_string())
        }
    } else if cmd.contains("brightnessctl") {
        if cmd.contains('+') {
            ("Media & Audio", "Increase Brightness".to_string())
        } else {
            ("Media & Audio", "Decrease Brightness".to_string())
        }
    } else if cmd.contains("playerctl") {
        if cmd.contains("next") {
            ("Media & Audio", "Next Track".to_string())
        } else if cmd.contains("prev") {
            ("Media & Audio", "Previous Track".to_string())
        } else {
            ("Media & Audio", "Play / Pause Media".to_string())
        }
    } else if cmd.contains("grimblast") {
        ("Applications & Tools", "Screenshot Selection".to_string())
    } else if cmd.contains("hyprlock") || cmd.contains("locker") {
        ("Applications & Tools", "Lock Screen".to_string())
    } else if cmd.contains("ghostty") || cmd.contains("terminal") || cmd.contains("kitty") {
        ("Applications & Tools", "Open Terminal".to_string())
    } else if cmd.contains("menu") || cmd.contains("vicinae") || cmd.contains("rofi") {
        ("Applications & Tools", "Application Launcher".to_string())
    } else if cmd.contains("dolphin") || cmd.contains("files") || cmd.contains("nautilus") {
        ("Applications & Tools", "Open File Manager".to_string())
    } else if cmd.starts_with("exec,") {
        let app = cmd.strip_prefix("exec,").unwrap_or(cmd).trim();
        ("Applications & Tools", format!("Launch {app}"))
    } else {
        ("General", format!("Action: {cmd}"))
    }
}

pub fn parse_hyprland_shortcuts() -> Vec<KeyboardShortcut> {
    let mut shortcuts = Vec::new();
    let mut seen_combos = HashSet::new();

    // 1. Custom shortcuts take top priority
    for custom in load_custom_shortcuts() {
        let combo = format!("{}:{}", custom.mods.join("+"), custom.key);
        seen_combos.insert(combo);
        shortcuts.push(custom);
    }

    // 2. Parse hyprland config file
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{home}/.config/hypr/hyprland.config"),
        format!("{home}/.config/hypr/hyprland.conf"),
    ];

    for path in &candidates {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with("bind") && (trimmed.contains('=') || trimmed.contains(',')) {
                    // e.g. "bind = $mainMod, Return, exec, ghostty"
                    // or "bindel = ,XF86AudioRaiseVolume, exec, wpctl..."
                    if let Some((_, rest)) = trimmed.split_once('=') {
                        let parts: Vec<&str> = rest.splitn(3, ',').map(|s| s.trim()).collect();
                        if parts.len() >= 2 {
                            let mods_str = parts[0];
                            let key_str = parts[1];
                            let command_str = if parts.len() > 2 { parts[2] } else { "" };

                            let mods = normalize_mods(mods_str);
                            let key = normalize_key(key_str);
                            let combo = format!("{}:{}", mods.join("+"), key);

                            if !seen_combos.contains(&combo) && !key.is_empty() {
                                seen_combos.insert(combo);
                                let (cat, name) = categorize_shortcut(command_str);
                                shortcuts.push(KeyboardShortcut {
                                    id: format!("hypr_{}_{}", mods.join("_"), key),
                                    name,
                                    mods,
                                    key,
                                    command: command_str.to_string(),
                                    category: cat.to_string(),
                                    is_custom: false,
                                });
                            }
                        }
                    }
                }
            }
            if !shortcuts.is_empty() {
                break;
            }
        }
    }

    // 3. If hyprland config was sparse, augment from `hyprctl binds -j`
    if shortcuts.len() < 5 {
        if let Ok(output) = Command::new("hyprctl").args(["binds", "-j"]).output() {
            if let Ok(json) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
                for bind in json {
                    let key = bind["key"].as_str().unwrap_or("").to_string();
                    let dispatcher = bind["dispatcher"].as_str().unwrap_or("").to_string();
                    let arg = bind["arg"].as_str().unwrap_or("").to_string();
                    let modmask = bind["modmask"].as_i64().unwrap_or(0);

                    let mut mods = Vec::new();
                    if modmask & 64 != 0 {
                        mods.push("Super".to_string());
                    }
                    if modmask & 1 != 0 {
                        mods.push("Shift".to_string());
                    }
                    if modmask & 4 != 0 {
                        mods.push("Ctrl".to_string());
                    }
                    if modmask & 8 != 0 {
                        mods.push("Alt".to_string());
                    }

                    let norm_key = normalize_key(&key);
                    let combo = format!("{}:{}", mods.join("+"), norm_key);
                    if !seen_combos.contains(&combo) && !norm_key.is_empty() && dispatcher != "__lua" {
                        seen_combos.insert(combo);
                        let cmd = if arg.is_empty() {
                            dispatcher.clone()
                        } else {
                            format!("{dispatcher}, {arg}")
                        };
                        let (cat, name) = categorize_shortcut(&cmd);
                        shortcuts.push(KeyboardShortcut {
                            id: format!("hyprctl_{}_{}", mods.join("_"), norm_key),
                            name,
                            mods,
                            key: norm_key,
                            command: cmd,
                            category: cat.to_string(),
                            is_custom: false,
                        });
                    }
                }
            }
        }
    }

    shortcuts
}

pub fn add_shortcut(shortcut: &KeyboardShortcut) -> Result<(), String> {
    let mut current = load_custom_shortcuts();
    current.retain(|s| s.id != shortcut.id);
    current.push(shortcut.clone());
    save_custom_shortcuts(&current)?;

    // Dynamically bind in Hyprland
    let hypr_mods = shortcut.mods.join(" ");
    let hypr_key = &shortcut.key;
    let hypr_cmd = if shortcut.command.starts_with("exec,") {
        shortcut.command.clone()
    } else {
        format!("exec, {}", shortcut.command)
    };

    let bind_arg = format!("{hypr_mods}, {hypr_key}, {hypr_cmd}");
    let _ = Command::new("hyprctl")
        .args(["keyword", "bind", &bind_arg])
        .output();

    Ok(())
}

pub fn remove_shortcut(id: &str) -> Result<(), String> {
    let mut current = load_custom_shortcuts();
    if let Some(target) = current.iter().find(|s| s.id == id).cloned() {
        current.retain(|s| s.id != id);
        save_custom_shortcuts(&current)?;

        // Unbind in Hyprland
        let hypr_mods = target.mods.join(" ");
        let hypr_key = &target.key;
        let _ = Command::new("hyprctl")
            .args(["keyword", "unbind", &format!("{hypr_mods}, {hypr_key}")])
            .output();
    }
    Ok(())
}
