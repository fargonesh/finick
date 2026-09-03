use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Request {
    Search { query: String },
    ListDir { path: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub is_desktop: bool,
    pub is_executable: bool,
    pub icon: Option<String>,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}
