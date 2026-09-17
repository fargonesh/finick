use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InstalledApp {
    pub desktop_id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub comment: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub mime_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DefaultAppCategoryInfo {
    pub category_id: String,
    pub title: String,
    pub description: String,
    pub icon_name: String,
    pub current_desktop_id: String,
    pub current_name: String,
    pub available_apps: Vec<InstalledApp>,
}

fn app_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(format!("{home}/.local/share/applications")));
        dirs.push(PathBuf::from(format!("{home}/.nix-profile/share/applications")));
        dirs.push(PathBuf::from(format!("/etc/profiles/per-user/{}/share/applications", std::env::var("USER").unwrap_or_default())));
    }
    dirs.push(PathBuf::from("/run/current-system/sw/share/applications"));
    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs
}

fn parse_desktop_file(path: &Path) -> Option<InstalledApp> {
    let content = fs::read_to_string(path).ok()?;
    let desktop_id = path.file_name()?.to_string_lossy().to_string();

    let mut in_desktop_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut comment = None;
    let mut no_display = false;
    let mut is_type_application = false;
    let mut categories = Vec::new();
    let mut mime_types = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_desktop_entry = trimmed == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || trimmed.starts_with('#') {
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "Type" => is_type_application = val == "Application",
                "Name" if name.is_none() => name = Some(val.to_string()),
                "Exec" if exec.is_none() => exec = Some(val.to_string()),
                "Icon" if icon.is_none() => icon = Some(val.to_string()),
                "Comment" if comment.is_none() => comment = Some(val.to_string()),
                "NoDisplay" => no_display = val.eq_ignore_ascii_case("true"),
                "Categories" => {
                    categories = val
                        .split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "MimeType" => {
                    mime_types = val
                        .split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    if no_display || !is_type_application {
        return None;
    }

    let name = name.unwrap_or_else(|| desktop_id.trim_end_matches(".desktop").to_string());
    let exec = exec.unwrap_or_default();
    let icon = icon.unwrap_or_else(|| "application-x-executable".to_string());

    Some(InstalledApp {
        desktop_id,
        name,
        exec,
        icon,
        comment,
        categories,
        mime_types,
    })
}

pub fn scan_installed_apps() -> Vec<InstalledApp> {
    let mut apps_map = HashMap::new();

    for dir in app_search_dirs() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(app) = parse_desktop_file(&path) {
                        apps_map.entry(app.desktop_id.clone()).or_insert(app);
                    }
                }
            }
        }
    }

    let mut apps: Vec<InstalledApp> = apps_map.into_values().collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn get_mimeapps_list_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(format!("{home}/.config/mimeapps.list"))
}

pub fn ensure_writable_mimeapps_list() -> Result<PathBuf, String> {
    let path = get_mimeapps_list_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // If it is a symlink (e.g. into /nix/store), read target content and make it a real file
    if fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let _ = fs::remove_file(&path);
        let _ = fs::write(&path, existing);
    } else if !path.exists() {
        let _ = fs::write(&path, "[Default Applications]\n");
    }
    Ok(path)
}

fn read_default_from_mimeapps(mime: &str) -> Option<String> {
    let path = get_mimeapps_list_path();
    let content = fs::read_to_string(path).ok()?;
    let mut in_defaults = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_defaults = trimmed == "[Default Applications]";
            continue;
        }
        if in_defaults && !trimmed.starts_with('#') {
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim() == mime {
                    let first = v.split(';').next().unwrap_or(v).trim();
                    if !first.is_empty() {
                        return Some(first.to_string());
                    }
                }
            }
        }
    }
    None
}

fn query_xdg_mime(mime: &str) -> Option<String> {
    if let Some(desktop) = read_default_from_mimeapps(mime) {
        if !desktop.is_empty() {
            return Some(desktop);
        }
    }
    if let Ok(output) = Command::new("xdg-mime").args(["query", "default", mime]).output() {
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn query_xdg_browser() -> Option<String> {
    if let Ok(output) = Command::new("xdg-settings").args(["get", "default-web-browser"]).output() {
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !out.is_empty() {
            return Some(out);
        }
    }
    query_xdg_mime("x-scheme-handler/https")
        .or_else(|| query_xdg_mime("text/html"))
}

fn query_finick_defaults() -> HashMap<String, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = PathBuf::from(format!("{home}/.config/finick/defaults.json"));
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
            return map;
        }
    }
    HashMap::new()
}

fn save_finick_default(key: &str, val: &str) {
    let home = std::env::var("HOME").unwrap_or_default();
    let dir = PathBuf::from(format!("{home}/.config/finick"));
    let _ = fs::create_dir_all(&dir);
    let mut map = query_finick_defaults();
    map.insert(key.to_string(), val.to_string());
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = fs::write(dir.join("defaults.json"), json);
    }
}

pub fn get_default_apps() -> Vec<DefaultAppCategoryInfo> {
    let all_apps = scan_installed_apps();
    let finick_defaults = query_finick_defaults();

    let find_app_name = |desktop_id: &str| -> String {
        all_apps
            .iter()
            .find(|a| a.desktop_id == desktop_id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| {
                desktop_id
                    .trim_end_matches(".desktop")
                    .split('.')
                    .last()
                    .unwrap_or(desktop_id)
                    .to_string()
            })
    };

    let mut categories = Vec::new();

    // 1. Web Browser
    let browser_current = query_xdg_browser()
        .or_else(|| finick_defaults.get("browser").cloned())
        .unwrap_or_default();
    let mut browser_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("WebBrowser"))
                || a.mime_types.iter().any(|m| m == "text/html")
                || a.desktop_id.to_lowercase().contains("edge")
                || a.desktop_id.to_lowercase().contains("firefox")
                || a.desktop_id.to_lowercase().contains("chrome")
                || a.desktop_id.to_lowercase().contains("brave")
                || a.desktop_id.to_lowercase().contains("zen")
        })
        .cloned()
        .collect();
    browser_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "browser".to_string(),
        title: "Web Browser".to_string(),
        description: "Application for opening websites and web links".to_string(),
        icon_name: "GLOBE".to_string(),
        current_name: find_app_name(&browser_current),
        current_desktop_id: browser_current,
        available_apps: browser_apps,
    });

    // 2. Terminal Emulator
    let terminal_current = finick_defaults
        .get("terminal")
        .cloned()
        .unwrap_or_else(|| "ghostty.desktop".to_string());
    let mut term_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("TerminalEmulator"))
                || a.desktop_id.to_lowercase().contains("ghostty")
                || a.desktop_id.to_lowercase().contains("kitty")
                || a.desktop_id.to_lowercase().contains("alacritty")
                || a.desktop_id.to_lowercase().contains("foot")
                || a.desktop_id.to_lowercase().contains("xterm")
                || a.desktop_id.to_lowercase().contains("terminal")
        })
        .cloned()
        .collect();
    term_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "terminal".to_string(),
        title: "Terminal Emulator".to_string(),
        description: "Command line shell and terminal environment".to_string(),
        icon_name: "TERMINAL".to_string(),
        current_name: find_app_name(&terminal_current),
        current_desktop_id: terminal_current,
        available_apps: term_apps,
    });

    // 3. File Manager
    let files_current = query_xdg_mime("inode/directory")
        .or_else(|| finick_defaults.get("file_manager").cloned())
        .unwrap_or_default();
    let mut file_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("FileManager"))
                || a.mime_types.iter().any(|m| m == "inode/directory")
                || a.desktop_id.to_lowercase().contains("files")
                || a.desktop_id.to_lowercase().contains("dolphin")
                || a.desktop_id.to_lowercase().contains("nautilus")
                || a.desktop_id.to_lowercase().contains("thunar")
        })
        .cloned()
        .collect();
    file_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "file_manager".to_string(),
        title: "File Manager".to_string(),
        description: "Application for browsing directories and local files".to_string(),
        icon_name: "FOLDER".to_string(),
        current_name: find_app_name(&files_current),
        current_desktop_id: files_current,
        available_apps: file_apps,
    });

    // 4. Text / Code Editor
    let editor_current = query_xdg_mime("text/plain")
        .or_else(|| finick_defaults.get("editor").cloned())
        .unwrap_or_default();
    let mut editor_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("TextEditor") || c.eq_ignore_ascii_case("Development"))
                || a.mime_types.iter().any(|m| m == "text/plain")
                || a.desktop_id.to_lowercase().contains("code")
                || a.desktop_id.to_lowercase().contains("nvim")
                || a.desktop_id.to_lowercase().contains("vim")
                || a.desktop_id.to_lowercase().contains("gedit")
                || a.desktop_id.to_lowercase().contains("kate")
        })
        .cloned()
        .collect();
    editor_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "editor".to_string(),
        title: "Text & Code Editor".to_string(),
        description: "Application for viewing and modifying text files and scripts".to_string(),
        icon_name: "DOCUMENT".to_string(),
        current_name: find_app_name(&editor_current),
        current_desktop_id: editor_current,
        available_apps: editor_apps,
    });

    // 5. Email Client
    let mail_current = query_xdg_mime("x-scheme-handler/mailto")
        .or_else(|| finick_defaults.get("mail").cloned())
        .unwrap_or_default();
    let mut mail_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("Email"))
                || a.mime_types.iter().any(|m| m == "x-scheme-handler/mailto")
                || a.desktop_id.to_lowercase().contains("thunderbird")
                || a.desktop_id.to_lowercase().contains("mail")
        })
        .cloned()
        .collect();
    mail_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "mail".to_string(),
        title: "Email Client".to_string(),
        description: "Application for sending and reading emails and mailto links".to_string(),
        icon_name: "MAIL".to_string(),
        current_name: find_app_name(&mail_current),
        current_desktop_id: mail_current,
        available_apps: mail_apps,
    });

    // 6. Image Viewer
    let img_current = query_xdg_mime("image/png")
        .or_else(|| finick_defaults.get("image").cloned())
        .unwrap_or_default();
    let mut img_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("RasterGraphics") || c.eq_ignore_ascii_case("Viewer"))
                || a.mime_types.iter().any(|m| m.starts_with("image/"))
                || a.desktop_id.to_lowercase().contains("loupe")
                || a.desktop_id.to_lowercase().contains("gwenview")
                || a.desktop_id.to_lowercase().contains("eog")
        })
        .cloned()
        .collect();
    img_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "image".to_string(),
        title: "Image Viewer".to_string(),
        description: "Application for opening photos, screenshots, and artwork".to_string(),
        icon_name: "FILE_IMAGE".to_string(),
        current_name: find_app_name(&img_current),
        current_desktop_id: img_current,
        available_apps: img_apps,
    });

    // 7. Video Player
    let vid_current = query_xdg_mime("video/mp4")
        .or_else(|| finick_defaults.get("video").cloned())
        .unwrap_or_default();
    let mut vid_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("AudioVideo") || c.eq_ignore_ascii_case("Player"))
                || a.mime_types.iter().any(|m| m.starts_with("video/"))
                || a.desktop_id.to_lowercase().contains("showtime")
                || a.desktop_id.to_lowercase().contains("mpv")
                || a.desktop_id.to_lowercase().contains("vlc")
        })
        .cloned()
        .collect();
    vid_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "video".to_string(),
        title: "Video Player".to_string(),
        description: "Application for playing movies, video clips, and recordings".to_string(),
        icon_name: "VIDEO".to_string(),
        current_name: find_app_name(&vid_current),
        current_desktop_id: vid_current,
        available_apps: vid_apps,
    });

    // 8. Music Player
    let music_current = query_xdg_mime("audio/mpeg")
        .or_else(|| finick_defaults.get("music").cloned())
        .unwrap_or_default();
    let mut music_apps: Vec<InstalledApp> = all_apps
        .iter()
        .filter(|a| {
            a.categories.iter().any(|c| c.eq_ignore_ascii_case("Audio") || c.eq_ignore_ascii_case("Music"))
                || a.mime_types.iter().any(|m| m.starts_with("audio/"))
                || a.desktop_id.to_lowercase().contains("decibels")
                || a.desktop_id.to_lowercase().contains("amberol")
                || a.desktop_id.to_lowercase().contains("rhythmbox")
        })
        .cloned()
        .collect();
    music_apps.sort_by(|a, b| a.name.cmp(&b.name));
    categories.push(DefaultAppCategoryInfo {
        category_id: "music".to_string(),
        title: "Music Player".to_string(),
        description: "Application for playing audio tracks, podcasts, and albums".to_string(),
        icon_name: "MUSIC".to_string(),
        current_name: find_app_name(&music_current),
        current_desktop_id: music_current,
        available_apps: music_apps,
    });

    categories
}

fn write_to_mimeapps_list(desktop_id: &str, mimes: &[&str]) -> Result<(), String> {
    let path = ensure_writable_mimeapps_list()?;
    let content = fs::read_to_string(&path).unwrap_or_default();

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut default_section_idx = None;
    let mut next_section_idx = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "[Default Applications]" {
            default_section_idx = Some(i);
        } else if default_section_idx.is_some() && trimmed.starts_with('[') && trimmed.ends_with(']') {
            next_section_idx = Some(i);
            break;
        }
    }

    let insert_at = match default_section_idx {
        Some(idx) => idx + 1,
        None => {
            lines.push("[Default Applications]".to_string());
            lines.len()
        }
    };

    let search_end = next_section_idx.unwrap_or(lines.len());

    for mime in mimes {
        let prefix = format!("{mime}=");
        let mut found = false;
        for line in &mut lines[insert_at..search_end] {
            if line.trim().starts_with(&prefix) {
                *line = format!("{mime}={desktop_id}");
                found = true;
                break;
            }
        }
        if !found {
            lines.insert(insert_at, format!("{mime}={desktop_id}"));
        }
    }

    fs::write(&path, lines.join("\n") + "\n")
        .map_err(|e| format!("Failed to write mimeapps.list: {e}"))?;
    Ok(())
}

pub fn set_default_app(category_id: &str, desktop_id: &str) -> Result<(), String> {
    ensure_writable_mimeapps_list()?;
    save_finick_default(category_id, desktop_id);

    match category_id {
        "browser" => {
            // Apply with xdg-settings
            let _ = Command::new("xdg-settings")
                .args(["set", "default-web-browser", desktop_id])
                .output();
            let mimes = ["x-scheme-handler/http", "x-scheme-handler/https", "text/html", "application/xhtml+xml"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        "terminal" => {
            let exec_cmd = desktop_id
                .trim_end_matches(".desktop")
                .split('.')
                .last()
                .unwrap_or(desktop_id);
            let _ = Command::new("hyprctl")
                .args(["keyword", "$terminal", exec_cmd])
                .output();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.desktop.default-applications.terminal", "exec", exec_cmd])
                .output();
        }
        "file_manager" => {
            let mimes = ["inode/directory"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        "editor" => {
            let mimes = ["text/plain", "text/markdown"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        "mail" => {
            let mimes = ["x-scheme-handler/mailto"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        "image" => {
            let mimes = ["image/png", "image/jpeg", "image/webp", "image/gif", "image/svg+xml"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        "video" => {
            let mimes = ["video/mp4", "video/x-matroska", "video/webm", "video/quicktime"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        "music" => {
            let mimes = ["audio/mpeg", "audio/flac", "audio/ogg", "audio/wav", "audio/aac"];
            let _ = write_to_mimeapps_list(desktop_id, &mimes);
            for m in mimes {
                let _ = Command::new("xdg-mime").args(["default", desktop_id, m]).output();
            }
        }
        _ => return Err(format!("Unknown category: {category_id}")),
    }

    Ok(())
}
