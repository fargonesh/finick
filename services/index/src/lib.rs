use {
    image::ImageReader,
    ipc::log::trace,
    notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher},
    r2d2::Pool,
    r2d2_sqlite::SqliteConnectionManager,
    regex::Regex,
    rusqlite::params,
    std::{
        collections::HashMap,
        env::{self, var},
        fs::{self, read_dir},
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        sync::{mpsc, Arc},
        time::SystemTime,
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
                    if desktop_entry_hidden(&path) {
                        continue;
                    }
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

pub fn desktop_entry_hidden(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    let mut in_entry = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_entry = t == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let val = v.trim();
            if (k.trim() == "NoDisplay" || k.trim() == "Hidden") && val.eq_ignore_ascii_case("true") {
                return true;
            }
        }
    }
    false
}

pub fn fuzzy_score(query: &str, name: &str, path: &str, is_desktop: bool, is_executable: bool) -> Option<i64> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    let n = name.to_lowercase();
    let p = path.to_lowercase();
    let mut score: i64;
    if n == q {
        score = 2000;
    } else if n.starts_with(&q) {
        score = 1200;
    } else if n.contains(&q) {
        score = 700;
    } else if p.split('/').next_back().unwrap_or("").starts_with(&q) {
        score = 600;
    } else {
        let qchars: Vec<char> = q.chars().collect();
        let subseq = |text: &str| -> Option<i64> {
            let tchars: Vec<char> = text.chars().collect();
            let mut ti = 0usize;
            let mut gap = 0i64;
            let mut first = -1i64;
            for qc in &qchars {
                let mut found = false;
                while ti < tchars.len() {
                    if tchars[ti] == *qc {
                        if first < 0 {
                            first = ti as i64;
                        }
                        found = true;
                        ti += 1;
                        break;
                    }
                    gap += 1;
                    ti += 1;
                }
                if !found {
                    return None;
                }
            }
            Some(200 - gap * 2 - first.max(0))
        };
        if let Some(s) = subseq(&n) {
            score = s.max(1);
        } else if let Some(s) = subseq(&p) {
            score = (s - 80).max(1);
        } else {
            return None;
        }
    }
    if is_desktop {
        score += 400;
    }
    if is_executable {
        score += 40;
    }
    score -= (name.len() as i64).min(60);
    Some(score)
}

pub fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Builds a subsequence LIKE pattern (`%c1%c2%...%`) so the SQL prefilter
/// does not drop subsequence-only matches before [`fuzzy_score`] runs.
/// Queries longer than 12 chars fall back to `None` to bound pattern cost.
pub fn fuzzy_like_pattern(query: &str) -> Option<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() || q.chars().count() > 12 {
        return None;
    }
    let mut pat = String::from("%");
    for c in q.chars() {
        if c == '%' || c == '_' || c == '\\' {
            pat.push('\\');
        }
        pat.push(c);
        pat.push('%');
    }
    Some(pat)
}

pub const USAGE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS usage (
        path TEXT PRIMARY KEY,
        count INTEGER NOT NULL DEFAULT 1,
        last_used INTEGER NOT NULL
    )";

pub fn ensure_usage_table(conn: &rusqlite::Connection) {
    let _ = conn.execute(USAGE_TABLE_SQL, []);
}

pub fn record_usage(pool: &Pool<SqliteConnectionManager>, path: &str) {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let Ok(conn) = pool.get() else { return };
    ensure_usage_table(&conn);
    let _ = conn.execute(
        "INSERT INTO usage (path, count, last_used) VALUES (?1, 1, ?2)
         ON CONFLICT(path) DO UPDATE SET count = count + 1, last_used = excluded.last_used",
        params![path, now],
    );
}

pub fn load_usage_counts(pool: &Pool<SqliteConnectionManager>) -> HashMap<String, u64> {
    let Ok(conn) = pool.get() else { return HashMap::new() };
    ensure_usage_table(&conn);
    let mut map = HashMap::new();
    let Ok(mut stmt) = conn.prepare("SELECT path, count FROM usage") else { return map };
    let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))) else { return map };
    for row in rows.filter_map(Result::ok) {
        map.insert(row.0, row.1.max(0) as u64);
    }
    map
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct RecentEntry {
    pub name: String,
    pub path: String,
    pub count: u64,
    pub icon: Option<String>,
    pub is_desktop: bool,
    pub is_executable: bool,
    pub is_dir: bool,
}

pub fn recents_path() -> PathBuf {
    if let Ok(home) = var("HOME") {
        let p = PathBuf::from(home).join(".cache").join("finick").join("launcher_recents.json");
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        return p;
    }
    config::finick_root().join("launcher_recents.json")
}

pub fn load_recents() -> Vec<RecentEntry> {
    let p = recents_path();
    let Ok(content) = fs::read_to_string(&p) else {
        return Vec::new();
    };
    let mut entries: Vec<RecentEntry> = serde_json::from_str(&content).unwrap_or_default();
    entries.sort_by(|a, b| b.count.cmp(&a.count));
    entries.truncate(20);
    entries
}

pub fn recent_counts() -> HashMap<String, u64> { load_recents().into_iter().map(|e| (e.path, e.count)).collect() }

pub fn bump_recent(r: &SearchResult) {
    let mut entries = load_recents();
    if let Some(e) = entries.iter_mut().find(|e| e.path == r.path) {
        e.count += 1;
        e.name = r.name.clone();
        e.icon = r.icon.clone();
    } else {
        entries.push(RecentEntry {
            name: r.name.clone(),
            path: r.path.clone(),
            count: 1,
            icon: r.icon.clone(),
            is_desktop: r.is_desktop,
            is_executable: r.is_executable,
            is_dir: r.is_dir,
        });
    }
    entries.sort_by(|a, b| b.count.cmp(&a.count));
    entries.truncate(20);
    let p = recents_path();
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let tmp = p.with_extension("tmp");
        if fs::write(&tmp, json).is_ok() {
            let _ = fs::rename(&tmp, &p);
        }
    }
}

fn read_desktop(path: &PathBuf) -> Result<(String, Option<String>), ()> {
    if desktop_entry_hidden(path) {
        return Err(());
    }
    let desktop = fs::read_to_string(path).map_err(|_| ())?;

    let name = desktop
        .lines()
        .find(|l| l.starts_with("Name="))
        .map(|n| n.split('=').skip(1).collect::<Vec<_>>().join("="))
        .ok_or(())?;

    let icon = desktop.lines().find(|l| l.starts_with("Icon="));
    let icon = icon.map(|i| i.split('=').next_back().unwrap_or_default().to_string());

    if let Some(icon) = icon.and_then(|icon| resolve_icon(&icon)) {
        let decoded = ImageReader::open(Path::new(&icon)).ok().and_then(|r| r.decode().ok());
        if let Some(img) = decoded {
            let img = img.resize(64, 64, image::imageops::FilterType::Lanczos3);

            let path = config::finick_root().join("icons").join(format!("{name}.png"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();

            img.save(&path).map_err(|_| ())?;
            return Ok((name, Some(path.to_string_lossy().to_string())));
        }
        return Ok((name, Some(icon)));
    }

    Ok((name, None))
}

fn resolve_icon(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if name.contains('/') {
        return Path::new(name).exists().then(|| name.to_string());
    }
    let stem = name.strip_suffix(".png").or_else(|| name.strip_suffix(".svg")).or_else(|| name.strip_suffix(".xpm")).unwrap_or(name);
    let extensions = ["png", "svg", "xpm", ""];
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(home) = var("HOME") {
        dirs.push(PathBuf::from(format!("{home}/.icons")));
        dirs.push(PathBuf::from(format!("{home}/.local/share/icons")));
    }
    if let Ok(xdg) = var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(format!("{xdg}/icons")));
    }
    for base in ["/usr/share/icons", "/usr/local/share/icons", "/usr/share/pixmaps", "/run/current-system/sw/share/icons"] {
        dirs.push(PathBuf::from(base));
    }
    for dir in &dirs {
        for ext in extensions {
            let candidate = if ext.is_empty() { dir.join(stem) } else { dir.join(format!("{stem}.{ext}")) };
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
        if let Ok(entries) = read_dir(dir) {
            for entry in entries.filter_map(|f| f.ok()) {
                let fname = entry.file_name().to_string_lossy().to_string();
                let fstem = fname.rsplit('.').nth(1).unwrap_or(&fname);
                if fstem.eq_ignore_ascii_case(stem) {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
        for theme in ["hicolor", "Adwaita", "gnome", "breeze", "Papirus"] {
            for size in ["48x48", "64x64", "scalable", "symbolic"] {
                for ext in ["png", "svg", "xpm"] {
                    let candidate = dir.join(theme).join(size).join("apps").join(format!("{stem}.{ext}"));
                    if candidate.is_file() {
                        return Some(candidate.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    None
}

pub fn search(query: &str, pool: Pool<SqliteConnectionManager>, cb: impl Fn(SearchResult)) {
    let like_query = format!("%{}%", escape_like(query));
    let fuzzy_pat = fuzzy_like_pattern(query);
    let conn = pool.get().unwrap();
    let mut counts = load_usage_counts(&pool);
    for (path, n) in recent_counts() {
        counts.entry(path).or_insert(n);
    }

    let mut res = conn
        .prepare(
            "SELECT name, path, icon, desktop, executable, is_dir, size, last_accessed
             FROM files
             WHERE name LIKE ?1 ESCAPE '\\' OR path LIKE ?1 ESCAPE '\\'
                OR (?2 IS NOT NULL AND (name LIKE ?2 ESCAPE '\\' OR path LIKE ?2 ESCAPE '\\'))
             ORDER BY last_accessed DESC, depth ASC
             LIMIT 300",
        )
        .unwrap();

    let mut found_paths = std::collections::HashSet::new();
    let mut scored: Vec<(i64, u64, SearchResult)> = Vec::new();

    let fuzzy_ref: Option<&str> = fuzzy_pat.as_deref();
    let mut rows = res.query(params![like_query, fuzzy_ref]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let name: String = row.get(0).unwrap();
        let path: String = row.get(1).unwrap();
        let icon: Option<String> = row.get(2).unwrap();
        let desktop: bool = row.get(3).unwrap();
        let executable: bool = row.get(4).unwrap();
        let is_dir: bool = row.get(5).unwrap_or(false);
        let size: Option<i64> = row.get(6).unwrap_or(None);
        let last_accessed: Option<i64> = row.get(7).unwrap_or(None);

        if desktop && path.ends_with(".desktop") && desktop_entry_hidden(Path::new(&path)) {
            continue;
        }
        let Some(base) = fuzzy_score(query, &name, &path, desktop, executable) else {
            continue;
        };
        let uses = counts.get(&path).copied().unwrap_or(0);
        let item = SearchResult {
            name,
            path: path.clone(),
            icon,
            is_desktop: desktop,
            is_executable: executable,
            is_dir,
            size: size.map(|s| s as u64),
            modified: last_accessed.map(|t| t as u64),
        };
        found_paths.insert(path);
        scored.push((base, uses, item));
    }

    if let Ok(path_var) = var("PATH") {
        for folder in env::split_paths(&path_var) {
            if let Ok(entries) = folder.read_dir() {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    if path.is_file() && is_executable(&path) {
                        let path_str = path.to_string_lossy().to_string();
                        if found_paths.contains(&path_str) {
                            continue;
                        }
                        let Some(base) = fuzzy_score(query, &name, &path_str, false, true) else {
                            continue;
                        };
                        let size = entry.metadata().ok().map(|m| m.len());
                        let modified = entry
                            .metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());
                        let uses = counts.get(&path_str).copied().unwrap_or(0);
                        found_paths.insert(path_str.clone());
                        scored.push((base, uses, SearchResult {
                            name,
                            path: path_str,
                            is_desktop: false,
                            is_executable: true,
                            icon: None,
                            is_dir: false,
                            size,
                            modified,
                        }));
                    }
                }
            }
        }
    }

    scored.sort_by(|a, b| {
        let rec_a = (a.1.min(50) as i64) * 25;
        let rec_b = (b.1.min(50) as i64) * 25;
        (b.0 + rec_b).cmp(&(a.0 + rec_a))
    });
    scored.truncate(50);
    for (_, _, item) in scored {
        cb(item);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_beats_subsequence() {
        let a = fuzzy_score("fire", "Firefox", "/usr/bin/firefox", true, false).unwrap();
        let b = fuzzy_score("fire", "Mozilla Firefox ESR", "/usr/bin/firefox-esr", false, false).unwrap();
        assert!(a > b);
    }

    #[test]
    fn desktop_beats_plain() {
        let a = fuzzy_score("term", "Terminal", "/usr/share/applications/t.desktop", true, false).unwrap();
        let b = fuzzy_score("term", "Terminal", "/usr/bin/terminal", false, true).unwrap();
        assert!(a > b);
    }

    #[test]
    fn no_match_returns_none() {
        assert!(fuzzy_score("zzzqqq", "Firefox", "/usr/bin/firefox", true, false).is_none());
    }

    #[test]
    fn empty_query_scores_zero() {
        assert_eq!(fuzzy_score("", "Anything", "/tmp/x", false, false), Some(0));
    }

    #[test]
    fn subsequence_matches_despite_gap() {
        assert!(fuzzy_score("fx", "Firefox", "/usr/bin/firefox", false, false).is_some());
    }

    #[test]
    fn exact_beats_prefix_beats_contains() {
        let exact = fuzzy_score("files", "Files", "/usr/bin/files", false, false).unwrap();
        let prefix = fuzzy_score("fil", "Files", "/usr/bin/files", false, false).unwrap();
        let contains = fuzzy_score("fil", "My Files App", "/usr/bin/myfiles", false, false).unwrap();
        assert!(exact > prefix && prefix > contains);
    }

    #[test]
    fn fuzzy_like_pattern_is_subsequence() {
        assert_eq!(fuzzy_like_pattern("fr"), Some("%f%r%".to_string()));
        assert_eq!(fuzzy_like_pattern(""), None);
        assert!(fuzzy_like_pattern("averylongquerystring").is_none());
    }

    #[test]
    fn fuzzy_like_pattern_escapes_wildcards() {
        assert_eq!(fuzzy_like_pattern("a%b"), Some("%a%\\%%b%".to_string()));
        assert_eq!(escape_like("100%_x\\y"), "100\\%\\_x\\\\y");
    }
}
