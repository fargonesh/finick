use {
    super::desktop_apps,
    serde::{Deserialize, Serialize},
};

pub use super::store_flatpak as flatpak;
pub use super::store_hm as hm;
pub use super::store_nix as nix;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StoreSource {
    Flathub,
    Nixpkgs,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StoreEntry {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub source: StoreSource,
    pub version: String,
    pub installed: bool,
    pub nix_locked: bool,
    pub impure_fallback: bool,
}

impl StoreEntry {
    pub fn nix(attr: &str, summary: &str, installed: bool) -> Self {
        let pname = attr.split('.').last().unwrap_or(attr).to_string();
        Self {
            id: format!("nixpkgs#{attr}"),
            name: pname,
            summary: summary.to_string(),
            source: StoreSource::Nixpkgs,
            installed,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct StoreDetails {
    pub entry: StoreEntry,
    pub nix_meta: Option<nix::NixPackage>,
    pub flatpak_metadata: String,
    pub flatpak_perms: Vec<flatpak::FlatpakPermission>,
}

pub fn details_for(entry: &StoreEntry) -> StoreDetails {
    match entry.source {
        StoreSource::Nixpkgs => {
            let attr = entry.id.trim_start_matches("nixpkgs#");
            StoreDetails { entry: entry.clone(), nix_meta: Some(nix::package_details(attr)), ..Default::default() }
        }
        StoreSource::Flathub => StoreDetails {
            entry: entry.clone(),
            flatpak_metadata: flatpak::metadata(&entry.id),
            flatpak_perms: flatpak::permissions(&entry.id),
            ..Default::default()
        },
        StoreSource::System => StoreDetails { entry: entry.clone(), ..Default::default() },
    }
}

impl Default for StoreEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            summary: String::new(),
            source: StoreSource::System,
            version: String::new(),
            installed: false,
            nix_locked: false,
            impure_fallback: false,
        }
    }
}

pub fn list_installed() -> Vec<StoreEntry> {
    let mut out: Vec<StoreEntry> = Vec::new();
    let decls = hm::load_decls();
    for app in desktop_apps::scan_installed_apps().into_iter().take(500) {
        out.push(StoreEntry {
            id: app.desktop_id.clone(),
            name: app.name,
            summary: app.comment.unwrap_or_default(),
            source: StoreSource::System,
            installed: true,
            ..Default::default()
        });
    }
    for fp in flatpak::list_installed() {
        if !out.iter().any(|e| e.id == fp.app_id) {
            out.push(StoreEntry {
                id: fp.app_id.clone(),
                name: if fp.name.is_empty() { fp.app_id.clone() } else { fp.name },
                summary: fp.description,
                source: StoreSource::Flathub,
                version: fp.version,
                installed: true,
                ..Default::default()
            });
        }
    }
    for attr in decls.nix_packages.iter().chain(nix::profile_list().iter()) {
        let clean = attr.trim().trim_start_matches("nixpkgs#").to_string();
        let id = format!("nixpkgs#{clean}");
        if !out.iter().any(|e| e.id == id) {
            out.push(StoreEntry {
                id: id.clone(),
                name: clean.split('.').last().unwrap_or(&clean).to_string(),
                summary: String::new(),
                source: StoreSource::Nixpkgs,
                installed: true,
                nix_locked: decls.nix_packages.iter().any(|x| x.trim() == clean),
                ..Default::default()
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn search_all(query: &str) -> Vec<StoreEntry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<StoreEntry> = Vec::new();
    for fp in flatpak::search(query) {
        out.push(StoreEntry {
            id: fp.app_id.clone(),
            name: fp.name,
            summary: fp.description,
            source: StoreSource::Flathub,
            version: fp.version,
            installed: fp.installed,
            ..Default::default()
        });
    }
    for pkg in nix::search(query) {
        out.push(StoreEntry {
            id: format!("nixpkgs#{}", pkg.attr),
            name: pkg.pname,
            summary: pkg.description,
            source: StoreSource::Nixpkgs,
            version: pkg.version,
            installed: pkg.installed,
            ..Default::default()
        });
    }
    for app in desktop_apps::scan_installed_apps() {
        if app.name.to_lowercase().contains(&q) || app.desktop_id.to_lowercase().contains(&q) {
            if !out.iter().any(|e| e.id == app.desktop_id) {
                out.push(StoreEntry {
                    id: app.desktop_id.clone(),
                    name: app.name,
                    summary: app.comment.unwrap_or_default(),
                    source: StoreSource::System,
                    installed: true,
                    ..Default::default()
                });
            }
        }
    }
    out.truncate(80);
    out
}

pub fn install_nix_reproducible(attr: &str) -> Result<String, String> {
    let clean = attr.trim().trim_start_matches("nixpkgs#");
    hm::add_nix_package(clean)?;
    match hm::apply_home_manager() {
        Ok(log) => Ok(format!("declared {clean} in apps.nix and applied. {log}")),
        Err(hm_err) => match nix::profile_install(clean) {
            Ok(msg) => Ok(format!("home-manager failed ({hm_err}); fallback: {msg}")),
            Err(fp_err) => Err(format!("home-manager failed: {hm_err}; fallback failed: {fp_err}")),
        },
    }
}

pub fn remove_nix_reproducible(attr: &str) -> Result<String, String> {
    let clean = attr.trim().trim_start_matches("nixpkgs#");
    hm::remove_nix_package(clean)?;
    let _ = nix::profile_remove(clean);
    match hm::apply_home_manager() {
        Ok(log) => Ok(format!("removed {clean} and applied. {log}")),
        Err(e) => Ok(format!("removed {clean} locally; home-manager re-apply needed: {e}")),
    }
}
