use {
    serde::{Deserialize, Serialize},
    std::process::Command,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct FlatpakApp {
    pub app_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub branch: String,
    pub origin: String,
    pub installed: bool,
}

pub fn flatpak_available() -> bool {
    Command::new("flatpak").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn run_flatpak(args: &[&str]) -> Option<String> {
    Command::new("flatpak")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

pub fn ensure_flathub_remote() -> Result<(), String> {
    if !flatpak_available() {
        return Err("flatpak not installed".to_string());
    }
    if let Some(out) = run_flatpak(&["remotes", "--columns=name"]) {
        if out.lines().any(|l| l.trim() == "flathub") {
            return Ok(());
        }
    }
    Command::new("flatpak")
        .args(["remote-add", "--user", "--if-not-exists", "flathub", "https://flathub.org/repo/flathub.flatpakrepo"])
        .output()
        .map_err(|e| format!("failed to add flathub: {e}"))?;
    Ok(())
}

pub fn list_installed() -> Vec<FlatpakApp> {
    let Some(out) = run_flatpak(&["list", "--app", "--columns=application,name,description,version,branch,origin"]) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            Some(FlatpakApp {
                app_id: parts.first().unwrap_or(&"").trim().to_string(),
                name: parts.get(1).unwrap_or(&"").trim().to_string(),
                description: parts.get(2).unwrap_or(&"").trim().to_string(),
                version: parts.get(3).unwrap_or(&"").trim().to_string(),
                branch: parts.get(4).unwrap_or(&"stable").trim().to_string(),
                origin: parts.get(5).unwrap_or(&"").trim().to_string(),
                installed: true,
            })
        })
        .filter(|a| !a.app_id.is_empty())
        .collect()
}

pub fn search(query: &str) -> Vec<FlatpakApp> {
    let q = query.trim();
    if q.is_empty() || !flatpak_available() {
        return Vec::new();
    }
    let installed_ids: std::collections::HashSet<String> =
        list_installed().into_iter().map(|a| a.app_id).collect();
    let Some(out) = run_flatpak(&["search", "--columns=application,name,description,version,branch,remotes", q]) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("Name\t") || line.starts_with("Application") {
                return None;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 2 {
                return None;
            }
            let (app_id, name, description, version, branch, origin) = if parts.len() >= 6 {
                (parts[2].trim(), parts[0].trim(), parts[1].trim(), parts[3].trim(), parts[4].trim(), parts[5].trim())
            } else {
                (parts[0].trim(), parts[0].trim(), parts.get(1).unwrap_or(&"").trim(), "", "stable", "flathub")
            };
            if app_id.is_empty() {
                return None;
            }
            Some(FlatpakApp {
                app_id: app_id.to_string(),
                name: if name.is_empty() { app_id.to_string() } else { name.to_string() },
                description: description.to_string(),
                version: version.to_string(),
                branch: branch.to_string(),
                origin: origin.to_string(),
                installed: installed_ids.contains(app_id),
            })
        })
        .take(50)
        .collect()
}

pub fn install(app_id: &str) -> Result<String, String> {
    let id = app_id.trim();
    if id.is_empty() {
        return Err("empty app id".to_string());
    }
    let _ = ensure_flathub_remote();
    let out = Command::new("flatpak")
        .args(["install", "--user", "-y", "flathub", id])
        .output()
        .map_err(|e| format!("failed to run flatpak: {e}"))?;
    if out.status.success() {
        Ok(format!("installed {id}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn remove(app_id: &str) -> Result<String, String> {
    let id = app_id.trim();
    let out = Command::new("flatpak")
        .args(["uninstall", "--user", "-y", id])
        .output()
        .map_err(|e| format!("failed to run flatpak: {e}"))?;
    if out.status.success() {
        Ok(format!("removed {id}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn update_single(app_id: &str) -> Result<String, String> {
    let id = app_id.trim();
    let out = Command::new("flatpak").args(["update", "--user", "-y", id]).output().map_err(|e| format!("failed: {e}"))?;
    if out.status.success() {
        Ok(format!("updated {id}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn update_all() -> Result<String, String> {
    let out = Command::new("flatpak").args(["update", "--user", "-y"]).output().map_err(|e| format!("failed: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlatpakPermission {
    pub key: String,
    pub label: String,
    pub allowed: bool,
}

pub fn metadata(app_id: &str) -> String {
    run_flatpak(&["info", "--show-metadata", app_id.trim()]).unwrap_or_default()
}

pub fn permissions(app_id: &str) -> Vec<FlatpakPermission> {
    let meta = metadata(app_id);
    let mut in_context = false;
    let mut map: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for line in meta.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_context = t == "[Context]";
            continue;
        }
        if !in_context || t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            map.insert(k.trim().to_string(), !v.trim().is_empty());
        }
    }
    let labels = [
        ("shared", "IPC / network share"),
        ("sockets", "Sockets (X11, Wayland, Pulse)"),
        ("devices", "Device access"),
        ("filesystems", "Filesystem access"),
        ("network", "Network"),
        ("bluetooth", "Bluetooth"),
    ];
    let mut out = Vec::new();
    for (key, label) in labels {
        if let Some(allowed) = map.get(key) {
            out.push(FlatpakPermission { key: key.to_string(), label: label.to_string(), allowed: *allowed });
        }
    }
    for (k, allowed) in map {
        if !labels.iter().any(|(key, _)| *key == k) {
            out.push(FlatpakPermission { key: k.clone(), label: k, allowed });
        }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

pub fn set_permission(app_id: &str, key: &str, allow: bool) -> Result<String, String> {
    let id = app_id.trim();
    let flag = if allow { format!("--allow={key}") } else { format!("--disallow={key}") };
    let out = Command::new("flatpak")
        .args(["override", "--user", &flag, id])
        .output()
        .map_err(|e| format!("override failed: {e}"))?;
    if out.status.success() {
        Ok(format!("{} {key}", if allow { "allowed" } else { "revoked" }))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn pending_updates() -> Vec<FlatpakApp> {
    let installed = list_installed();
    let Some(out) = run_flatpak(&["remote-ls", "--user", "--updates", "--columns=application,name,description,version,branch,origin"]) else {
        return Vec::new();
    };
    let update_ids: std::collections::HashSet<String> = out
        .lines()
        .filter_map(|l| {
            let id = l.split('\t').next().unwrap_or("").trim();
            if id.is_empty() { None } else { Some(id.to_string()) }
        })
        .collect();
    installed.into_iter().filter(|a| update_ids.contains(&a.app_id)).collect()
}
