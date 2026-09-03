use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(strum::Display, strum::EnumString, Serialize, Deserialize, Debug, Clone)]
pub enum App {
    Scan,
    Files,
    IndexService,
    Other(String),
}

impl From<App> for PathBuf {
    fn from(app: App) -> Self {
        PathBuf::from(app.to_string())
    }
}

