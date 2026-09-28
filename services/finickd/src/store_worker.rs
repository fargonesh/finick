use system::store::{self, StoreSource};

#[derive(Clone, Debug)]
pub enum StoreJob {
    InstallNix(String),
    RemoveNix(String),
    InstallFlatpak(String),
    RemoveFlatpak(String),
    UpdateFlatpaks,
    ApplyHomeManager,
}

pub fn describe(job: &StoreJob) -> String {
    match job {
        StoreJob::InstallNix(a) => format!("install nix {a}"),
        StoreJob::RemoveNix(a) => format!("remove nix {a}"),
        StoreJob::InstallFlatpak(id) => format!("install flatpak {id}"),
        StoreJob::RemoveFlatpak(id) => format!("remove flatpak {id}"),
        StoreJob::UpdateFlatpaks => "update flatpaks".to_string(),
        StoreJob::ApplyHomeManager => "apply home-manager".to_string(),
    }
}

pub fn run_job(job: StoreJob) -> Result<String, String> {
    match job {
        StoreJob::InstallNix(attr) => store::install_nix_reproducible(&attr),
        StoreJob::RemoveNix(attr) => store::remove_nix_reproducible(&attr),
        StoreJob::InstallFlatpak(id) => {
            let msg = store::flatpak::install(&id)?;
            let _ = store::hm::add_flatpak(&id);
            Ok(msg)
        }
        StoreJob::RemoveFlatpak(id) => {
            let msg = store::flatpak::remove(&id)?;
            let _ = store::hm::remove_flatpak(&id);
            Ok(msg)
        }
        StoreJob::UpdateFlatpaks => store::flatpak::update_all(),
        StoreJob::ApplyHomeManager => store::hm::apply_home_manager(),
    }
}

pub fn job_source(job: &StoreJob) -> Option<StoreSource> {
    match job {
        StoreJob::InstallNix(_) | StoreJob::RemoveNix(_) => Some(StoreSource::Nixpkgs),
        StoreJob::InstallFlatpak(_) | StoreJob::RemoveFlatpak(_) | StoreJob::UpdateFlatpaks => {
            Some(StoreSource::Flathub)
        }
        StoreJob::ApplyHomeManager => None,
    }
}
