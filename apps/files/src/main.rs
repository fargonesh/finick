#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use {
    config::ty::App,
    freya::prelude::*,
    std::{
        env,
        path::{Path, PathBuf},
    },
    ui::*,
};

#[derive(Clone, PartialEq, Debug)]
enum ItemType {
    File,
    Folder,
}

#[derive(Clone, PartialEq, Debug)]
struct Item {
    ty: ItemType,
    name: String,
    path: String,
    size: u64,
    modified: Option<u64>,
}

#[derive(Clone, PartialEq, Debug)]
struct PendingPaste {
    srcs: Vec<String>,
    is_cut: bool,
    dest_dir: String,
}

static LOAD_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LOAD_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LOAD_ERR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static LAST_TRASHED: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

#[derive(Clone, PartialEq, Debug)]
struct JobProgress {
    label: String,
    done: usize,
    total: usize,
    current: String,
}

#[derive(Clone, PartialEq)]
enum ViewMode {
    List,
    Grid,
}

#[derive(Clone, Copy, PartialEq)]
enum IconSize {
    Small,
    Medium,
    Large,
}

impl IconSize {
    fn px(self) -> f32 {
        match self {
            Self::Small => 24.,
            Self::Medium => 32.,
            Self::Large => 48.,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Small => "S",
            Self::Medium => "M",
            Self::Large => "L",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Small => Self::Medium,
            Self::Medium => Self::Large,
            Self::Large => Self::Small,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Name,
    Size,
    Modified,
}

fn crumbs_for(path: &str) -> Vec<(String, String)> {
    let p = PathBuf::from(path);
    let mut out = Vec::new();
    let mut cur = PathBuf::new();
    let mut any = false;
    for comp in p.components() {
        use std::path::Component as C;
        match comp {
            C::RootDir => {
                cur.push("/");
                out.push(("/".to_string(), "/".to_string()));
                any = true;
            }
            C::Prefix(_) | C::CurDir | C::ParentDir => {}
            C::Normal(s) => {
                cur.push(s);
                any = true;
                out.push((s.to_string_lossy().to_string(), cur.to_string_lossy().to_string()));
            }
        }
    }
    if !any {
        out.push(("/".to_string(), "/".to_string()));
    }
    out
}

fn free_space_for(path: &str) -> Option<String> {
    let out = std::process::Command::new("df").args(["-h", path]).output().ok()?;
    let txt = String::from_utf8_lossy(&out.stdout);
    let last = txt.lines().last()?;
    let parts: Vec<&str> = last.split_whitespace().collect();
    if parts.len() >= 4 { Some(parts[3].to_string()) } else { None }
}

fn visible_items(all: &[Item], sort: SortKey, dirs_first: bool, show_hidden: bool) -> Vec<Item> {
    let mut v: Vec<Item> = all.iter().filter(|i| show_hidden || !i.name.starts_with('.')).cloned().collect();
    v.sort_by(|a, b| {
        if dirs_first {
            match (&a.ty, &b.ty) {
                (ItemType::Folder, ItemType::File) => return std::cmp::Ordering::Less,
                (ItemType::File, ItemType::Folder) => return std::cmp::Ordering::Greater,
                _ => {}
            }
        }
        match sort {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Size => b.size.cmp(&a.size).then(a.name.cmp(&b.name)),
            SortKey::Modified => b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)),
        }
    });
    v
}

fn places_path() -> PathBuf {
    let p = config::finick_root().join("places.json");
    if !p.exists() {
        let legacy = env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or(PathBuf::from("/tmp"))
            .join(".config")
            .join("finick")
            .join("places.json");
        if legacy.exists() {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&legacy, &p);
        }
    }
    p
}

fn load_pinned() -> Vec<(String, String)> {
    let p = places_path();
    if let Ok(s) = std::fs::read_to_string(p) {
        if let Ok(v) = serde_json::from_str::<Vec<(String, String)>>(&s) {
            return v;
        }
    }
    Vec::new()
}

fn save_pinned(v: &[(String, String)]) {
    let p = places_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(v) {
        let _ = std::fs::write(p, s);
    }
}

fn trash_files_path() -> PathBuf {
    let base = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
    base.join(".local").join("share").join("Trash").join("files")
}

fn recent_path() -> PathBuf {
    let base = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
    base.join(".cache").join("finick").join("files_recent.json")
}

fn load_recent() -> Vec<String> {
    let p = recent_path();
    if let Ok(s) = std::fs::read_to_string(p) {
        if let Ok(v) = serde_json::from_str::<Vec<String>>(&s) {
            return v.into_iter().filter(|d| Path::new(d).is_dir()).take(20).collect();
        }
    }
    Vec::new()
}

fn record_recent(dir: &str) {
    if dir.is_empty() || !Path::new(dir).is_dir() {
        return;
    }
    if dir == trash_files_path().to_string_lossy().as_ref() {
        return;
    }
    let mut v = load_recent();
    v.retain(|d| d != dir);
    v.insert(0, dir.to_string());
    v.truncate(20);
    let p = recent_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(&v) {
        let _ = std::fs::write(p, s);
    }
}

fn list_devices() -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(user) = env::var("USER") {
        let media = PathBuf::from("/media").join(&user);
        if let Ok(entries) = std::fs::read_dir(&media) {
            for e in entries.filter_map(|e| e.ok()) {
                if e.path().is_dir() {
                    let name = e.file_name().to_string_lossy().to_string();
                    out.push((name.clone(), e.path().to_string_lossy().to_string()));
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir("/mnt") {
        for e in entries.filter_map(|e| e.ok()) {
            if e.path().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                out.push((name.clone(), e.path().to_string_lossy().to_string()));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn dir_sig(path: &str) -> String {
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let count = std::fs::read_dir(path).map(|r| r.count()).unwrap_or(0);
    format!("{mtime}-{count}")
}

fn small_thumb_ok(size: u64, idx: usize) -> bool { size > 0 && size < 5 * 1024 * 1024 && idx < 60 }

fn path_exists(p: &str) -> bool { Path::new(p).exists() }

fn empty_trash_all() -> Result<usize, String> {
    let dir = trash_files_path();
    let mut n = 0usize;
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for e in entries.filter_map(|e| e.ok()) {
        let p = e.path();
        let r = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        if let Err(e) = r {
            return Err(e.to_string());
        }
        n += 1;
    }
    let (_, info) = trash_dirs();
    if info.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&info) {
            for e in entries.filter_map(|e| e.ok()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let _ = LAST_TRASHED.lock().map(|mut v| v.clear());
    Ok(n)
}

fn restore_all_trash() -> Result<usize, String> {
    let dir = trash_files_path();
    let (_, info) = trash_dirs();
    let home = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
    let mut n = 0usize;
    let entries: Vec<PathBuf> =
        std::fs::read_dir(&dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok().map(|x| x.path())).collect();
    for src in entries {
        let name = src.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or("file".to_string());
        let dest = unique_dest(&home, &name);
        let r = std::fs::rename(&src, &dest);
        if let Err(e) = r {
            return Err(format!("{name}: {e}"));
        }
        let _ = std::fs::remove_file(info.join(format!("{name}.trashinfo")));
        n += 1;
    }
    let _ = LAST_TRASHED.lock().map(|mut v| v.clear());
    Ok(n)
}

fn fetch_via_index(req: index::ty::Request, fallback_dir: Option<String>, mut items_state: State<Vec<Item>>) {
    use std::sync::atomic::Ordering;
    let my_gen = LOAD_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    LOAD_BUSY.store(true, Ordering::SeqCst);
    let _ = LOAD_ERR.lock().map(|mut e| *e = None);
    items_state.set(Vec::new());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Vec<Item>, Option<String>)>();
    std::thread::spawn(move || {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let tx_clone = inner_tx.clone();
        let res = ipsea::send_command(
            App::IndexService,
            &req,
            Some(move |res: index::ty::SearchResult| {
                let _ = tx_clone.send(Item {
                    ty: if res.is_dir { ItemType::Folder } else { ItemType::File },
                    name: res.name,
                    path: res.path,
                    size: res.size.unwrap_or(0),
                    modified: res.modified,
                });
            }),
        );
        drop(inner_tx);
        let mut result: Vec<Item> = inner_rx.into_iter().collect();
        let mut op_err: Option<String> = None;
        if res.is_err() {
            if let Some(dir) = fallback_dir.as_deref() {
                if let Err(e) = std::fs::read_dir(dir) {
                    op_err = Some(format!("Cannot read directory: {e}"));
                }
            } else {
                op_err = Some(String::from("Search unavailable"));
            }
        }
        if let Some(dir) = fallback_dir.as_deref() {
            if result.is_empty() || res.is_err() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let md = entry.metadata().ok();
                        let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                        let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified = md
                            .as_ref()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());
                        result.push(Item {
                            ty: if is_dir { ItemType::Folder } else { ItemType::File },
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: entry.path().to_string_lossy().to_string(),
                            size,
                            modified,
                        });
                    }
                }
            }
        }
        result.sort_by(|a, b| match (&a.ty, &b.ty) {
            (ItemType::Folder, ItemType::File) => std::cmp::Ordering::Less,
            (ItemType::File, ItemType::Folder) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        let _ = tx.send((result, op_err));
    });
    spawn(async move {
        use std::sync::atomic::Ordering;
        match rx.recv().await {
            Some((result, op_err)) => {
                if LOAD_GEN.load(Ordering::SeqCst) != my_gen {
                    return;
                }
                let _ = LOAD_ERR.lock().map(|mut e| *e = op_err);
                LOAD_BUSY.store(false, Ordering::SeqCst);
                items_state.set(result);
            }
            None => {
                if LOAD_GEN.load(Ordering::SeqCst) != my_gen {
                    return;
                }
                let _ = LOAD_ERR.lock().map(|mut e| *e = Some(String::from("Load failed")));
                LOAD_BUSY.store(false, Ordering::SeqCst);
            }
        }
    });
}

fn load_dir(path: String, items_state: State<Vec<Item>>) {
    let fallback = path.clone();
    fetch_via_index(index::ty::Request::ListDir { path }, Some(fallback), items_state)
}

fn perform_search(query: String, items_state: State<Vec<Item>>) {
    fetch_via_index(index::ty::Request::Search { query }, None, items_state)
}

fn format_size(size: u64) -> String {
    if size == 0 {
        return String::new();
    }
    if size < 1024 {
        return String::from("0 KB");
    }
    let kb = size as f64 / 1024.0;
    if kb < 1024.0 {
        format!("{} KB", size / 1024)
    } else if kb < 1024.0 * 1024.0 {
        format!("{:.1} MB", kb / 1024.0)
    } else {
        format!("{:.1} GB", kb / 1024.0 / 1024.0)
    }
}

fn trash_dirs() -> (PathBuf, PathBuf) {
    let files = trash_files_path();
    let info = files.parent().map(|p| p.join("info")).unwrap_or(PathBuf::from("/tmp"));
    (files, info)
}

fn move_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(p) = dst.parent() {
        std::fs::create_dir_all(p)?;
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            if src.is_dir() {
                copy_dir_recursive(&src.to_path_buf(), &dst.to_path_buf())?;
                std::fs::remove_dir_all(src)?;
            } else {
                std::fs::copy(src, dst)?;
                std::fs::remove_file(src)?;
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn move_to_trash(orig: &Path) -> std::io::Result<(String, String)> {
    let (files, info) = trash_dirs();
    std::fs::create_dir_all(&files)?;
    std::fs::create_dir_all(&info)?;
    let name = orig.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(String::from("item"));
    let mut uname = name.clone();
    let mut n = 1;
    while files.join(&uname).exists() || info.join(format!("{uname}.trashinfo")).exists() {
        n += 1;
        uname = format!("{name}.{n}");
    }
    move_path(orig, &files.join(&uname))?;
    let date = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let content = format!("[Trash Info]\nPath={}\nDeletionDate={}\n", orig.to_string_lossy(), date);
    if let Err(e) = std::fs::write(info.join(format!("{uname}.trashinfo")), content) {
        let _ = move_path(&files.join(&uname), orig);
        return Err(e);
    }
    Ok((uname, orig.to_string_lossy().to_string()))
}

fn restore_last_trashed() -> Result<usize, String> {
    let pairs: Vec<(String, String)> =
        LAST_TRASHED.lock().map(|mut v| std::mem::take(&mut *v)).map_err(|e| e.to_string())?;
    if pairs.is_empty() {
        return Ok(0);
    }
    let (files, info) = trash_dirs();
    let mut n = 0usize;
    let mut errs = Vec::new();
    for (uname, orig) in pairs {
        let src = files.join(&uname);
        let dest = PathBuf::from(&orig);
        let dest = if dest.exists() {
            let parent = dest.parent().map(|p| p.to_path_buf()).unwrap_or(PathBuf::from("/"));
            let name = dest.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or(String::from("item"));
            unique_dest(&parent, &name)
        } else {
            dest
        };
        match move_path(&src, &dest) {
            Ok(()) => {
                let _ = std::fs::remove_file(info.join(format!("{uname}.trashinfo")));
                n += 1;
            }
            Err(e) => errs.push(format!("{orig}: {e}")),
        }
    }
    if errs.is_empty() { Ok(n) } else { Err(errs.join("; ")) }
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let ty = e.file_type()?;
        let dst_path = dst.join(e.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&e.path(), &dst_path)?;
        } else {
            std::fs::copy(e.path(), dst_path)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive_cancel(src: &PathBuf, dst: &PathBuf, cancel: State<bool>) -> std::io::Result<()> {
    if *cancel.read() {
        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled"));
    }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        if *cancel.read() {
            return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled"));
        }
        let e = e?;
        let ty = e.file_type()?;
        let dst_path = dst.join(e.file_name());
        if ty.is_dir() {
            copy_dir_recursive_cancel(&e.path(), &dst_path, cancel)?;
        } else {
            std::fs::copy(e.path(), dst_path)?;
        }
    }
    Ok(())
}

fn selection_handle_press(
    path: String,
    item: Item,
    mut selected_paths: State<Vec<String>>,
    mut anchor: State<Option<String>>,
    mut focus: State<Option<Item>>,
    ordered: Vec<String>,
    ctrl: bool,
    shift: bool,
) {
    let mut cur = selected_paths.read().clone();
    if shift {
        if let Some(a) = anchor.read().clone() {
            if let (Some(ai), Some(ci)) = (ordered.iter().position(|p| p == &a), ordered.iter().position(|p| p == &path)) {
                let (lo, hi) = if ai <= ci { (ai, ci) } else { (ci, ai) };
                for p in &ordered[lo..=hi] {
                    if !cur.contains(p) {
                        cur.push(p.clone());
                    }
                }
                selected_paths.set(cur);
                focus.set(Some(item));
                return;
            }
        }
        if !cur.contains(&path) {
            cur.push(path.clone());
        }
        selected_paths.set(cur);
        anchor.set(Some(path));
        focus.set(Some(item));
        return;
    }
    if ctrl {
        if cur.contains(&path) {
            cur.retain(|p| p != &path);
        } else {
            cur.push(path.clone());
        }
        let has = cur.contains(&path);
        selected_paths.set(cur);
        anchor.set(Some(path.clone()));
        if has {
            focus.set(Some(item));
        } else if focus.read().as_ref().is_some_and(|f| f.path == path) {
            focus.set(None);
        }
        return;
    }
    selected_paths.set(vec![path.clone()]);
    anchor.set(Some(path));
    focus.set(Some(item));
}

fn unique_dest(dir: &Path, name: &str) -> PathBuf {
    let base = dir.join(name);
    if !base.exists() {
        return base;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let mut n = 1;
    loop {
        let cand = dir.join(format!("{stem} copy{n}{ext}"));
        if !cand.exists() {
            return cand;
        }
        n += 1;
    }
}

fn create_new_file_in(dir: &str) -> std::io::Result<PathBuf> {
    let d = PathBuf::from(dir);
    std::fs::create_dir_all(&d)?;
    let mut n = 0;
    loop {
        let name = if n == 0 { "New File".to_string() } else { format!("New File ({n})") };
        let dest = d.join(&name);
        if !dest.exists() {
            std::fs::write(&dest, "")?;
            return Ok(dest);
        }
        n += 1;
    }
}

fn duplicate_one(src: &str) -> std::io::Result<PathBuf> {
    let sp = PathBuf::from(src);
    let parent = sp.parent().map(|p| p.to_path_buf()).unwrap_or(PathBuf::from("/"));
    let name = sp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("file".to_string());
    let dest = unique_dest(&parent, &name);
    if sp.is_dir() {
        copy_dir_recursive(&sp, &dest)?;
    } else {
        std::fs::copy(&sp, &dest)?;
    }
    Ok(dest)
}

fn format_modified(ts: Option<u64>) -> String {
    match ts {
        Some(v) => {
            chrono::DateTime::from_timestamp(v as i64, 0).map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
        }
        None => String::from("Unknown"),
    }
}

fn format_perms(path: &str) -> String {
    let md = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return String::from("Unknown"),
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = md.permissions().mode() & 0o777;
        let bit = |b: u32, c: char| if mode & b != 0 { c } else { '-' };
        format!(
            "{}{}{}{}{}{}{}{}{} ({mode:o})",
            bit(0o400, 'r'),
            bit(0o200, 'w'),
            bit(0o100, 'x'),
            bit(0o40, 'r'),
            bit(0o20, 'w'),
            bit(0o10, 'x'),
            bit(0o4, 'r'),
            bit(0o2, 'w'),
            bit(0o1, 'x')
        )
    }
    #[cfg(not(unix))]
    {
        if md.permissions().readonly() { String::from("Read-only") } else { String::from("Read-write") }
    }
}

fn file_type_label(name: &str, is_dir: bool) -> String {
    if is_dir {
        return String::from("Directory");
    }
    match name.to_lowercase().rsplit('.').next().unwrap_or("") {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => String::from("Image"),
        "mp4" | "mkv" | "avi" | "mov" | "webm" => String::from("Video"),
        "mp3" | "wav" | "flac" | "ogg" => String::from("Audio"),
        "pdf" => String::from("PDF document"),
        "zip" | "tar" | "gz" | "xz" | "7z" | "rar" => String::from("Archive"),
        "rs" | "ts" | "js" | "py" | "go" | "c" | "cpp" | "sh" | "toml" | "json" | "yaml" => String::from("Code"),
        "txt" | "md" | "log" | "csv" => String::from("Text"),
        ext if ext.is_empty() => String::from("File"),
        ext => format!("{} file", ext.to_uppercase()),
    }
}

fn open_with_chooser(path: &str) {
    let p = path.to_string();
    let _ = std::process::Command::new("mimeopen")
        .arg("-d")
        .arg(&p)
        .spawn()
        .or_else(|_| std::process::Command::new("xdg-open").arg(&p).spawn());
}

fn show_in_folder(path: &str) {
    let parent = PathBuf::from(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or("/".to_string());
    let _ = std::process::Command::new("xdg-open").arg(&parent).spawn();
}

#[allow(clippy::too_many_arguments)]
fn start_paste_job_multi(
    srcs: Vec<String>,
    overwrite: bool,
    is_cut: bool,
    dest_dir: String,
    items: State<Vec<Item>>,
    mut notice: State<Option<(String, NoticeKind)>>,
    mut job_running: State<bool>,
    mut job_label: State<String>,
    mut job_progress: State<Option<JobProgress>>,
    job_cancel: State<bool>,
    mut clipboard: State<Option<(Vec<String>, bool)>>,
) {
    let total = srcs.len();
    job_label.set(if is_cut { "Moving...".to_string() } else { "Copying...".to_string() });
    job_running.set(true);
    let mut jc = job_cancel;
    jc.set(false);
    job_progress.set(Some(JobProgress {
        label: if is_cut { "Moving".to_string() } else { "Copying".to_string() },
        done: 0,
        total,
        current: String::new(),
    }));
    spawn(async move {
        let mut ok = 0usize;
        let mut skipped = 0usize;
        let mut cancelled = false;
        let mut err: Option<String> = None;
        for (i, src) in srcs.iter().enumerate() {
            if *job_cancel.read() {
                cancelled = true;
                break;
            }
            let name = PathBuf::from(src).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("file".to_string());
            let dest = PathBuf::from(&dest_dir).join(&name);
            job_progress.set(Some(JobProgress {
                label: if is_cut { "Moving".to_string() } else { "Copying".to_string() },
                done: i,
                total,
                current: name.clone(),
            }));
            let r: Result<(), String> = (|| {
                if dest.exists() {
                    if !overwrite {
                        skipped += 1;
                        return Ok(());
                    }
                    if dest.is_dir() {
                        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
                    } else {
                        std::fs::remove_file(&dest).map_err(|e| e.to_string())?;
                    }
                }
                if is_cut {
                    std::fs::rename(src, &dest).map_err(|e| e.to_string())
                } else if PathBuf::from(src).is_dir() {
                    copy_dir_recursive_cancel(&PathBuf::from(src), &dest, job_cancel).map_err(|e| e.to_string())
                } else {
                    if let Some(p) = dest.parent() {
                        let _ = std::fs::create_dir_all(p);
                    }
                    std::fs::copy(src, &dest).map(|_| ()).map_err(|e| e.to_string())
                }
            })();
            if let Err(e) = r {
                if e == "cancelled" {
                    cancelled = true;
                } else {
                    err = Some(format!("{name}: {e}"));
                }
                break;
            }
            ok += 1;
        }
        job_running.set(false);
        job_progress.set(None);
        if cancelled {
            notice.set(Some(("Cancelled".to_string(), NoticeKind::Warn)));
        } else if let Some(e) = err {
            notice.set(Some((format!("Paste failed: {e}"), NoticeKind::Error)));
        } else {
            if is_cut {
                clipboard.set(None);
            }
            let mut msg = format!("{ok} item{} {}", if ok == 1 { "" } else { "s" }, if is_cut { "moved" } else { "copied" });
            if skipped > 0 {
                msg.push_str(&format!(" ({skipped} skipped, already exist)"));
            }
            notice.set(Some((msg, NoticeKind::Success)));
            load_dir(dest_dir, items);
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn do_paste_request_multi(
    clip: Option<(Vec<String>, bool)>,
    dest_dir: String,
    items: State<Vec<Item>>,
    mut notice: State<Option<(String, NoticeKind)>>,
    job_running: State<bool>,
    job_label: State<String>,
    job_progress: State<Option<JobProgress>>,
    job_cancel: State<bool>,
    clipboard: State<Option<(Vec<String>, bool)>>,
    pending_paste: State<Option<PendingPaste>>,
    show_overwrite: State<bool>,
) {
    if let Some((srcs, is_cut)) = clip {
        let conflicts: Vec<String> = srcs
            .iter()
            .filter(|s| {
                let name = PathBuf::from(s).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                PathBuf::from(&dest_dir).join(&name).exists()
            })
            .cloned()
            .collect();
        if !conflicts.is_empty() {
            let mut p = pending_paste;
            p.set(Some(PendingPaste { srcs, is_cut, dest_dir }));
            let mut s = show_overwrite;
            s.set(true);
            return;
        }
        let missing: Vec<String> = srcs
            .iter()
            .filter(|s| {
                let name = PathBuf::from(s).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                !PathBuf::from(&dest_dir).join(&name).exists()
            })
            .cloned()
            .collect();
        if missing.len() != srcs.len() {
            notice.set(Some(("Some items exist, copied the rest".to_string(), NoticeKind::Warn)));
        }
        if !missing.is_empty() {
            start_paste_job_multi(
                missing,
                false,
                is_cut,
                dest_dir,
                items,
                notice,
                job_running,
                job_label,
                job_progress,
                job_cancel,
                clipboard,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn confirm_trash_delete_multi(
    paths: Vec<String>,
    items: State<Vec<Item>>,
    mut notice: State<Option<(String, NoticeKind)>>,
    mut job_running: State<bool>,
    mut job_label: State<String>,
    mut job_progress: State<Option<JobProgress>>,
    job_cancel: State<bool>,
    current_dir: String,
) {
    let total = paths.len();
    job_label.set("Moving to Trash...".to_string());
    job_running.set(true);
    let mut jc = job_cancel;
    jc.set(false);
    job_progress.set(Some(JobProgress { label: "Trashing".to_string(), done: 0, total, current: String::new() }));
    spawn(async move {
        let mut cancelled = false;
        let mut err: Option<String> = None;
        let mut n = 0usize;
        let mut trashed: Vec<(String, String)> = Vec::new();
        for (i, p) in paths.iter().enumerate() {
            if *job_cancel.read() {
                cancelled = true;
                break;
            }
            job_progress.set(Some(JobProgress { label: "Trashing".to_string(), done: i, total, current: p.clone() }));
            match move_to_trash(Path::new(p)) {
                Ok((u, o)) => {
                    trashed.push((u, o));
                    n += 1;
                }
                Err(e) => {
                    let name = PathBuf::from(p).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or(p.clone());
                    err = Some(format!("{name}: {e}"));
                    break;
                }
            }
        }
        job_running.set(false);
        job_progress.set(None);
        if cancelled {
            notice.set(Some(("Cancelled".to_string(), NoticeKind::Warn)));
        } else if let Some(e) = err {
            if !trashed.is_empty() {
                let _ = LAST_TRASHED.lock().map(|mut v| v.extend(trashed));
            }
            notice.set(Some((format!("Trash failed: {e}"), NoticeKind::Error)));
        } else {
            if !trashed.is_empty() {
                let _ = LAST_TRASHED.lock().map(|mut v| v.extend(trashed));
            }
            notice.set(Some((
                format!("Moved {n} item{} to Trash (Undo available)", if n == 1 { "" } else { "s" }),
                NoticeKind::Success,
            )));
            load_dir(current_dir, items);
        }
    });
}

fn app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let current_path = use_state(|| env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()));
    let items = use_state(Vec::<Item>::new);
    let mut selected_item = use_state(|| Option::<Item>::None);
    let view_mode = use_state(|| ViewMode::Grid);
    let icon_size = use_state(|| IconSize::Large);
    let uri_input = use_state(String::new);
    let search_query = use_state(String::new);
    let pinned = use_state(load_pinned);
    let show_add_place = use_state(|| false);
    let add_place_name = use_state(String::new);
    let rename_target = use_state(|| Option::<Item>::None);
    let rename_input = use_state(String::new);
    let clipboard = use_state(|| Option::<(Vec<String>, bool)>::None);
    let mut selected_paths = use_state(Vec::<String>::new);
    let mut anchor_path = use_state(|| Option::<String>::None);
    let ctrl_held = use_state(|| false);
    let shift_held = use_state(|| false);
    let show_hidden = use_state(|| false);
    let notice = use_state(|| Option::<(String, NoticeKind)>::None);
    let job_running = use_state(|| false);
    let job_label = use_state(String::new);
    let job_progress = use_state(|| Option::<JobProgress>::None);
    let job_cancel = use_state(|| false);
    let show_rename = use_state(|| false);
    let show_sidebar = use_state(|| true);
    let last_click: State<Option<(String, std::time::Instant)>> = use_state(|| None);
    let drag_start: State<Option<(f64, f64)>> = use_state(|| None);
    let drag_current: State<Option<(f64, f64)>> = use_state(|| None);
    let last_cp = use_state(|| env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string()));
    let loaded = use_state(|| false);
    let history = use_state(|| vec![env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string())]);
    let hist_idx = use_state(|| 0usize);
    let hist_nav = use_state(|| false);
    let sort_key = use_state(|| SortKey::Name);
    let dirs_first = use_state(|| true);
    let free_text = use_state(|| Option::<String>::None);
    let load_gen = use_state(|| 0u64);
    let is_loading = use_state(|| false);
    let load_error = use_state(|| Option::<String>::None);
    let show_delete_confirm = use_state(|| false);
    let pending_delete = use_state(Vec::<String>::new);
    let show_overwrite = use_state(|| false);
    let pending_paste = use_state(|| Option::<PendingPaste>::None);
    {
        use std::sync::atomic::Ordering;
        let g = LOAD_GEN.load(Ordering::SeqCst);
        if *load_gen.read() != g {
            let mut s = load_gen;
            s.set(g);
        }
        let b = LOAD_BUSY.load(Ordering::SeqCst);
        if *is_loading.read() != b {
            let mut s = is_loading;
            s.set(b);
        }
        let e = LOAD_ERR.lock().map(|v| v.clone()).unwrap_or(None);
        if *load_error.read() != e {
            let mut s = load_error;
            s.set(e);
        }
    }
    let recent_dirs = use_state(load_recent);
    let watch_tick = use_state(|| 0u64);
    let watch_sig = use_state(|| Option::<String>::None);
    use_hook(move || {
        let mut t = watch_tick;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                t.set(*t.read() + 1);
            }
        });
    });
    use_side_effect(move || {
        let _tick = *watch_tick.read();
        let path = current_path.read().clone();
        let sig = dir_sig(&path);
        let prev = watch_sig.read().clone();
        if prev.is_none() {
            let mut w = watch_sig;
            w.set(Some(sig));
        } else if prev.is_some_and(|p| p != sig) {
            let mut w = watch_sig;
            w.set(Some(sig));
            load_dir(path, items);
        }
    });

    let cp = current_path.read().clone();
    if *last_cp.read() != cp {
        let mut lcp = last_cp;
        let mut uin = uri_input;
        lcp.set(cp.clone());
        uin.set(cp.clone());
        let mut hn = hist_nav;
        if *hn.read() {
            hn.set(false);
        } else {
            let mut h = history;
            let mut hi = hist_idx;
            let mut hv = h.read().clone();
            let cur_i = *hi.read();
            hv.truncate(cur_i.saturating_add(1));
            if hv.last().is_none_or(|l| l != &cp) {
                hv.push(cp.clone());
                hi.set(hv.len().saturating_sub(1));
                h.set(hv);
            }
        }
        selected_item.set(None);
        selected_paths.set(Vec::new());
        anchor_path.set(None);
        let mut ft = free_text;
        ft.set(free_space_for(&cp));
        record_recent(&cp);
        let mut rd = recent_dirs;
        rd.set(load_recent());
        let mut ws = watch_sig;
        ws.set(Some(dir_sig(&cp)));
    }

    if !*loaded.read() {
        let mut l = loaded;
        l.set(true);
        load_dir(current_path.read().clone(), items);
    }

    let pinned_val = pinned.read().clone();
    if !pinned_val.is_empty() {
        let _ = save_pinned(&pinned_val);
    }
    let home_path = env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let docs_path = env::home_dir().map(|v| v.join("Documents").to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let dl_path = env::home_dir().map(|v| v.join("Downloads").to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let pic_path = env::home_dir().map(|v| v.join("Pictures").to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let trash_path = trash_files_path().to_string_lossy().to_string();
    let devices = list_devices();
    let recent_list = recent_dirs.read().clone();
    let is_trash_view = cp == trash_path;

    let sidebar = rect()
        .width(Size::px(220.))
        .height(Size::fill())
        .background(t.bg_sidebar)
        .border(Border::new().width(1.).fill(t.border_card))
        .padding(12.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .main_align(Alignment::SpaceBetween)
                .margin((0., 0., 12., 0.))
                .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("PLACES"))
                .child(
                    rect()
                        .cursor(CursorIcon::Pointer)
                        .padding((2., 6.))
                        .corner_radius(6.)
                        .on_press({
                            let mut s = show_sidebar;
                            move |_| s.set(false)
                        })
                        .child(label().font_size(10.).color(t.text_muted).text("Collapse")),
                ),
        )
        .child(sidebar_entry("Home", HOME, &current_path, home_path.clone(), items, pinned, clipboard))
        .maybe(path_exists(&docs_path), |el| {
            el.child(sidebar_entry("Documents", DOCUMENT, &current_path, docs_path.clone(), items, pinned, clipboard))
        })
        .maybe(path_exists(&dl_path), |el| {
            el.child(sidebar_entry("Downloads", DOWNLOAD, &current_path, dl_path.clone(), items, pinned, clipboard))
        })
        .maybe(path_exists(&pic_path), |el| {
            el.child(sidebar_entry("Pictures", PICTURE, &current_path, pic_path.clone(), items, pinned, clipboard))
        })
        .child(sidebar_entry("Trash", TRASH, &current_path, trash_path.clone(), items, pinned, clipboard))
        .maybe(!devices.is_empty(), |el| {
            el.child(
                rect()
                    .margin((12., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(
                        label()
                            .font_size(11.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_muted)
                            .margin((0., 0., 8., 0.))
                            .text("DEVICES"),
                    )
                    .children(devices.iter().map(|(name, path)| {
                        sidebar_entry(name, STORAGE, &current_path, path.clone(), items, pinned, clipboard)
                    })),
            )
        })
        .maybe(!recent_list.is_empty(), |el| {
            el.child(
                rect()
                    .margin((12., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(
                        label()
                            .font_size(11.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_muted)
                            .margin((0., 0., 8., 0.))
                            .text("RECENT"),
                    )
                    .children(recent_list.iter().map(|path| {
                        let name =
                            PathBuf::from(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(path.clone());
                        sidebar_entry(&name, CLOCK_ICON, &current_path, path.clone(), items, pinned, clipboard)
                    })),
            )
        })
        .child(if !pinned.read().is_empty() {
            rect()
                .margin((12., 0., 0., 0.))
                .content(Content::Flex)
                .child(
                    label()
                        .font_size(11.)
                        .font_weight(FontWeight::BOLD)
                        .color(t.text_muted)
                        .margin((0., 0., 8., 0.))
                        .text("PINNED"),
                )
                .children(
                    pinned
                        .read()
                        .iter()
                        .map(|(name, path)| sidebar_entry(name, PIN, &current_path, path.clone(), items, pinned, clipboard)),
                )
                .into_element()
        } else {
            rect().into_element()
        })
        .child(
            rect().margin((12., 0., 0., 0.)).child(
                rect()
                    .width(Size::fill())
                    .padding(8.)
                    .corner_radius(8.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .cursor(CursorIcon::Pointer)
                    .on_press({
                        let mut show = show_add_place;
                        let mut name_st = add_place_name;
                        let cp2 = current_path.read().clone();
                        move |_| {
                            let base = PathBuf::from(&cp2)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or("Place".to_string());
                            name_st.set(base);
                            show.set(true);
                        }
                    })
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(6.)
                            .content(Content::Flex)
                            .child(icon(PLUS, 12., t.text_primary))
                            .child(label().font_size(12.).color(t.text_primary).text("Add Place")),
                    ),
            ),
        );

    let top_bar = rect()
        .height(Size::px(56.))
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .padding(10.)
        .spacing(8.)
        .background(t.bg_base)
        .border(Border::new().width(1.).fill(t.border_card))
        .child(
            rect()
                .padding(8.)
                .corner_radius(8.)
                .background(if *show_sidebar.read() { t.bg_card } else { t.bg_surface })
                .border(Border::new().width(1.).fill(t.border_card))
                .cursor(CursorIcon::Pointer)
                .on_press({
                    let mut s = show_sidebar;
                    move |_| s.set(!*s.read())
                })
                .child(icon(SIDEBAR, 16., t.text_primary)),
        )
        .child(icon_button("‹", {
            let mut current = current_path;
            let items = items;
            let h = history;
            let mut hi = hist_idx;
            let mut hn = hist_nav;
            move || {
                let i = *hi.read();
                let hv = h.read().clone();
                if i > 0 && i < hv.len() {
                    hn.set(true);
                    let target = hv[i - 1].clone();
                    hi.set(i - 1);
                    current.set(target.clone());
                    load_dir(target, items);
                }
            }
        }))
        .child(icon_button("›", {
            let mut current = current_path;
            let items = items;
            let h = history;
            let mut hi = hist_idx;
            let mut hn = hist_nav;
            move || {
                let i = *hi.read();
                let hv = h.read().clone();
                if i + 1 < hv.len() {
                    hn.set(true);
                    let target = hv[i + 1].clone();
                    hi.set(i + 1);
                    current.set(target.clone());
                    load_dir(target, items);
                }
            }
        }))
        .child(icon_button("↻", {
            let current = current_path;
            let items = items;
            move || {
                load_dir(current.read().clone(), items);
            }
        }))
        .child(
            rect()
                .padding(8.)
                .corner_radius(8.)
                .background(t.bg_card)
                .border(Border::new().width(1.).fill(t.border_card))
                .cursor(CursorIcon::Pointer)
                .on_press({
                    let mut current = current_path;
                    let items = items;
                    move |_| {
                        let path = PathBuf::from(current.read().clone());
                        if let Some(parent) = path.parent() {
                            let new_path = parent.to_string_lossy().to_string();
                            current.set(new_path.clone());
                            load_dir(new_path, items);
                        }
                    }
                })
                .child(icon(ARROW_LEFT, 16., t.text_primary)),
        )
        .child(
            rect()
                .width(Size::flex(1.))
                .height(Size::px(36.))
                .cross_align(Alignment::Center)
                .content(Content::Flex)
                .background(t.bg_card)
                .border(Border::new().width(1.).fill(t.border_card))
                .corner_radius(8.)
                .padding((2., 10.))
                .child(Input::new(uri_input).width(Size::fill()).flat().placeholder("Path").on_submit({
                    let mut current = current_path;
                    let items = items;
                    move |val: String| {
                        current.set(val.clone());
                        load_dir(val, items);
                    }
                })),
        )
        .child(
            rect()
                .horizontal()
                .main_align(Alignment::End)
                .cross_align(Alignment::Center)
                .content(Content::Flex)
                .spacing(6.)
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .width(Size::px(220.))
                        .background(t.bg_card)
                        .border(Border::new().width(1.).fill(t.border_card))
                        .corner_radius(999.)
                        .padding((2., 6.))
                        .spacing(4.)
                        .child(icon(SEARCH, 14., t.text_muted))
                        .child(Input::new(search_query).width(Size::fill()).flat().placeholder("Search").on_submit({
                            let items = items;
                            move |val: String| {
                                perform_search(val, items);
                            }
                        })),
                )
                .maybe(!search_query.read().is_empty(), |el| {
                    el.child(status_chip("Search", true, None)).child(ghost_button("Clear", {
                        let mut q = search_query;
                        let cp = current_path;
                        let items_c = items;
                        move || {
                            q.set(String::new());
                            load_dir(cp.read().clone(), items_c);
                        }
                    }))
                })
                .child(
                    rect()
                        .padding(6.)
                        .corner_radius(8.)
                        .background(t.bg_card)
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut s = icon_size;
                            move |_| s.set(s.read().next())
                        })
                        .child(label().font_size(11.).color(t.text_primary).text(icon_size.read().label().to_string())),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(if *view_mode.read() == ViewMode::Grid { t.bg_active } else { t.bg_card })
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut vm = view_mode;
                            move |_| vm.set(ViewMode::Grid)
                        })
                        .child(icon(
                            LAYOUT_GRID,
                            14.,
                            if *view_mode.read() == ViewMode::Grid { t.text_primary } else { t.text_muted },
                        )),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(if *view_mode.read() == ViewMode::List { t.bg_active } else { t.bg_card })
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let mut vm = view_mode;
                            move |_| vm.set(ViewMode::List)
                        })
                        .child(icon(
                            LIST,
                            14.,
                            if *view_mode.read() == ViewMode::List { t.text_primary } else { t.text_muted },
                        )),
                )
                .child(
                    rect()
                        .padding(8.)
                        .corner_radius(8.)
                        .background(t.primary_accent)
                        .cursor(CursorIcon::Pointer)
                        .on_press({
                            let path_state = current_path;
                            let items_state = items;
                            move |_| {
                                let target_path = PathBuf::from(path_state.read().clone()).join("New Folder");
                                let mut unique_path = target_path.clone();
                                let mut counter = 1;
                                while unique_path.exists() {
                                    unique_path =
                                        PathBuf::from(path_state.read().clone()).join(format!("New Folder ({})", counter));
                                    counter += 1;
                                }
                                let _ = std::fs::create_dir_all(&unique_path);
                                load_dir(path_state.read().clone(), items_state);
                            }
                        })
                        .child(icon(PLUS, 14., t.bg_base)),
                ),
        );

    let is_grid = *view_mode.read() == ViewMode::Grid;
    let show_hidden_val = *show_hidden.read();
    let sort_val = *sort_key.read();
    let dirs_first_val = *dirs_first.read();
    let items_read: Vec<Item> = visible_items(&items.read(), sort_val, dirs_first_val, show_hidden_val);
    let items_total = items_read.len();
    let ordered_paths: Vec<String> = items_read.iter().map(|i| i.path.clone()).collect();
    let icon_px = icon_size.read().px();

    let crumb_bar: Element = {
        let segs = crumbs_for(&cp);
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .width(Size::fill())
            .spacing(4.)
            .padding((4., 12., 0., 12.))
            .children(segs.into_iter().map(|(name, full)| {
                let target = full.clone();
                let mut current = current_path;
                let items_c = items;
                ghost_button(name, move || {
                    current.set(target.clone());
                    load_dir(target.clone(), items_c);
                })
                .into_element()
            }))
            .into_element()
    };

    let tool_bar: Element = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .width(Size::fill())
        .spacing(GAP)
        .padding((4., 12., 0., 12.))
        .child(segmented_control(
            vec![("Name", SortKey::Name), ("Size", SortKey::Size), ("Modified", SortKey::Modified)],
            sort_val,
            {
                let mut s = sort_key;
                move |v: SortKey| s.set(v)
            },
        ))
        .child(focus_pill("Dirs first", dirs_first_val, {
            let mut d = dirs_first;
            move || d.set(!*d.read())
        }))
        .child(focus_pill("Hidden", show_hidden_val, {
            let mut h = show_hidden;
            move || h.set(!*h.read())
        }))
        .into_element();

    let trash_bar: Element = if is_trash_view {
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .width(Size::fill())
            .spacing(GAP)
            .padding((4., 12., 0., 12.))
            .child(label().font_size(12.).color(t.text_muted).text("Trash"))
            .child(secondary_button("Restore all", {
                let items_c = items;
                let mut note = notice;
                let cp_c = current_path;
                move || match restore_all_trash() {
                    Ok(n) => {
                        note.set(Some((format!("Restored {n} item{}", if n == 1 { "" } else { "s" }), NoticeKind::Success)));
                        load_dir(cp_c.read().clone(), items_c);
                    }
                    Err(e) => note.set(Some((format!("Restore failed: {e}"), NoticeKind::Error))),
                }
            }))
            .child(danger_button("Empty trash", {
                let items_c = items;
                let mut note = notice;
                let cp_c = current_path;
                move || match empty_trash_all() {
                    Ok(n) => {
                        note.set(Some((format!("Emptied {n} item{}", if n == 1 { "" } else { "s" }), NoticeKind::Success)));
                        load_dir(cp_c.read().clone(), items_c);
                    }
                    Err(e) => note.set(Some((format!("Empty failed: {e}"), NoticeKind::Error))),
                }
            }))
            .into_element()
    } else {
        rect().into_element()
    };

    let bottom_bar: Element = {
        let sel_multi = selected_paths.read().clone();
        let sel_one = selected_item.read().clone();
        let sel_count = if sel_multi.is_empty() { sel_one.as_ref().map(|_| 1).unwrap_or(0) } else { sel_multi.len() };
        let sel_bytes = if sel_multi.is_empty() {
            sel_one.as_ref().map(|s| s.size).unwrap_or(0)
        } else {
            items.read().iter().filter(|i| sel_multi.contains(&i.path)).map(|i| i.size).sum()
        };
        let free_s = free_text.read().clone().unwrap_or_default();
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .width(Size::fill())
            .spacing(8.)
            .padding((6., 12.))
            .background(t.panel)
            .border(Border::new().width(1.).fill(t.border))
            .child(status_chip(format!("{items_total} items"), false, None))
            .child(status_chip(
                if sel_count > 0 {
                    format!("{sel_count} selected ({})", format_size(sel_bytes))
                } else {
                    "Nothing selected".to_string()
                },
                sel_count > 0,
                None,
            ))
            .maybe(!free_s.is_empty(), |el| el.child(status_chip(format!("{free_s} free"), false, None)))
            .into_element()
    };

    let drag_box: Element = if let (Some((x1, y1)), Some((x2, y2))) = (*drag_start.read(), *drag_current.read()) {
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);
        let w = max_x - min_x;
        let h = max_y - min_y;
        if w > 3.0 || h > 3.0 {
            rect()
                .position(Position::new_absolute().top(min_y as f32).left(min_x as f32))
                .width(Size::px(w as f32))
                .height(Size::px(h as f32))
                .background(Color::from_argb(40, 50, 120, 240))
                .border(Border::new().width(1.).fill(Color::from_argb(180, 50, 120, 240)))
                .corner_radius(2.)
                .into_element()
        } else {
            rect().into_element()
        }
    } else {
        rect().into_element()
    };

    let items_view = ScrollView::new().width(Size::fill()).height(Size::fill()).child(
        rect()
            .direction(if is_grid { Direction::Horizontal } else { Direction::Vertical })
            .width(Size::fill())
            .padding(12.)
            .content(if is_grid { Content::Wrap { wrap_spacing: Some(6.0) } } else { Content::Flex })
            .on_pointer_down({
                let mut ds = drag_start;
                let mut dc = drag_current;
                let mut sel = selected_item;
                let mut multi = selected_paths;
                let ctrl = ctrl_held;
                let shift = shift_held;
                move |e: Event<PointerEventData>| {
                    let p = e.data().element_location();
                    ds.set(Some((p.x as f64, p.y as f64)));
                    dc.set(Some((p.x as f64, p.y as f64)));
                    if !*ctrl.read() && !*shift.read() {
                        sel.set(None);
                        multi.set(Vec::new());
                    }
                }
            })
            .on_global_pointer_move({
                let ds = drag_start;
                let mut dc = drag_current;
                move |e: Event<PointerEventData>| {
                    if ds.read().is_some() {
                        let p = e.data().element_location();
                        dc.set(Some((p.x as f64, p.y as f64)));
                    }
                }
            })
            .on_global_pointer_press({
                let mut ds = drag_start;
                let mut dc = drag_current;
                move |_| {
                    ds.set(None);
                    dc.set(None);
                }
            })
            .child(drag_box)
            .children(items_read.into_iter().enumerate().map(|(idx, item)| {
                let path_state = current_path;
                let items_state = items;
                let sel_state = selected_item;
                let multi_state = selected_paths;
                let anchor_state = anchor_path;
                let ctrl_state = ctrl_held;
                let shift_state = shift_held;
                let ordered = ordered_paths.clone();
                let mut rename_t = rename_target;
                let mut rename_in = rename_input;
                let mut show_rn = show_rename;
                let path = item.path.clone();
                let is_folder = item.ty == ItemType::Folder;
                let is_selected =
                    multi_state.read().contains(&path) || sel_state.read().as_ref().is_some_and(|s| s.path == path);
                let is_cut = clipboard.read().clone().is_some_and(|(paths, cut)| cut && paths.contains(&path));
                let item_clone = item.clone();
                let item_for_menu = item.clone();
                let bg = if is_selected { t.bg_selected } else { Color::TRANSPARENT };
                let i_svg = file_icon_for(&item.name, is_folder);
                let is_img = !is_folder && is_image_ext(&item.name);

                let ctx_menu = {
                    let path_c = item_for_menu.path.clone();
                    let name_c = item_for_menu.name.clone();
                    let is_folder_c = is_folder;
                    let cp_c = current_path;
                    let items_c = items;
                    let mut clip_c = clipboard;
                    Menu::new()
                        .child(ctx_button(if is_folder_c { "Open" } else { "Open File" }, {
                            let pc = path_c.clone();
                            let mut ps = path_state;
                            let is2 = is_folder_c;
                            move || {
                                if is2 {
                                    ps.set(pc.clone());
                                    load_dir(pc.clone(), items_c);
                                } else {
                                    let _ = std::process::Command::new("xdg-open").arg(&pc).spawn();
                                }
                            }
                        }))
                        .child(ctx_button("Cut", {
                            let pc = path_c.clone();
                            let multi = multi_state.read().clone();
                            move || {
                                let mut v = multi.clone();
                                if !v.contains(&pc) {
                                    v.push(pc.clone());
                                }
                                clip_c.set(Some((v, true)));
                            }
                        }))
                        .child(ctx_button("Copy", {
                            let pc = path_c.clone();
                            let multi = multi_state.read().clone();
                            move || {
                                let mut v = multi.clone();
                                if !v.contains(&pc) {
                                    v.push(pc.clone());
                                }
                                clip_c.set(Some((v, false)));
                            }
                        }))
                        .child(ctx_button("Paste", {
                            let pc = if is_folder_c { path_c.clone() } else { cp_c.read().clone() };
                            let clip_v = clip_c.read().clone();
                            move || {
                                do_paste_request_multi(
                                    clip_v.clone(),
                                    pc.clone(),
                                    items_c,
                                    notice,
                                    job_running,
                                    job_label,
                                    job_progress,
                                    job_cancel,
                                    clip_c,
                                    pending_paste,
                                    show_overwrite,
                                );
                            }
                        }))
                        .child(ctx_button("Duplicate", {
                            let pc = path_c.clone();
                            let mut note = notice;
                            move || match duplicate_one(&pc) {
                                Ok(_) => {
                                    note.set(Some(("Duplicated".to_string(), NoticeKind::Success)));
                                    load_dir(cp_c.read().clone(), items_c);
                                }
                                Err(e) => note.set(Some((format!("Duplicate failed: {e}"), NoticeKind::Error))),
                            }
                        }))
                        .child(ctx_button("Open-With", {
                            let pc = path_c.clone();
                            move || open_with_chooser(&pc)
                        }))
                        .child(ctx_button("Show in folder", {
                            let pc = path_c.clone();
                            move || show_in_folder(&pc)
                        }))
                        .child(ctx_button("Rename", {
                            let m1 = item_for_menu.clone();
                            let nc = name_c.clone();
                            move || {
                                rename_in.set(nc.clone());
                                rename_t.set(Some(m1.clone()));
                                show_rn.set(true);
                            }
                        }))
                        .child(ctx_button("Delete", {
                            let m2 = item_for_menu.clone();
                            let mut ms = multi_state;
                            let mut anchor = anchor_state;
                            let mut pend = pending_delete;
                            let mut show = show_delete_confirm;
                            move || {
                                if !ms.read().contains(&m2.path) {
                                    ms.set(vec![m2.path.clone()]);
                                    anchor.set(Some(m2.path.clone()));
                                }
                                pend.set(ms.read().clone());
                                show.set(true);
                            }
                        }))
                };

                if is_grid {
                    let thumb: Element = if is_img && small_thumb_ok(item.size, idx) {
                        rect()
                            .width(Size::px(icon_px + 16.))
                            .height(Size::px(icon_px + 16.))
                            .corner_radius(8.)
                            .overflow(Overflow::Clip)
                            .background(t.bg_card)
                            .center()
                            .child(
                                ImageViewer::new(PathBuf::from(path.clone()))
                                    .width(Size::px(icon_px + 12.))
                                    .height(Size::px(icon_px + 12.)),
                            )
                            .into_element()
                    } else {
                        rect()
                            .width(Size::px(icon_px + 16.))
                            .height(Size::px(icon_px + 16.))
                            .center()
                            .child(icon(i_svg, icon_px, if is_folder { t.primary_accent } else { t.text_secondary }))
                            .into_element()
                    };
                    rect()
                        .vertical()
                        .width(Size::px(120.))
                        .height(Size::px(140.))
                        .padding(8.)
                        .spacing(6.)
                        .margin(6.)
                        .corner_radius(12.)
                        .background(bg)
                        .cursor(CursorIcon::Pointer)
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::Center)
                        .content(Content::Flex)
                        .on_pointer_enter({
                            let ds = drag_start;
                            let mut ms = multi_state;
                            let mut ss = sel_state;
                            let it = item_clone.clone();
                            move |_| {
                                if ds.read().is_some() {
                                    let mut cur = ms.read().clone();
                                    if !cur.contains(&it.path) {
                                        cur.push(it.path.clone());
                                        ms.set(cur);
                                    }
                                    ss.set(Some(it.clone()));
                                }
                            }
                        })
                        .on_press({
                            let mut lc = last_click;
                            let mut ps = path_state;
                            let ss = sel_state;
                            let ms = multi_state;
                            let anchor = anchor_state;
                            let ctrl = ctrl_state;
                            let shift = shift_state;
                            let ordered_c = ordered.clone();
                            let p = path.clone();
                            let it = item_clone.clone();
                            let is_f = is_folder;
                            move |_| {
                                let now = std::time::Instant::now();
                                let is_double = if let Some((ref lp, lt)) = *lc.read() {
                                    lp == &p && now.duration_since(lt).as_millis() < 450
                                } else {
                                    false
                                };
                                if is_double {
                                    lc.set(None);
                                    if is_f {
                                        ps.set(p.clone());
                                        load_dir(p.clone(), items_state);
                                    } else {
                                        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
                                    }
                                } else {
                                    lc.set(Some((p.clone(), now)));
                                    selection_handle_press(
                                        p.clone(),
                                        it.clone(),
                                        ms,
                                        anchor,
                                        ss,
                                        ordered_c.clone(),
                                        *ctrl.read(),
                                        *shift.read(),
                                    );
                                }
                            }
                        })
                        .on_secondary_down({
                            let mut ss = sel_state;
                            let mut ms = multi_state;
                            let mut anchor = anchor_state;
                            let it = item_clone.clone();
                            let m = ctx_menu.clone();
                            move |_| {
                                if !ms.read().contains(&it.path) {
                                    ms.set(vec![it.path.clone()]);
                                    anchor.set(Some(it.path.clone()));
                                }
                                ss.set(Some(it.clone()));
                                ContextMenu::open_from_down(m.clone());
                            }
                        })
                        .child(thumb)
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_primary)
                                .text_align(TextAlign::Center)
                                .text(item.name.clone()),
                        )
                        .maybe(is_cut, |el| el.child(status_chip("Cut", true, None)))
                        .into_element()
                } else {
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .padding(8.)
                        .corner_radius(8.)
                        .background(bg)
                        .cursor(CursorIcon::Pointer)
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .spacing(8.)
                        .on_pointer_enter({
                            let ds = drag_start;
                            let mut ms = multi_state;
                            let mut ss = sel_state;
                            let it = item_clone.clone();
                            move |_| {
                                if ds.read().is_some() {
                                    let mut cur = ms.read().clone();
                                    if !cur.contains(&it.path) {
                                        cur.push(it.path.clone());
                                        ms.set(cur);
                                    }
                                    ss.set(Some(it.clone()));
                                }
                            }
                        })
                        .on_press({
                            let mut lc = last_click;
                            let mut ps = path_state;
                            let ss = sel_state;
                            let ms = multi_state;
                            let anchor = anchor_state;
                            let ctrl = ctrl_state;
                            let shift = shift_state;
                            let ordered_c = ordered.clone();
                            let p = path.clone();
                            let it = item_clone.clone();
                            let is_f = is_folder;
                            move |_| {
                                let now = std::time::Instant::now();
                                let is_double = if let Some((ref lp, lt)) = *lc.read() {
                                    lp == &p && now.duration_since(lt).as_millis() < 450
                                } else {
                                    false
                                };
                                if is_double {
                                    lc.set(None);
                                    if is_f {
                                        ps.set(p.clone());
                                        load_dir(p.clone(), items_state);
                                    } else {
                                        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
                                    }
                                } else {
                                    lc.set(Some((p.clone(), now)));
                                    selection_handle_press(
                                        p.clone(),
                                        it.clone(),
                                        ms,
                                        anchor,
                                        ss,
                                        ordered_c.clone(),
                                        *ctrl.read(),
                                        *shift.read(),
                                    );
                                }
                            }
                        })
                        .on_secondary_down({
                            let mut ss = sel_state;
                            let mut ms = multi_state;
                            let mut anchor = anchor_state;
                            let it = item_clone.clone();
                            let m = ctx_menu.clone();
                            move |_| {
                                if !ms.read().contains(&it.path) {
                                    ms.set(vec![it.path.clone()]);
                                    anchor.set(Some(it.path.clone()));
                                }
                                ss.set(Some(it.clone()));
                                ContextMenu::open_from_down(m.clone());
                            }
                        })
                        .child(icon(i_svg, 20., if is_folder { t.primary_accent } else { t.text_secondary }))
                        .child(
                            rect()
                                .width(Size::fill())
                                .content(Content::Flex)
                                .child(label().font_size(13.).color(t.text_primary).text(item.name.clone())),
                        )
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_muted)
                                .width(Size::px(80.))
                                .text_align(TextAlign::End)
                                .text(format_size(item.size)),
                        )
                        .maybe(is_cut, |el| el.child(status_chip("Cut", true, None)))
                        .into_element()
                }
            })),
    );

    let loading_el: Element = rect()
        .width(Size::fill())
        .padding(24.)
        .center()
        .content(Content::Flex)
        .child(label().font_size(13.).color(t.text_muted).text("Loading…"))
        .into_element();
    let error_el: Element = match load_error.read().clone() {
        Some(e) => rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .width(Size::fill())
            .padding(12.)
            .spacing(8.)
            .child(notice_pill(format!("Load failed: {e}"), NoticeKind::Error))
            .child(ghost_button("Retry", {
                let cp = current_path;
                let items_c = items;
                move || {
                    load_dir(cp.read().clone(), items_c);
                }
            }))
            .into_element(),
        None => rect().into_element(),
    };
    let empty_el: Element = empty_state(FOLDER, "Empty folder", "This folder has no items").into_element();
    let content_view: Element = if *is_loading.read() {
        loading_el
    } else if load_error.read().is_some() {
        error_el
    } else if items_total == 0 {
        empty_el
    } else {
        items_view.into()
    };

    let status_row: Element = {
        let note = notice.read().clone();
        let job = *job_running.read();
        let jl = job_label.read().clone();
        let prog = job_progress.read().clone();
        let trashed_n = LAST_TRASHED.lock().map(|v| v.len()).unwrap_or(0);
        if job || note.is_some() || trashed_n > 0 {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .content(Content::Flex)
                .width(Size::fill())
                .padding((4., 12.))
                .maybe(job, |el| {
                    let txt = match prog.clone() {
                        Some(p) => format!("{} {}/{} {}", p.label, p.done, p.total, p.current),
                        None => jl.clone(),
                    };
                    el.child(notice_pill(txt, NoticeKind::Info)).child(ghost_button("Cancel", {
                        let mut c = job_cancel;
                        move || c.set(true)
                    }))
                })
                .maybe(note.is_some(), |el| {
                    let (msg, kind) = note.clone().unwrap();
                    el.child(notice_pill(msg, kind))
                })
                .maybe(trashed_n > 0, |el| {
                    el.child(notice_pill(format!("{trashed_n} in Trash"), NoticeKind::Info)).child(ghost_button("Undo", {
                        let items_c = items;
                        let cp = current_path;
                        let mut note_st = notice;
                        move || match restore_last_trashed() {
                            Ok(n) => {
                                note_st.set(Some((
                                    format!("Restored {n} item{}", if n == 1 { "" } else { "s" }),
                                    NoticeKind::Success,
                                )));
                                load_dir(cp.read().clone(), items_c);
                            }
                            Err(e) => note_st.set(Some((format!("Restore failed: {e}"), NoticeKind::Error))),
                        }
                    }))
                })
                .into_element()
        } else {
            rect().into_element()
        }
    };

    let drawer: Element = if let Some(sel) = selected_item.read().clone() {
        let is_folder = sel.ty == ItemType::Folder;
        let is_img_sel = !is_folder && is_image_ext(&sel.name);
        rect()
            .width(Size::px(280.))
            .height(Size::fill())
            .background(t.bg_surface)
            .padding(16.)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .main_align(Alignment::SpaceBetween)
                    .cross_align(Alignment::Center)
                    .margin((0., 0., 12., 0.))
                    .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("PROPERTIES"))
                    .child({
                        let mut sel_state = selected_item;
                        rect()
                            .cursor(CursorIcon::Pointer)
                            .padding((2., 6.))
                            .corner_radius(4.)
                            .background(t.bg_card)
                            .on_press(move |_| sel_state.set(None))
                            .child(label().font_size(11.).color(t.text_muted).text("✕"))
                    }),
            )
            .child({
                if is_img_sel {
                    let path_buf = PathBuf::from(sel.path.clone());
                    rect()
                        .width(Size::fill())
                        .height(Size::px(160.))
                        .cross_align(Alignment::Center)
                        .main_align(Alignment::Center)
                        .content(Content::Flex)
                        .margin((0., 0., 16., 0.))
                        .corner_radius(12.)
                        .background(t.bg_card)
                        .overflow(Overflow::Clip)
                        .child(ImageViewer::new(path_buf).width(Size::fill()).height(Size::fill()))
                        .into_element()
                } else {
                    rect()
                        .width(Size::fill())
                        .cross_align(Alignment::Center)
                        .content(Content::Flex)
                        .margin((0., 0., 16., 0.))
                        .child(icon(
                            file_icon_for(&sel.name, is_folder),
                            48.,
                            if is_folder { t.primary_accent } else { t.text_secondary },
                        ))
                        .into_element()
                }
            })
            .child(
                label()
                    .font_size(16.)
                    .font_weight(FontWeight::BOLD)
                    .color(t.text_primary)
                    .margin((0., 0., 4., 0.))
                    .text(sel.name.clone()),
            )
            .child(label().font_size(11.).color(t.text_muted).margin((0., 0., 12., 0.)).text(format_size(sel.size)))
            .child(
                rect()
                    .margin((0., 0., 12., 0.))
                    .child(label().font_size(10.).color(t.text_muted).text("PATH"))
                    .child(label().font_size(11.).color(t.text_primary).text(sel.path.clone())),
            )
            .child(
                rect()
                    .margin((0., 0., 12., 0.))
                    .child(field_label("Type"))
                    .child(label().font_size(11.).color(t.text_primary).text(file_type_label(&sel.name, is_folder))),
            )
            .child(
                rect()
                    .margin((0., 0., 12., 0.))
                    .child(field_label("Modified"))
                    .child(label().font_size(11.).color(t.text_primary).text(format_modified(sel.modified))),
            )
            .child(
                rect()
                    .margin((0., 0., 12., 0.))
                    .child(field_label("Permissions"))
                    .child(label().font_size(11.).color(t.text_primary).text(format_perms(&sel.path))),
            )
            .maybe(clipboard.read().clone().is_some_and(|(paths, is_cut)| is_cut && paths.contains(&sel.path)), |el| {
                el.child(status_chip("Cut", true, None))
            })
            .child({
                let s_path = sel.path.clone();
                let mut path_state = current_path;
                let items_state = items;
                rect()
                    .margin((12., 0., 0., 0.))
                    .child(primary_button(if is_folder { "Open Directory" } else { "Open File" }, move || {
                        if is_folder {
                            path_state.set(s_path.clone());
                            load_dir(s_path.clone(), items_state);
                        } else {
                            let _ = std::process::Command::new("xdg-open").arg(&s_path).spawn();
                        }
                    }))
                    .into_element()
            })
            .child({
                if is_folder {
                    let mut pin_state = pinned;
                    let s_name = sel.name.clone();
                    let s_path = sel.path.clone();
                    let is_pinned = pin_state.read().iter().any(|(_, p)| p == &s_path);
                    rect()
                        .margin((8., 0., 0., 0.))
                        .child(secondary_button(
                            if is_pinned { "Unpin from Sidebar" } else { "Pin to Sidebar" },
                            move || {
                                let mut current = pin_state.read().clone();
                                if is_pinned {
                                    current.retain(|(_, p)| p != &s_path);
                                } else {
                                    current.push((s_name.clone(), s_path.clone()));
                                }
                                save_pinned(&current);
                                pin_state.set(current);
                            },
                        ))
                        .into_element()
                } else {
                    rect().into_element()
                }
            })
            .child({
                let mut clip = clipboard;
                let multi = selected_paths.read().clone();
                let s_path = sel.path.clone();
                rect()
                    .horizontal()
                    .spacing(8.)
                    .margin((8., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(secondary_button_full("Cut", {
                        let p = s_path.clone();
                        let m = multi.clone();
                        move || {
                            let mut v = m.clone();
                            if !v.contains(&p) {
                                v.push(p.clone());
                            }
                            if v.is_empty() {
                                v = vec![p.clone()];
                            }
                            clip.set(Some((v, true)))
                        }
                    })))
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(secondary_button_full("Copy", {
                        let p = s_path.clone();
                        let m = multi.clone();
                        move || {
                            let mut v = m.clone();
                            if !v.contains(&p) {
                                v.push(p.clone());
                            }
                            if v.is_empty() {
                                v = vec![p.clone()];
                            }
                            clip.set(Some((v, false)))
                        }
                    })))
                    .into_element()
            })
            .child({
                let s_path = sel.path.clone();
                let s_path2 = sel.path.clone();
                rect()
                    .horizontal()
                    .spacing(8.)
                    .margin((8., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .content(Content::Flex)
                            .child(secondary_button_full("Open-With", move || open_with_chooser(&s_path))),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .content(Content::Flex)
                            .child(secondary_button_full("Show in folder", move || show_in_folder(&s_path2))),
                    )
                    .into_element()
            })
            .child({
                let mut note = notice;
                let s_path = sel.path.clone();
                rect()
                    .margin((8., 0., 0., 0.))
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(secondary_button_full("Duplicate", move || match duplicate_one(&s_path) {
                        Ok(_) => {
                            note.set(Some(("Duplicated".to_string(), NoticeKind::Success)));
                            load_dir(current_path.read().clone(), items);
                        }
                        Err(e) => note.set(Some((format!("Duplicate failed: {e}"), NoticeKind::Error))),
                    }))
                    .into_element()
            })
            .child({
                let s_path = sel.path.clone();
                let mut rt = rename_target;
                let mut ri = rename_input;
                let mut sr = show_rename;
                let name_c = sel.name.clone();
                rect()
                    .horizontal()
                    .spacing(8.)
                    .margin((8., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(secondary_button("Rename", move || {
                                ri.set(name_c.clone());
                                rt.set(Some(sel.clone()));
                                sr.set(true);
                            }))
                            .content(Content::Flex),
                    )
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(danger_button("Delete", {
                        let mut pend = pending_delete;
                        let mut show = show_delete_confirm;
                        move || {
                            let mut paths = selected_paths.read().clone();
                            if !paths.contains(&s_path) {
                                paths.push(s_path.clone());
                            }
                            pend.set(paths);
                            show.set(true);
                        }
                    })))
                    .into_element()
            })
            .child({
                let clip_val = clipboard.read().clone();
                if clip_val.is_some() {
                    let dest = current_path.read().clone();
                    let items_state = items;
                    rect()
                        .margin((8., 0., 0., 0.))
                        .width(Size::fill())
                        .content(Content::Flex)
                        .child(secondary_button_full("Paste Here", move || {
                            do_paste_request_multi(
                                clip_val.clone(),
                                dest.clone(),
                                items_state,
                                notice,
                                job_running,
                                job_label,
                                job_progress,
                                job_cancel,
                                clipboard,
                                pending_paste,
                                show_overwrite,
                            );
                        }))
                        .into_element()
                } else {
                    rect().into_element()
                }
            })
            .into_element()
    } else {
        rect().width(Size::px(0.)).into_element()
    };

    let add_place_modal: Element = if *show_add_place.read() {
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(360.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.bg_surface)
                    .spacing(12.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Add Place"))
                    .child(Input::new(add_place_name).width(Size::fill()).placeholder("Name"))
                    .child(label().font_size(11.).color(t.text_muted).text(current_path.read().clone()))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .on_press({
                                        let mut s = show_add_place;
                                        move |_| s.set(false)
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Cancel")),
                            )
                            .child(
                                rect()
                                    .padding((8., 12.))
                                    .corner_radius(8.)
                                    .background(t.primary_accent)
                                    .on_press({
                                        let mut s = show_add_place;
                                        let mut pin = pinned;
                                        let name_s = add_place_name;
                                        let path_s = current_path;
                                        move |_| {
                                            let n = name_s.read().trim().to_string();
                                            if !n.is_empty() {
                                                let mut v = pin.read().clone();
                                                v.push((n, path_s.read().clone()));
                                                save_pinned(&v);
                                                pin.set(v);
                                            }
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.bg_base).text("Add")),
                            ),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    let rename_modal: Element = if *show_rename.read() {
        let _target = rename_target.read().clone();
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(360.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.bg_surface)
                    .spacing(12.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Rename"))
                    .child(Input::new(rename_input).width(Size::fill()).placeholder("New name").on_submit({
                        let mut show = show_rename;
                        let tgt = rename_target;
                        let inp = rename_input;
                        let cp = current_path;
                        let items_s = items;
                        let mut sel = selected_item;
                        move |val: String| {
                            if let Some(it) = tgt.read().clone() {
                                let mut note = notice;
                                let name = val.trim().to_string();
                                if name.is_empty() {
                                    note.set(Some(("Name cannot be empty".to_string(), NoticeKind::Error)));
                                    return;
                                }
                                let new_path =
                                    PathBuf::from(&it.path).parent().unwrap_or(PathBuf::from("/").as_path()).join(&name);
                                if new_path.exists() {
                                    note.set(Some(("A file with that name exists".to_string(), NoticeKind::Error)));
                                    return;
                                }
                                match std::fs::rename(&it.path, &new_path) {
                                    Ok(()) => {
                                        sel.set(None);
                                        load_dir(cp.read().clone(), items_s);
                                    }
                                    Err(e) => note.set(Some((format!("Rename failed: {e}"), NoticeKind::Error))),
                                }
                            }
                            let _ = inp;
                            show.set(false);
                        }
                    }))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .on_press({
                                        let mut s = show_rename;
                                        move |_| s.set(false)
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Cancel")),
                            )
                            .child(
                                rect()
                                    .padding((8., 12.))
                                    .corner_radius(8.)
                                    .background(t.primary_accent)
                                    .on_press({
                                        let mut s = show_rename;
                                        let tgt = rename_target;
                                        let inp = rename_input;
                                        let cp = current_path;
                                        let items_s = items;
                                        let mut sel = selected_item;
                                        move |_| {
                                            if let Some(it) = tgt.read().clone() {
                                                let mut note = notice;
                                                let new_name = inp.read().trim().to_string();
                                                if new_name.is_empty() {
                                                    note.set(Some(("Name cannot be empty".to_string(), NoticeKind::Error)));
                                                    return;
                                                }
                                                let new_path = PathBuf::from(&it.path)
                                                    .parent()
                                                    .unwrap_or(PathBuf::from("/").as_path())
                                                    .join(&new_name);
                                                if new_path.exists() {
                                                    note.set(Some((
                                                        "A file with that name exists".to_string(),
                                                        NoticeKind::Error,
                                                    )));
                                                    return;
                                                }
                                                match std::fs::rename(&it.path, &new_path) {
                                                    Ok(()) => {
                                                        sel.set(None);
                                                        load_dir(cp.read().clone(), items_s);
                                                    }
                                                    Err(e) => {
                                                        note.set(Some((format!("Rename failed: {e}"), NoticeKind::Error)))
                                                    }
                                                }
                                            }
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.bg_base).text("Rename")),
                            ),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    let delete_modal: Element = if *show_delete_confirm.read() {
        let n = pending_delete.read().len();
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(380.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.bg_surface)
                    .spacing(12.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Move to Trash"))
                    .child(label().font_size(12.).color(t.text_muted).text(format!(
                        "Move {n} item{} to Trash? You can undo from the status bar.",
                        if n == 1 { "" } else { "s" }
                    )))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press({
                                        let mut s = show_delete_confirm;
                                        let mut p = pending_delete;
                                        move |_| {
                                            p.set(Vec::new());
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Cancel")),
                            )
                            .child(
                                rect()
                                    .padding((8., 12.))
                                    .corner_radius(8.)
                                    .background(t.primary_accent)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press({
                                        let mut s = show_delete_confirm;
                                        let mut p = pending_delete;
                                        let mut sel = selected_item;
                                        let mut multi = selected_paths;
                                        let cp = current_path;
                                        let items_c = items;
                                        move |_| {
                                            confirm_trash_delete_multi(
                                                p.read().clone(),
                                                items_c,
                                                notice,
                                                job_running,
                                                job_label,
                                                job_progress,
                                                job_cancel,
                                                cp.read().clone(),
                                            );
                                            p.set(Vec::new());
                                            sel.set(None);
                                            multi.set(Vec::new());
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.bg_base).text("Move to Trash")),
                            ),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    let overwrite_modal: Element = if *show_overwrite.read() {
        let (count, _is_cut) = pending_paste.read().clone().map(|p| (p.srcs.len(), p.is_cut)).unwrap_or((0, false));
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::px(380.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.bg_surface)
                    .spacing(12.)
                    .child(
                        label()
                            .font_size(16.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_primary)
                            .text("Items already exist"),
                    )
                    .child(label().font_size(12.).color(t.text_muted).text(format!(
                        "{count} item{} already exist at the destination. Overwrite them?",
                        if count == 1 { "" } else { "s" }
                    )))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press({
                                        let mut s = show_overwrite;
                                        let mut p = pending_paste;
                                        move |_| {
                                            p.set(None);
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Cancel")),
                            )
                            .child(
                                rect()
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press({
                                        let mut s = show_overwrite;
                                        let mut p = pending_paste;
                                        let mut note = notice;
                                        let items_c = items;
                                        move |_| {
                                            if let Some(pv) = p.read().clone() {
                                                let missing: Vec<String> = pv
                                                    .srcs
                                                    .iter()
                                                    .filter(|x| {
                                                        let name = PathBuf::from(x)
                                                            .file_name()
                                                            .map(|y| y.to_string_lossy().to_string())
                                                            .unwrap_or_default();
                                                        !PathBuf::from(&pv.dest_dir).join(&name).exists()
                                                    })
                                                    .cloned()
                                                    .collect();
                                                if missing.is_empty() {
                                                    note.set(Some((
                                                        "All items already exist, nothing pasted".to_string(),
                                                        NoticeKind::Warn,
                                                    )));
                                                } else {
                                                    start_paste_job_multi(
                                                        missing,
                                                        false,
                                                        pv.is_cut,
                                                        pv.dest_dir,
                                                        items_c,
                                                        notice,
                                                        job_running,
                                                        job_label,
                                                        job_progress,
                                                        job_cancel,
                                                        clipboard,
                                                    );
                                                }
                                            }
                                            p.set(None);
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.text_primary).text("Skip existing")),
                            )
                            .child(
                                rect()
                                    .padding((8., 12.))
                                    .corner_radius(8.)
                                    .background(t.primary_accent)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press({
                                        let mut s = show_overwrite;
                                        let mut p = pending_paste;
                                        let items_c = items;
                                        move |_| {
                                            if let Some(pv) = p.read().clone() {
                                                start_paste_job_multi(
                                                    pv.srcs,
                                                    true,
                                                    pv.is_cut,
                                                    pv.dest_dir,
                                                    items_c,
                                                    notice,
                                                    job_running,
                                                    job_label,
                                                    job_progress,
                                                    job_cancel,
                                                    clipboard,
                                                );
                                            }
                                            p.set(None);
                                            s.set(false);
                                        }
                                    })
                                    .child(label().font_size(12.).color(t.bg_base).text("Overwrite")),
                            ),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(t.bg_base)
        .on_global_key_down({
            let mut ctrl = ctrl_held;
            let mut shift = shift_held;
            let mut clip = clipboard;
            let mut sel = selected_item;
            let mut multi = selected_paths;
            let cp2 = current_path;
            let items2 = items;
            let mut rename_t = rename_target;
            let mut rename_in = rename_input;
            let mut show_rn = show_rename;
            let mut search_q = search_query;
            move |e: Event<KeyboardEventData>| {
                if e.data().key == Key::Named(NamedKey::Control) {
                    ctrl.set(true);
                    return;
                }
                if e.data().key == Key::Named(NamedKey::Shift) {
                    shift.set(true);
                    return;
                }
                if e.data().key == Key::Named(NamedKey::Escape) {
                    if !search_q.read().is_empty() {
                        search_q.set(String::new());
                        load_dir(cp2.read().clone(), items2);
                    }
                    sel.set(None);
                    multi.set(Vec::new());
                    return;
                }
                if e.data().modifiers.ctrl() {
                    match e.data().key {
                        Key::Character(ref s) if s == "c" => {
                            let mut v = multi.read().clone();
                            if v.is_empty() {
                                if let Some(it) = sel.read().clone() {
                                    v = vec![it.path];
                                }
                            }
                            if !v.is_empty() {
                                clip.set(Some((v, false)));
                            }
                        }
                        Key::Character(ref s) if s == "x" => {
                            let mut v = multi.read().clone();
                            if v.is_empty() {
                                if let Some(it) = sel.read().clone() {
                                    v = vec![it.path];
                                }
                            }
                            if !v.is_empty() {
                                clip.set(Some((v, true)));
                            }
                        }
                        Key::Character(ref s) if s == "v" => {
                            do_paste_request_multi(
                                clip.read().clone(),
                                cp2.read().clone(),
                                items2,
                                notice,
                                job_running,
                                job_label,
                                job_progress,
                                job_cancel,
                                clipboard,
                                pending_paste,
                                show_overwrite,
                            );
                        }
                        Key::Character(ref s) if s == "a" => {
                            let all: Vec<String> =
                                visible_items(&items2.read(), *sort_key.read(), *dirs_first.read(), *show_hidden.read())
                                    .into_iter()
                                    .map(|i| i.path)
                                    .collect();
                            sel.set(None);
                            multi.set(all);
                        }
                        _ => {}
                    }
                } else {
                    match &e.data().key {
                        Key::Named(NamedKey::F2) => {
                            if let Some(it) = sel.read().clone() {
                                rename_in.set(it.name.clone());
                                rename_t.set(Some(it));
                                show_rn.set(true);
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Some(it) = sel.read().clone() {
                                if it.ty == ItemType::Folder {
                                    let mut cur = cp2;
                                    cur.set(it.path.clone());
                                    load_dir(it.path, items2);
                                } else {
                                    let _ = std::process::Command::new("xdg-open").arg(&it.path).spawn();
                                }
                            }
                        }
                        Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::ArrowRight) => {
                            let list =
                                visible_items(&items2.read(), *sort_key.read(), *dirs_first.read(), *show_hidden.read());
                            if !list.is_empty() {
                                let cur = sel.read().clone().map(|s| s.path);
                                let pos =
                                    cur.and_then(|p| list.iter().position(|i| i.path == p)).map(|i| i + 1).unwrap_or(0);
                                let next = list[pos.min(list.len() - 1)].clone();
                                multi.set(Vec::new());
                                anchor_path.set(Some(next.path.clone()));
                                sel.set(Some(next));
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::ArrowLeft) => {
                            let list =
                                visible_items(&items2.read(), *sort_key.read(), *dirs_first.read(), *show_hidden.read());
                            if !list.is_empty() {
                                let cur = sel.read().clone().map(|s| s.path);
                                let pos = cur
                                    .and_then(|p| list.iter().position(|i| i.path == p))
                                    .map(|i| i.saturating_sub(1))
                                    .unwrap_or(0);
                                let prev = list[pos].clone();
                                multi.set(Vec::new());
                                anchor_path.set(Some(prev.path.clone()));
                                sel.set(Some(prev));
                            }
                        }
                        Key::Named(NamedKey::Delete) => {
                            let mut paths = multi.read().clone();
                            if paths.is_empty() {
                                if let Some(it) = sel.read().clone() {
                                    paths = vec![it.path];
                                }
                            }
                            if !paths.is_empty() {
                                confirm_trash_delete_multi(
                                    paths,
                                    items2,
                                    notice,
                                    job_running,
                                    job_label,
                                    job_progress,
                                    job_cancel,
                                    cp2.read().clone(),
                                );
                                sel.set(None);
                                multi.set(Vec::new());
                            }
                        }
                        _ => {}
                    }
                }
            }
        })
        .on_global_key_up({
            let mut ctrl = ctrl_held;
            let mut shift = shift_held;
            move |e: Event<KeyboardEventData>| {
                if e.data().key == Key::Named(NamedKey::Control) {
                    ctrl.set(false);
                }
                if e.data().key == Key::Named(NamedKey::Shift) {
                    shift.set(false);
                }
            }
        })
        .child(ContextMenuViewer::new())
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::fill())
                .content(Content::Flex)
                .background(t.bg_base)
                .maybe(*show_sidebar.read(), |el| el.child(sidebar))
                .child(
                    rect()
                        .vertical()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .content(Content::Flex)
                        .child(top_bar)
                        .child(crumb_bar)
                        .child(tool_bar)
                        .child(trash_bar)
                        .child(status_row)
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::fill())
                                .content(Content::Flex)
                                .on_secondary_down({
                                    let cp_c = current_path.read().clone();
                                    let clip_v = clipboard.read().clone();
                                    let hidden_v = *show_hidden.read();
                                    let grid_v = *view_mode.read() == ViewMode::Grid;
                                    move |_| {
                                        let dest = cp_c.clone();
                                        let cv = clip_v.clone();
                                        ContextMenu::open_from_down(
                                            Menu::new()
                                                .child(ctx_button("New Folder", {
                                                    let d = dest.clone();
                                                    move || {
                                                        let target = PathBuf::from(&d).join("New Folder");
                                                        let mut u = target.clone();
                                                        let mut n = 1;
                                                        while u.exists() {
                                                            u = PathBuf::from(&d).join(format!("New Folder ({n})"));
                                                            n += 1;
                                                        }
                                                        let _ = std::fs::create_dir_all(&u);
                                                        load_dir(d.clone(), items);
                                                    }
                                                }))
                                                .child(ctx_button("New File", {
                                                    let d = dest.clone();
                                                    let mut note = notice;
                                                    move || match create_new_file_in(&d) {
                                                        Ok(_) => load_dir(d.clone(), items),
                                                        Err(e) => note
                                                            .set(Some((format!("New file failed: {e}"), NoticeKind::Error))),
                                                    }
                                                }))
                                                .child(ctx_button("Paste", {
                                                    let cv2 = cv.clone();
                                                    let dest2 = dest.clone();
                                                    move || {
                                                        do_paste_request_multi(
                                                            cv2.clone(),
                                                            dest2.clone(),
                                                            items,
                                                            notice,
                                                            job_running,
                                                            job_label,
                                                            job_progress,
                                                            job_cancel,
                                                            clipboard,
                                                            pending_paste,
                                                            show_overwrite,
                                                        );
                                                    }
                                                }))
                                                .child(ctx_button("Refresh", {
                                                    let d = dest.clone();
                                                    move || load_dir(d.clone(), items)
                                                }))
                                                .child(ctx_button(if grid_v { "View: List" } else { "View: Grid" }, {
                                                    let mut vm = view_mode;
                                                    move || vm.set(if grid_v { ViewMode::List } else { ViewMode::Grid })
                                                }))
                                                .child(ctx_button(
                                                    if hidden_v { "Hide hidden files" } else { "Show hidden files" },
                                                    {
                                                        let mut h = show_hidden;
                                                        move || h.set(!hidden_v)
                                                    },
                                                )),
                                        );
                                    }
                                })
                                .child(content_view),
                        )
                        .child(bottom_bar),
                )
                .maybe_child(selected_item.read().is_some().then(|| drawer)),
        )
        .child(add_place_modal)
        .child(rename_modal)
        .child(delete_modal)
        .child(overwrite_modal)
        .into_element()
}

fn sidebar_entry(
    name: &str,
    icon_svg: &'static str,
    current_path: &State<String>,
    target_path: String,
    items_state: State<Vec<Item>>,
    pinned: State<Vec<(String, String)>>,
    _clipboard: State<Option<(Vec<String>, bool)>>,
) -> Element {
    let t = use_app_theme();
    let mut cp = *current_path;
    let tp = target_path.clone();
    let is_active = *cp.read() == tp;
    let bg = if is_active { t.bg_active } else { Color::TRANSPARENT };
    let name_owned = name.to_string();
    let tp_for_menu = tp.clone();
    let ctx = Menu::new()
        .child(
            MenuButton::new()
                .on_press({
                    let tp2 = tp_for_menu.clone();
                    let mut cp2 = cp;
                    move |_| {
                        cp2.set(tp2.clone());
                        load_dir(tp2.clone(), items_state);
                    }
                })
                .child("Open"),
        )
        .child(
            MenuButton::new()
                .on_press({
                    let tp2 = tp_for_menu.clone();
                    let name2 = name_owned.clone();
                    let mut pin = pinned;
                    move |_| {
                        let mut v = pin.read().clone();
                        if v.iter().any(|(_, p)| p == &tp2) {
                            v.retain(|(_, p)| p != &tp2);
                        } else {
                            v.push((name2.clone(), tp2.clone()));
                        }
                        save_pinned(&v);
                        pin.set(v);
                    }
                })
                .child(if pinned.read().iter().any(|(_, p)| p == &tp_for_menu) { "Unpin" } else { "Pin" }),
        );
    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .spacing(8.)
        .padding(8.)
        .corner_radius(8.)
        .background(bg)
        .margin((0., 0., 4., 0.))
        .on_press(move |_| {
            cp.set(tp.clone());
            load_dir(tp.clone(), items_state);
        })
        .on_secondary_down(move |_| ContextMenu::open_from_down(ctx.clone()))
        .child(icon(icon_svg, 14., t.text_primary))
        .child(
            label()
                .font_size(13.)
                .color(t.text_primary)
                .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .text(name_owned.clone()),
        )
        .into_element()
}

fn main() { launch(LaunchConfig::new().with_window(WindowConfig::new(app).with_title("Files").with_size(1100., 700.))) }
