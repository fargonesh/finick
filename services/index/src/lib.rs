use {
    image::ImageReader,
    ipsea::log::trace,
    notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher},
    r2d2::Pool,
    r2d2_sqlite::SqliteConnectionManager,
    regex::Regex,
    rusqlite::params,
    std::{
        env::{self, var},
        fs::{self, read_dir},
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        sync::{mpsc, Arc},
        time::{Duration, SystemTime},
    },
    ty::SearchResult,
};

struct ChannelData {
    name: String,
    path_str: String,
    parent_path: Option<String>,
    timestamp: i64,
    depth: usize,
    executable: bool,
    is_desktop: bool,
    icon: Option<String>,
    is_dir: bool,
    size: Option<u64>,
}

pub mod ty;
const MAX_DEPTH: usize = 5;
/// If a file was indexed less than this many seconds ago, skip re-indexing.
const REINDEX_THRESHOLD: i64 = 60;

fn folders() -> Vec<PathBuf> {
    let mut folders = vec![
        Path::new("/usr/share/applications/").to_path_buf(),
        Path::new("/usr/local/share/applications/").to_path_buf(),
        Path::new(&var("HOME").unwrap()).to_path_buf(),
    ];
    if let Ok(path) = var("PATH") {
        for folder in env::split_paths(&path) {
            folders.push(folder);
        }
    }
    folders
}

fn ignore() -> Vec<Regex> {
    vec![Regex::new("node_modules/.+").unwrap(), Regex::new("target/.+").unwrap(), Regex::new(r"/\..+").unwrap()]
}

fn is_binary_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        return matches!(ext.as_str(), "exe" | "bin" | "o" | "dll" | "so" | "dat" | "class" | "rmeta" | "rlib" | "d");
    }
    false
}

pub fn index(dirs: Option<Vec<PathBuf>>, pool: Pool<SqliteConnectionManager>) {
    let folders = dirs.unwrap_or(folders());
    let ignore = Arc::new(ignore());
    // Use a sync_channel with a fixed capacity to prevent unbounded memory usage.
    let (tx, rx) = mpsc::sync_channel::<ChannelData>(1000);

    // Spawn a thread to process file entries.
    std::thread::spawn(move || {
        let pool = pool.clone();
        while let Ok(data) = rx.recv() {
            let conn = pool.get().unwrap();
            println!("{:?}", data.icon);
            // Check if the file already exists and if it was indexed recently.
            let should_index =
                match conn.query_row("SELECT last_accessed FROM files WHERE path = ?1", params![&data.path_str], |row| {
                    row.get::<_, i64>(0)
                }) {
                    Ok(last_accessed) => {
                        // Only re-index if the stored timestamp is older than our threshold.
                        data.timestamp >= last_accessed + REINDEX_THRESHOLD
                    }
                    Err(_) => true, // File not indexed yet.
                };

            if should_index {
                // Using REPLACE here so that we update if it already exists.
                conn.execute(
                    "REPLACE INTO files (name, path, parent_path, depth, last_accessed, executable, desktop, icon, is_dir, \
                     size) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        data.name,
                        data.path_str,
                        data.parent_path,
                        data.depth,
                        data.timestamp,
                        data.executable,
                        data.is_desktop,
                        data.icon,
                        data.is_dir,
                        data.size.map(|s| s as i64),
                    ],
                )
                .unwrap();
            }
        }
    });

    folders.into_iter().for_each(|f| {
        std::thread::spawn({
            let ignore = ignore.clone();
            let tx = tx.clone();
            move || task(f, ignore.clone(), tx.clone(), 0)
        });
    });
}

fn task(dir: PathBuf, ignore: Arc<Vec<Regex>>, tx: mpsc::SyncSender<ChannelData>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }

    trace!("Indexing {}", dir.display());
    if let Ok(entries) = dir.read_dir() {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let path_str = path.to_string_lossy().to_string();
            let mut name = entry.file_name().to_string_lossy().to_string();
            let mut icon = Option::<String>::None;
            let mut is_desktop = false;
            let parent_path = path.parent().map(|p| p.to_string_lossy().to_string());

            let ft = entry.file_type().unwrap();
            if ft.is_dir() && !ignore.iter().any(|pat| pat.is_match(&path_str)) {
                let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;

                let _ = tx.send(ChannelData {
                    name: name.clone(),
                    path_str: path_str.clone(),
                    parent_path,
                    timestamp,
                    depth,
                    executable: false,
                    is_desktop: false,
                    icon: None,
                    is_dir: true,
                    size: None,
                });

                task(path, ignore.clone(), tx.clone(), depth + 1);
            } else if ft.is_file() && !is_binary_file(&path) {
                let timestamp = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or_else(|| SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64);
                let size = entry.metadata().ok().map(|m| m.len());
                let executable = is_executable(&path);

                if path.extension().map(|v| v.to_string_lossy().to_string()).unwrap_or_default() == "desktop" {
                    let (new_name, new_icon) = read_desktop(&path).unwrap_or_default();
                    name = if new_name.is_empty() { name } else { new_name };
                    icon = new_icon;
                    is_desktop = true;
                }

                let _ = tx.send(ChannelData {
                    name,
                    path_str,
                    parent_path,
                    timestamp,
                    depth,
                    executable,
                    is_desktop,
                    icon,
                    is_dir: false,
                    size,
                });
            } else if ft.is_symlink() {
                if let Ok(target) = fs::read_link(&path) {
                    if is_executable(&target) {
                        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
                        let size = fs::metadata(&path).ok().map(|m| m.len());
                        let executable = true;
                        // Send the symlink information. If the channel is full, this will block.
                        let _ = tx.send(ChannelData {
                            name,
                            path_str,
                            parent_path,
                            timestamp,
                            depth,
                            executable,
                            is_desktop,
                            icon,
                            is_dir: false,
                            size,
                        });
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn is_executable(path: &PathBuf) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}

fn read_desktop(path: &PathBuf) -> Result<(String, Option<String>), ()> {
    let desktop = fs::read_to_string(path).map_err(|_| ())?;

    let name = desktop
        .lines()
        .find(|l| l.starts_with("Name="))
        .map(|n| n.split('=').skip(1).collect::<Vec<_>>().join("="))
        .ok_or(())?;

    let icon = desktop.lines().find(|l| l.starts_with("Icon="));
    let icon = icon.map(|i| i.split('=').next_back().unwrap_or_default().to_string());

    if let Some(icon) = icon.and_then(|icon| resolve_icon(&icon)) {
        if let Ok(img) = ImageReader::open(Path::new(&icon)).unwrap().decode() {
            let img = img.resize(64, 64, image::imageops::FilterType::Lanczos3);

            let path = config::finick_root().join("icons").join(format!("{name}.png"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();

            img.save(&path).map_err(|_| ())?;
            return Ok((name, Some(path.to_string_lossy().to_string())));
        }
    }

    Ok((name, None))
}

fn resolve_icon(name: &str) -> Option<String> {
    let locations = vec![
        env::var("HOME").map(|v| format!("{v}/.icons/")).ok(),
        env::var("XDG_DATA_HOME").map(|v| format!("{v}/icons/")).ok(),
        Some("/usr/share/pixmaps".to_string()),
    ]
    .into_iter()
    .flatten();

    for location in locations {
        if let Ok(dir) = read_dir(location) {
            if let Some(icon) = dir.filter_map(|f| f.ok()).find(|f| f.file_name().to_string_lossy().starts_with(name)) {
                return Some(icon.path().to_string_lossy().to_string());
            }
        }
    }

    None
}

pub fn search(query: &str, pool: Pool<SqliteConnectionManager>, cb: impl Fn(SearchResult)) {
    let like_query = format!("%{query}%");
    let conn = pool.get().unwrap();

    // First, search in the database
    let mut res = conn
        .prepare(
            "SELECT name, path, icon, desktop, executable, is_dir, size, last_accessed
             FROM files
             WHERE name LIKE ?1 OR path LIKE ?1
             ORDER BY executable DESC, last_accessed DESC, depth ASC, LENGTH(replace(name, ?1, '')) ASC
             LIMIT 100",
        )
        .unwrap();

    let mut found_paths = std::collections::HashSet::new();

    let mut rows = res.query(params![like_query]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let name: String = row.get(0).unwrap();
        let path: String = row.get(1).unwrap();
        let icon: Option<String> = row.get(2).unwrap();
        let desktop: bool = row.get(3).unwrap();
        let executable: bool = row.get(4).unwrap();
        let is_dir: bool = row.get(5).unwrap_or(false);
        let size: Option<i64> = row.get(6).unwrap_or(None);
        let last_accessed: Option<i64> = row.get(7).unwrap_or(None);

        found_paths.insert(path.clone());
        cb(SearchResult {
            name,
            path,
            icon,
            is_desktop: desktop,
            is_executable: executable,
            is_dir,
            size: size.map(|s| s as u64),
            modified: last_accessed.map(|t| t as u64),
        });
    }

    // Quick 1-depth search in $PATH directories
    if let Ok(path_var) = var("PATH") {
        for folder in env::split_paths(&path_var) {
            if let Ok(entries) = folder.read_dir() {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    if path.is_file()
                        && is_executable(&path)
                        && path.file_name().map(|v| v.to_string_lossy()).unwrap_or_default().to_string().contains(query)
                    {
                        let path_str = path.to_string_lossy().to_string();
                        if !found_paths.contains(&path_str) {
                            let size = entry.metadata().ok().map(|m| m.len());
                            let modified = entry
                                .metadata()
                                .ok()
                                .and_then(|m| m.modified().ok())
                                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs());
                            cb(SearchResult {
                                name,
                                path: path.to_string_lossy().to_string(),
                                is_desktop: false,
                                is_executable: true,
                                icon: None,
                                is_dir: false,
                                size,
                                modified,
                            });
                        }
                    }
                }
            }
        }
    }
}

pub fn list_dir(path: &str, pool: Pool<SqliteConnectionManager>, cb: impl Fn(SearchResult)) {
    let clean_path = if path.len() > 1 && path.ends_with('/') { path.trim_end_matches('/') } else { path };
    let conn = pool.get().unwrap();
    let mut res = conn
        .prepare(
            "SELECT name, path, icon, desktop, executable, is_dir, size, last_accessed
             FROM files
             WHERE parent_path = ?1
             ORDER BY is_dir DESC, name ASC",
        )
        .unwrap();

    let mut rows = res.query(params![clean_path]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let name: String = row.get(0).unwrap();
        let path: String = row.get(1).unwrap();
        let icon: Option<String> = row.get(2).unwrap();
        let desktop: bool = row.get(3).unwrap();
        let executable: bool = row.get(4).unwrap();
        let is_dir: bool = row.get(5).unwrap_or(false);
        let size: Option<i64> = row.get(6).unwrap_or(None);
        let last_accessed: Option<i64> = row.get(7).unwrap_or(None);

        cb(SearchResult {
            name,
            path,
            icon,
            is_desktop: desktop,
            is_executable: executable,
            is_dir,
            size: size.map(|s| s as u64),
            modified: last_accessed.map(|t| t as u64),
        });
    }
}

pub fn watch(pool: Pool<SqliteConnectionManager>) {
    let (tx, rx) = mpsc::channel(); // std::sync::mpsc::channel

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default()).unwrap();

    for folder in folders() {
        let _ = watcher.watch(&folder, RecursiveMode::Recursive);
    }

    std::thread::spawn(move || loop {
        if let Ok(Ok(event)) = rx.recv() {
            match event.kind {
                EventKind::Create(_) => {
                    index(Some(event.paths), pool.clone());
                }
                EventKind::Remove(_) => {
                    event.paths.iter().for_each(|p| {
                        let param = format!("{}%", p.display());
                        pool.get().unwrap().execute("DELETE FROM files WHERE path LIKE ?1", params![param]).unwrap();
                    });
                }
                _ => {}
            }
        }
    });
}
