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

// --- Daemon jobs (progress IPC) -------------------------------------------
// Transport is the generic ipsea socket pair; types live here so daemon,
// store app and finickctl share one definition. Workers stream
// Started/Log/Finished for live notice lines.

/// Socket name for the store job server (served by finickd).
pub const STORE_SOCKET_NAME: &str = "finick-store";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StoreJob {
    InstallNix(String),
    RemoveNix(String),
    InstallFlatpak(String),
    RemoveFlatpak(String),
    UpdateFlatpak(String),
    UpdateFlatpaks,
    ApplyHomeManager,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StoreRequest {
    Run { job: StoreJob },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StoreResponse {
    Started { job: String },
    Log(String),
    Finished { result: Result<String, String> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum StoreProgress {
    Started(String),
    Log(String),
}

pub fn describe(job: &StoreJob) -> String {
    match job {
        StoreJob::InstallNix(a) => format!("install nix {a}"),
        StoreJob::RemoveNix(a) => format!("remove nix {a}"),
        StoreJob::InstallFlatpak(id) => format!("install flatpak {id}"),
        StoreJob::RemoveFlatpak(id) => format!("remove flatpak {id}"),
        StoreJob::UpdateFlatpak(id) => format!("update flatpak {id}"),
        StoreJob::UpdateFlatpaks => "update flatpaks".to_string(),
        StoreJob::ApplyHomeManager => "apply home-manager".to_string(),
    }
}

pub fn job_source(job: &StoreJob) -> Option<StoreSource> {
    match job {
        StoreJob::InstallNix(_) | StoreJob::RemoveNix(_) => Some(StoreSource::Nixpkgs),
        StoreJob::InstallFlatpak(_) | StoreJob::RemoveFlatpak(_) | StoreJob::UpdateFlatpak(_) | StoreJob::UpdateFlatpaks => {
            Some(StoreSource::Flathub)
        }
        StoreJob::ApplyHomeManager => None,
    }
}

/// Execute a job in-process, reporting progress. Used by the daemon worker
/// and as the local fallback when the daemon is unreachable.
pub fn run_job(job: StoreJob, progress: &mut dyn FnMut(StoreProgress)) -> Result<String, String> {
    let label = describe(&job);
    progress(StoreProgress::Started(label.clone()));
    progress(StoreProgress::Log(format!("starting {label}")));
    let result = match job {
        StoreJob::InstallNix(attr) => {
            progress(StoreProgress::Log(format!("declaring {attr} in apps.nix")));
            install_nix_reproducible(&attr)
        }
        StoreJob::RemoveNix(attr) => {
            progress(StoreProgress::Log(format!("removing {attr} from apps.nix")));
            remove_nix_reproducible(&attr)
        }
        StoreJob::InstallFlatpak(id) => {
            progress(StoreProgress::Log(format!("installing flatpak {id}")));
            match flatpak::install(&id) {
                Ok(msg) => {
                    let _ = hm::add_flatpak(&id);
                    progress(StoreProgress::Log(format!("declared {id} in apps.nix")));
                    Ok(msg)
                }
                Err(e) => Err(e),
            }
        }
        StoreJob::RemoveFlatpak(id) => {
            progress(StoreProgress::Log(format!("removing flatpak {id}")));
            match flatpak::remove(&id) {
                Ok(msg) => {
                    let _ = hm::remove_flatpak(&id);
                    progress(StoreProgress::Log(format!("pruned {id} from apps.nix")));
                    Ok(msg)
                }
                Err(e) => Err(e),
            }
        }
        StoreJob::UpdateFlatpak(id) => {
            progress(StoreProgress::Log(format!("updating flatpak {id}")));
            flatpak::update_single(&id)
        }
        StoreJob::UpdateFlatpaks => {
            progress(StoreProgress::Log("updating all flatpaks".to_string()));
            flatpak::update_all()
        }
        StoreJob::ApplyHomeManager => {
            progress(StoreProgress::Log("applying home-manager switch".to_string()));
            hm::apply_home_manager()
        }
    };
    match &result {
        Ok(msg) => {
            let short: String = msg.chars().take(180).collect();
            progress(StoreProgress::Log(format!("finished {label}: {short}")));
        }
        Err(e) => {
            let short: String = e.chars().take(180).collect();
            progress(StoreProgress::Log(format!("failed {label}: {short}")));
        }
    }
    result
}

/// Run a job against the daemon socket, falling back to in-process execution
/// when finickd is unreachable (daemon not running, e.g. dev sessions).
/// Progress callback fires for both paths; returns the final message.
pub fn run_job_ipc_or_local(job: StoreJob, progress: &mut dyn FnMut(StoreProgress)) -> Result<String, String> {
    run_job_on_socket(STORE_SOCKET_NAME, job, progress)
}

fn run_job_on_socket(
    socket: &str,
    job: StoreJob,
    progress: &mut dyn FnMut(StoreProgress),
) -> Result<String, String> {
    let req = StoreRequest::Run { job: job.clone() };
    let (tx, rx) = std::sync::mpsc::channel();
    let res = ipsea::send_command(socket, &req, Some(move |resp: StoreResponse| {
        let _ = tx.send(resp);
    }));
    if res.is_err() {
        return run_job(job, progress);
    }
    // Daemon owned the job; translate its stream. A dead stream mid-job
    // falls back to local execution rather than reporting success.
    let mut finished: Option<Result<String, String>> = None;
    for resp in rx {
        match resp {
            StoreResponse::Started { job } => progress(StoreProgress::Started(job)),
            StoreResponse::Log(line) => progress(StoreProgress::Log(line)),
            StoreResponse::Finished { result } => {
                finished = Some(result);
                break;
            }
        }
    }
    match finished {
        Some(r) => r,
        None => run_job(job, progress),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_describe_and_source() {
        assert_eq!(describe(&StoreJob::InstallNix("firefox".into())), "install nix firefox");
        assert_eq!(job_source(&StoreJob::UpdateFlatpak("org.x.Y".into())), Some(StoreSource::Flathub));
        assert_eq!(job_source(&StoreJob::ApplyHomeManager), None);
    }

    #[test]
    fn test_ipc_protocol_roundtrip() {
        let socket = format!("test-finick-store-{}", std::process::id());
        let server = socket.clone();
        std::thread::spawn(move || {
            // Stub worker: no real jobs run in tests, just the protocol.
            let _ = ipsea::start_server(server, |req: StoreRequest, sender: std::sync::mpsc::Sender<StoreResponse>| {
                let StoreRequest::Run { job } = req;
                let _ = sender.send(StoreResponse::Started { job: describe(&job) });
                let _ = sender.send(StoreResponse::Log("halfway".to_string()));
                let _ = sender.send(StoreResponse::Finished { result: Ok("done".to_string()) });
            });
        });
        std::thread::sleep(std::time::Duration::from_millis(150));
        let mut events = Vec::new();
        let res = run_job_on_socket(&socket, StoreJob::UpdateFlatpaks, &mut |ev| events.push(ev));
        assert_eq!(res, Ok("done".to_string()));
        assert_eq!(
            events,
            vec![
                StoreProgress::Started("update flatpaks".to_string()),
                StoreProgress::Log("halfway".to_string()),
            ]
        );
    }

    #[test]
    fn test_ipc_fallback_when_daemon_missing() {
        let mut events = Vec::new();
        let res = run_job_on_socket("test-finick-store-definitely-absent", StoreJob::RemoveFlatpak("no.such.App".into()), &mut |ev| {
            events.push(ev)
        });
        assert!(res.is_err());
        assert!(!events.is_empty());
        assert_eq!(events[0], StoreProgress::Started("remove flatpak no.such.App".to_string()));
        assert!(events.iter().any(|e| matches!(e, StoreProgress::Log(_))));
    }
}
