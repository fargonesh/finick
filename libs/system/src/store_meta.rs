use {super::store::StoreEntry, std::process::Command};

pub const STORE_CATEGORIES: &[&str] =
    &["All", "Internet", "Media", "Games", "Office", "Graphics", "Development", "System", "Utilities"];

fn hay(entry: &StoreEntry) -> String {
    format!("{} {} {}", entry.name, entry.summary, entry.id).to_lowercase()
}

pub fn category_of(entry: &StoreEntry) -> &'static str {
    if matches!(entry.source, super::store::StoreSource::System) {
        return "System";
    }
    let h = hay(entry);
    let has = |words: &[&str]| words.iter().any(|w| h.contains(w));
    if has(&["game", "steam", "minecraft", "chess", "emulator", "lutris", "retro"]) {
        return "Games";
    }
    if has(&["gimp", "krita", "inkscape", "blender", "photo", "draw", "paint"]) {
        return "Graphics";
    }
    if has(&["browser", "firefox", "chrom", "mail", "thunderbird", "chat", "discord", "telegram", "matrix", "mastodon"]) {
        return "Internet";
    }
    if has(&["music", "video", "audio", "player", "vlc", "mpv"])
        && !has(&["editor", "code", "terminal"])
    {
        return "Media";
    }
    if has(&["image"]) && !has(&["editor", "code", "terminal"]) {
        return "Graphics";
    }
    if has(&["office", "document", "spreadsheet", "writer", "calc", "libreoffice", "pdf", "notes", "calendar", "task"]) {
        return "Office";
    }
    if has(&["code", "editor", "git", "terminal", "sdk", "compiler", "rust", "python", "ide", "neovim", "vscode"]) {
        return "Development";
    }
    if has(&["driver", "firmware", "kernel", "settings", "tweaks", "monitor", "disk", "backup"]) {
        return "System";
    }
    "Utilities"
}

pub fn appstream_available() -> bool {
    Command::new("appstreamcli").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn screenshot_urls(app_id: &str) -> Vec<String> {
    let id = app_id.trim();
    if id.is_empty() || !appstream_available() {
        return Vec::new();
    }
    let out = Command::new("appstreamcli").args(["dump", id]).output();
    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    let mut urls: Vec<String> = Vec::new();
    let mut in_screenshot = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with("<screenshot") {
            in_screenshot = true;
            continue;
        }
        if t.starts_with("</screenshot") {
            in_screenshot = false;
            continue;
        }
        if in_screenshot && t.starts_with("<image") {
            if let Some(start) = t.find('>') {
                if let Some(end) = t.rfind("</image>") {
                    let url = t[start + 1..end].trim().to_string();
                    if (url.starts_with("http://") || url.starts_with("https://")) && !urls.contains(&url) {
                        urls.push(url);
                    }
                    if urls.len() >= 6 {
                        break;
                    }
                }
            }
        }
    }
    urls
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FwDevice {
    pub name: String,
    pub guid: String,
    pub current: String,
    pub latest: String,
    pub has_update: bool,
}

pub fn fwupd_present() -> bool {
    Command::new("fwupdmgr").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn parse_fwupdmgr_json(raw: &str) -> Vec<FwDevice> {
    let mut out = Vec::new();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else { return out };
    let devices = v.get("Devices").and_then(|d| d.as_array()).cloned().unwrap_or_default();
    for d in devices {
        let name = d.get("Name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let guid = d
            .get("Guid")
            .and_then(|x| x.as_array())
            .and_then(|a| a.first())
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .or_else(|| d.get("DeviceId").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();
        if name.is_empty() || guid.is_empty() {
            continue;
        }
        let current = d.get("Version").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let latest = d
            .get("VersionLowest")
            .and_then(|x| x.as_str())
            .or_else(|| d.get("Releases").and_then(|r| r.as_array()).and_then(|a| a.first()).and_then(|r| r.get("Version").and_then(|x| x.as_str())))
            .unwrap_or("")
            .to_string();
        let has_update = !latest.is_empty() && !current.is_empty() && latest != current;
        out.push(FwDevice { name, guid, current, latest, has_update });
    }
    out
}

pub fn list_firmware() -> Vec<FwDevice> {
    if !fwupd_present() {
        return Vec::new();
    }
    if let Ok(out) = Command::new("fwupdmgr").args(["get-devices", "--json"]).output() {
        if out.status.success() {
            let devices = parse_fwupdmgr_json(&String::from_utf8_lossy(&out.stdout));
            if !devices.is_empty() {
                return devices;
            }
        }
    }
    Vec::new()
}

pub fn firmware_updates() -> Vec<FwDevice> {
    list_firmware().into_iter().filter(|d| d.has_update).collect()
}

pub fn update_firmware(guid: &str) -> Result<String, String> {
    if !fwupd_present() {
        return Err("fwupdmgr not installed".to_string());
    }
    let id = guid.trim();
    if id.is_empty() {
        return Err("empty device id".to_string());
    }
    let out = Command::new("fwupdmgr").args(["update", id, "--assume-yes"]).output().map_err(|e| format!("failed: {e}"))?;
    if out.status.success() {
        Ok(format!("updated {id}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, summary: &str) -> StoreEntry {
        StoreEntry {
            id: name.to_string(),
            name: name.to_string(),
            summary: summary.to_string(),
            source: crate::store::StoreSource::Flathub,
            ..Default::default()
        }
    }

    #[test]
    fn test_category_of_basic() {
        assert_eq!(category_of(&entry("Firefox", "Fast web browser")), "Internet");
        assert_eq!(category_of(&entry("Steam", "Game launcher")), "Games");
        assert_eq!(category_of(&entry("GIMP", "Image editor")), "Graphics");
        assert_eq!(category_of(&entry("LibreOffice Writer", "Office docs")), "Office");
        assert_eq!(category_of(&entry("Neovim", "Code editor")), "Development");
        assert_eq!(category_of(&entry("Oddball Tool", "does things")), "Utilities");
    }

    #[test]
    fn test_parse_fwupdmgr_json() {
        let raw = r#"{"Devices":[{"Name":"SSD","Guid":["abc"],"Version":"1.0","Releases":[{"Version":"1.2"}]}]}"#;
        let devs = parse_fwupdmgr_json(raw);
        assert_eq!(devs.len(), 1);
        assert!(devs[0].has_update);
        assert_eq!(devs[0].latest, "1.2");
    }
}
