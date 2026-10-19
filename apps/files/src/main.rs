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
    speed: String,
}

#[derive(Clone, PartialEq, Debug)]
struct TrashEntry {
    trash_name: String,
    orig_path: String,
    date: String,
}

#[derive(Clone, PartialEq, Debug)]
struct OpUndo {
    kind: String,
    pairs: Vec<(String, String)>,
    label: String,
}

static LAST_OP: std::sync::Mutex<Option<OpUndo>> = std::sync::Mutex::new(None);
static WATCH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn format_speed(bytes_done: u64, elapsed_secs: f64) -> String {
    if elapsed_secs < 0.2 || bytes_done == 0 {
        return String::new();
    }
    let bps = bytes_done as f64 / elapsed_secs;
    if bps < 1024.0 {
        format!("{bps:.0} B/s")
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.1} MB/s", bps / 1024.0 / 1024.0)
    }
}

fn trash_undo_file() -> PathBuf {
    let base = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
    base.join(".cache").join("finick").join("files_trash_undo.json")
}

fn load_trash_undo_from(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<(String, String)>>(&s).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|(u, o)| !u.is_empty() && !o.is_empty())
        .take(50)
        .collect()
}

fn save_trash_undo_to(path: &Path, pairs: &[(String, String)]) -> bool {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    serde_json::to_string(&pairs).map(|s| std::fs::write(path, s).is_ok()).unwrap_or(false)
}

fn remember_trashed(pairs: Vec<(String, String)>) {
    if pairs.is_empty() {
        return;
    }
    let _ = LAST_TRASHED.lock().map(|mut v| {
        v.extend(pairs.clone());
        if v.len() > 50 {
            let drop_n = v.len() - 50;
            v.drain(..drop_n);
        }
    });
    let path = trash_undo_file();
    let mut merged = load_trash_undo_from(&path);
    merged.extend(pairs);
    if merged.len() > 50 {
        let drop_n = merged.len() - 50;
        merged.drain(..drop_n);
    }
    save_trash_undo_to(&path, &merged);
}

fn take_trash_undo() -> Vec<(String, String)> {
    let mem: Vec<(String, String)> = LAST_TRASHED.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
    let path = trash_undo_file();
    let file_pairs = load_trash_undo_from(&path);
    let _ = std::fs::remove_file(&path);
    if mem.is_empty() {
        return file_pairs;
    }
    if file_pairs.is_empty() {
        return mem;
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in mem.into_iter().chain(file_pairs) {
        if seen.insert(p.clone()) {
            out.push(p);
        }
    }
    out.truncate(50);
    out
}

fn pending_trash_undo_count() -> usize {
    let mem = LAST_TRASHED.lock().map(|v| v.len()).unwrap_or(0);
    if mem > 0 {
        return mem;
    }
    load_trash_undo_from(&trash_undo_file()).len()
}

fn internal_drop_error(srcs: &[String], dest_dir: &str) -> Option<String> {
    if srcs.is_empty() {
        return Some(String::from("Nothing selected to move"));
    }
    let dest = PathBuf::from(dest_dir);
    if !dest.is_dir() {
        return Some(format!("Destination is not a folder: {dest_dir}"));
    }
    let dest_canon = dest.canonicalize().unwrap_or(dest.clone());
    for src in srcs {
        let sp = PathBuf::from(src);
        if !sp.exists() {
            return Some(format!("Source no longer exists: {src}"));
        }
        let name = sp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.is_empty() {
            return Some(format!("Cannot move: {src}"));
        }
        if dest.join(&name) == sp {
            return Some(String::from("Already here"));
        }
        if let Ok(canon) = sp.canonicalize() {
            if canon == dest_canon {
                return Some(String::from("Already here"));
            }
            if dest_canon.starts_with(&canon) {
                return Some(String::from("Cannot move a folder into itself"));
            }
            if dest_canon.join(&name) == canon {
                return Some(String::from("Already here"));
            }
        } else if sp == dest {
            return Some(String::from("Already here"));
        }
    }
    None
}

fn dir_byte_size(path: &Path) -> u64 {
    if let Ok(md) = std::fs::metadata(path) {
        if !md.is_dir() {
            return md.len();
        }
    } else {
        return 0;
    }
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    let mut seen = 0usize;
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d).map(|r| r.filter_map(|e| e.ok()).collect::<Vec<_>>()).unwrap_or_default();
        for e in entries {
            seen += 1;
            if seen > 20000 {
                return total;
            }
            if let Ok(ty) = e.file_type() {
                if ty.is_dir() {
                    stack.push(e.path());
                } else if let Ok(md) = e.metadata() {
                    total = total.saturating_add(md.len());
                }
            }
        }
    }
    total
}

fn parse_trashinfo(path: &Path) -> Option<(String, String)> {
    let s = std::fs::read_to_string(path).ok()?;
    let mut orig = None;
    let mut date = String::new();
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("Path=") {
            orig = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("DeletionDate=") {
            date = v.trim().to_string();
        }
    }
    orig.map(|o| (o, date))
}

fn percent_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h: String = chars.by_ref().take(2).collect();
            if h.len() == 2 {
                if let Ok(b) = u8::from_str_radix(&h, 16) {
                    out.push(b as char);
                    continue;
                }
            }
            out.push('%');
            out.push_str(&h);
        } else {
            out.push(c);
        }
    }
    out
}

fn list_trash_entries() -> Vec<TrashEntry> {
    let (files, info) = trash_dirs();
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&files).map(|r| r.filter_map(|e| e.ok()).collect::<Vec<_>>()).unwrap_or_default();
    for e in entries {
        let name = e.file_name().to_string_lossy().to_string();
        let info_path = info.join(format!("{name}.trashinfo"));
        let (orig, date) =
            parse_trashinfo(&info_path).map(|(o, d)| (percent_decode(&o), d)).unwrap_or((String::new(), String::new()));
        out.push(TrashEntry { trash_name: name, orig_path: orig, date });
    }
    out.sort_by(|a, b| a.trash_name.cmp(&b.trash_name));
    out
}

fn trash_orig_map() -> std::collections::HashMap<String, String> {
    list_trash_entries().into_iter().map(|e| (e.trash_name, e.orig_path)).collect()
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
            Self::Small => 40.,
            Self::Medium => 56.,
            Self::Large => 72.,
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

fn small_thumb_ok(size: u64, idx: usize) -> bool { size > 0 && size <= THUMB_MAX_FILE_BYTES && idx < 60 }

fn path_exists(p: &str) -> bool { Path::new(p).exists() }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SearchScope {
    Current,
    All,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TypeFilter {
    All,
    Images,
    Docs,
    Code,
    Audio,
    Video,
    Archives,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DateFilter {
    Any,
    Today,
    Week,
    Month,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DrawerTab {
    Preview,
    Details,
}

#[derive(Clone, PartialEq, Debug)]
struct VolumeInfo {
    name: String,
    mount_path: String,
    device: String,
    total: u64,
    avail: u64,
    mounted: bool,
}

fn run_cmd(prog: &str, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new(prog).args(args).output().map_err(|e| format!("{prog} not available: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { format!("{prog} failed") } else { err })
    }
}

fn volume_usage_for(mount: &str) -> (u64, u64) {
    let out = std::process::Command::new("df").args(["-B1", mount]).output();
    if let Ok(out) = out {
        if out.status.success() {
            let txt = String::from_utf8_lossy(&out.stdout);
            if let Some(last) = txt.lines().last() {
                let parts: Vec<&str> = last.split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].parse::<u64>().unwrap_or(0);
                    let avail = parts[3].parse::<u64>().unwrap_or(0);
                    return (total, avail);
                }
            }
        }
    }
    (0, 0)
}

fn list_volumes() -> Vec<VolumeInfo> {
    let mut out: Vec<VolumeInfo> = Vec::new();
    if let Ok(txt) = run_cmd("lsblk", &["-J", "-o", "NAME,MOUNTPOINT,SIZE,FSTYPE,TYPE"]) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            let mut stack: Vec<&serde_json::Value> = v.get("blockdevices").and_then(|b| b.as_array()).map(|a| a.iter().collect()).unwrap_or_default();
            while let Some(dev) = stack.pop() {
                let name = dev.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let mp = dev.get("mountpoint").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let ty = dev.get("type").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if let Some(children) = dev.get("children").and_then(|c| c.as_array()) {
                    for c in children {
                        stack.push(c);
                    }
                }
                if ty != "part" && ty != "rom" && ty != "disk" {
                    continue;
                }
                if name.is_empty() {
                    continue;
                }
                let device = format!("/dev/{name}");
                if mp.is_empty() {
                    out.push(VolumeInfo { name: name.clone(), mount_path: String::new(), device, total: 0, avail: 0, mounted: false });
                } else if mp == "/" || mp == "/boot" || mp == "[SWAP]" || mp.starts_with("/snap") {
                    continue;
                } else {
                    let (total, avail) = volume_usage_for(&mp);
                    let label = PathBuf::from(&mp).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(mp.clone());
                    out.push(VolumeInfo { name: label, mount_path: mp, device, total, avail, mounted: true });
                }
            }
        }
    }
    if out.is_empty() {
        for (name, path) in list_devices() {
            let (total, avail) = volume_usage_for(&path);
            out.push(VolumeInfo { name, mount_path: path.clone(), device: path, total, avail, mounted: true });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.mount_path == b.mount_path && a.device == b.device);
    out
}

fn mount_volume(v: &VolumeInfo) -> Result<String, String> {
    if v.mounted && !v.mount_path.is_empty() {
        return Ok(v.mount_path.clone());
    }
    if v.device.starts_with("/dev/") {
        match run_cmd("udisksctl", &["mount", "-b", &v.device]) {
            Ok(msg) => {
                for tok in msg.split_whitespace() {
                    if tok.starts_with('/') && Path::new(tok).is_dir() {
                        return Ok(tok.trim_matches('.').to_string());
                    }
                }
                Ok(msg)
            }
            Err(e) => {
                let _ = run_cmd("gio", &["mount", &v.device]);
                Err(e)
            }
        }
    } else {
        run_cmd("gio", &["mount", &v.device])
    }
}

fn unmount_volume(v: &VolumeInfo) -> Result<String, String> {
    if !v.mount_path.is_empty() {
        if let Ok(msg) = run_cmd("udisksctl", &["unmount", "-p", &v.mount_path]) {
            return Ok(msg);
        }
        if let Ok(msg) = run_cmd("gio", &["mount", "-u", &v.mount_path]) {
            return Ok(msg);
        }
        return run_cmd("umount", &[&v.mount_path]);
    }
    run_cmd("udisksctl", &["unmount", "-b", &v.device])
}

fn eject_volume(v: &VolumeInfo) -> Result<String, String> {
    let _ = unmount_volume(v);
    if v.device.starts_with("/dev/") {
        run_cmd("udisksctl", &["power-off", "-b", &v.device])
    } else {
        Ok("Unmounted".to_string())
    }
}

fn is_network_uri(s: &str) -> bool {
    let l = s.trim().to_lowercase();
    ["sftp://", "smb://", "ftp://", "ftps://", "dav://", "davs://", "webdav://", "nfs://", "ssh://"]
        .iter()
        .any(|p| l.starts_with(p))
}

fn open_network_uri(uri: &str, mut notice: State<Option<(String, NoticeKind)>>) {
    let u = uri.trim().to_string();
    if u.is_empty() {
        notice.set(Some(("Enter a network address like sftp://host/share".to_string(), NoticeKind::Warn)));
        return;
    }
    if !is_network_uri(&u) {
        notice.set(Some(("Not a network address. Use sftp:// or smb://".to_string(), NoticeKind::Error)));
        return;
    }
    match run_cmd("gio", &["open", &u]) {
        Ok(_) => notice.set(Some((format!("Opened {u}"), NoticeKind::Success))),
        Err(e1) => match run_cmd("xdg-open", &[&u]) {
            Ok(_) => notice.set(Some((format!("Opened {u}"), NoticeKind::Success))),
            Err(e2) => notice.set(Some((
                format!("Cannot open {u}: {e1}; {e2}. Install gvfs for sftp/smb browsing."),
                NoticeKind::Error,
            ))),
        },
    }
}

fn starred_path() -> PathBuf {
    config::finick_root().join("starred.json")
}

fn load_starred() -> Vec<(String, String)> {
    if let Ok(s) = std::fs::read_to_string(starred_path()) {
        if let Ok(v) = serde_json::from_str::<Vec<(String, String)>>(&s) {
            return v.into_iter().filter(|(_, p)| Path::new(p).exists() || is_network_uri(p)).take(50).collect();
        }
    }
    Vec::new()
}

fn save_starred(v: &[(String, String)]) {
    let p = starred_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(v) {
        let _ = std::fs::write(p, s);
    }
}

fn recent_files_path() -> PathBuf {
    let base = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
    base.join(".cache").join("finick").join("files_recent_files.json")
}

fn load_recent_files() -> Vec<String> {
    if let Ok(s) = std::fs::read_to_string(recent_files_path()) {
        if let Ok(v) = serde_json::from_str::<Vec<String>>(&s) {
            return v.into_iter().filter(|p| Path::new(p).is_file()).take(20).collect();
        }
    }
    Vec::new()
}

fn record_recent_file(path: &str) {
    if !Path::new(path).is_file() {
        return;
    }
    let mut v = load_recent_files();
    v.retain(|d| d != path);
    v.insert(0, path.to_string());
    v.truncate(20);
    let p = recent_files_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(&v) {
        let _ = std::fs::write(p, s);
    }
}

fn type_label_for_filter(name: &str, is_dir: bool) -> TypeFilter {
    if is_dir {
        return TypeFilter::All;
    }
    let l = name.to_lowercase();
    let ext = l.rsplit('.').next().unwrap_or("");
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tiff" | "heic" => TypeFilter::Images,
        "txt" | "md" | "log" | "csv" | "pdf" | "doc" | "docx" | "odt" => TypeFilter::Docs,
        "rs" | "ts" | "js" | "py" | "go" | "c" | "cpp" | "h" | "java" | "sh" | "toml" | "json" | "yaml" | "yml" | "html" | "css" => TypeFilter::Code,
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" => TypeFilter::Audio,
        "mp4" | "mkv" | "avi" | "mov" | "webm" => TypeFilter::Video,
        "zip" | "tar" | "gz" | "xz" | "7z" | "rar" | "bz2" | "zst" | "tgz" => TypeFilter::Archives,
        _ => TypeFilter::All,
    }
}

fn matches_type_filter(item: &Item, f: TypeFilter) -> bool {
    if f == TypeFilter::All {
        return true;
    }
    if item.ty == ItemType::Folder {
        return true;
    }
    type_label_for_filter(&item.name, false) == f
}

fn matches_date_filter(item: &Item, f: DateFilter) -> bool {
    match f {
        DateFilter::Any => true,
        _ => {
            let ts = item.modified.unwrap_or(0) as i64;
            if ts == 0 {
                return false;
            }
            let now = chrono::Local::now().timestamp();
            let age = now - ts;
            match f {
                DateFilter::Today => age < 24 * 3600,
                DateFilter::Week => age < 7 * 24 * 3600,
                DateFilter::Month => age < 30 * 24 * 3600,
                DateFilter::Any => true,
            }
        }
    }
}

const THUMB_MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const THUMB_MAX_FILES: usize = 500;
const THUMB_MAX_BYTES: u64 = 200 * 1024 * 1024;
const TEXT_PREVIEW_MAX_BYTES: u64 = 16 * 1024;
const TEXT_PREVIEW_MAX_CHARS: usize = 2000;

fn thumb_cache_dir() -> PathBuf {
    let base = env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp")).join(".cache"));
    base.join("finick").join("files_thumbs")
}

fn thumb_key(path: &str, mtime: Option<u64>, size: u64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    mtime.unwrap_or(0).hash(&mut h);
    size.hash(&mut h);
    format!("{:x}", h.finish())
}

fn thumb_path_for(path: &str, mtime: Option<u64>, size: u64) -> PathBuf {
    thumb_cache_dir().join(format!("{}.png", thumb_key(path, mtime, size)))
}

fn cached_thumb_if_exists(path: &str, mtime: Option<u64>, size: u64) -> Option<PathBuf> {
    if size == 0 || size > THUMB_MAX_FILE_BYTES {
        return None;
    }
    let dest = thumb_path_for(path, mtime, size);
    if dest.is_file() { Some(dest) } else { None }
}

fn prune_thumb_cache() {
    let dir = thumb_cache_dir();
    let entries = std::fs::read_dir(&dir).map(|r| r.filter_map(|e| e.ok()).collect::<Vec<_>>()).unwrap_or_default();
    if entries.len() <= THUMB_MAX_FILES {
        let total: u64 = entries.iter().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum();
        if total <= THUMB_MAX_BYTES {
            return;
        }
    }
    let mut sized: Vec<(std::time::SystemTime, u64, PathBuf)> = entries
        .into_iter()
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            if !md.is_file() {
                return None;
            }
            let mtime = md.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((mtime, md.len(), e.path()))
        })
        .collect();
    sized.sort_by(|a, b| a.0.cmp(&b.0));
    let mut total: u64 = sized.iter().map(|(_, s, _)| *s).sum();
    for (_, s, p) in sized {
        if total <= THUMB_MAX_BYTES && thumb_count_hint(&dir) <= THUMB_MAX_FILES {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            total = total.saturating_sub(s);
        }
    }
}

fn thumb_count_hint(dir: &Path) -> usize { std::fs::read_dir(dir).map(|r| r.count()).unwrap_or(0) }

fn request_cached_thumb(path: &str, mtime: Option<u64>, size: u64) {
    static INFLIGHT: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if size == 0 || size > THUMB_MAX_FILE_BYTES {
        return;
    }
    let dest = thumb_path_for(path, mtime, size);
    if dest.is_file() {
        return;
    }
    let key = dest.to_string_lossy().to_string();
    if let Ok(v) = INFLIGHT.lock().map(|v| v.contains(&key)) {
        if v {
            return;
        }
    }
    if let Ok(mut v) = INFLIGHT.lock() {
        if v.len() > 32 {
            return;
        }
        v.push(key.clone());
    }
    let path_o = path.to_string();
    std::thread::spawn(move || {
        let _ = ensure_cached_thumb(&path_o, mtime, size);
        let _ = INFLIGHT.lock().map(|mut v| v.retain(|k| k != &key));
    });
}

fn ensure_cached_thumb(path: &str, mtime: Option<u64>, size: u64) -> Option<PathBuf> {
    if size == 0 || size > THUMB_MAX_FILE_BYTES {
        return None;
    }
    let dest = thumb_path_for(path, mtime, size);
    if dest.is_file() {
        return Some(dest);
    }
    let _ = std::fs::create_dir_all(thumb_cache_dir());
    let lower = path.to_lowercase();
    if lower.ends_with(".pdf") {
        let prefix = dest.with_extension("");
        let r = run_cmd("pdftoppm", &["-png", "-singlefile", "-scale-to", "256", path, &prefix.to_string_lossy()]);
        if r.is_ok() && dest.is_file() {
            prune_thumb_cache();
            return Some(dest);
        }
        return None;
    }
    if !is_image_ext(Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default().as_str()) {
        return None;
    }
    match image::open(path) {
        Ok(img) => {
            let t = img.thumbnail(128, 128);
            match t.save(&dest) {
                Ok(()) => {
                    prune_thumb_cache();
                    Some(dest)
                }
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

fn is_pdf_name(name: &str) -> bool { name.to_lowercase().ends_with(".pdf") }

fn is_text_previewable(name: &str) -> bool {
    let l = name.to_lowercase();
    let ext = l.rsplit('.').next().unwrap_or("");
    matches!(ext, "txt" | "md" | "log" | "csv" | "ini" | "conf" | "toml" | "json" | "yaml" | "yml" | "rs" | "ts" | "js" | "py" | "go" | "c" | "cpp" | "h" | "sh" | "css" | "html")
}

fn read_text_preview(path: &str) -> Option<String> {
    if !is_text_previewable(Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default().as_str()) {
        return None;
    }
    let md = std::fs::metadata(path).ok()?;
    if md.len() == 0 {
        return None;
    }
    let f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    use std::io::Read;
    f.take(TEXT_PREVIEW_MAX_BYTES).read_to_end(&mut buf).ok()?;
    let s = String::from_utf8_lossy(&buf);
    let truncated = md.len() > TEXT_PREVIEW_MAX_BYTES || s.chars().count() > TEXT_PREVIEW_MAX_CHARS;
    let mut out: String = s.chars().take(TEXT_PREVIEW_MAX_CHARS).collect();
    if truncated {
        out.push_str("…");
    }
    Some(out)
}

fn read_pdf_preview(path: &str) -> Option<String> {
    if !is_pdf_name(
        Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default().as_str(),
    ) {
        return None;
    }
    let out = std::process::Command::new("pdftotext")
        .args(["-l", "1", "-f", "1", path, "-"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let truncated = s.chars().count() > TEXT_PREVIEW_MAX_CHARS;
    let mut preview: String = s.chars().take(TEXT_PREVIEW_MAX_CHARS).collect();
    if truncated {
        preview.push_str("…");
    }
    Some(preview)
}

fn is_av_name(name: &str) -> bool {
    let l = name.to_lowercase();
    let ext = l.rsplit('.').next().unwrap_or("");
    matches!(ext, "mp4" | "mkv" | "avi" | "mov" | "webm" | "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a")
}

fn restore_all_trash() -> Result<usize, String> {
    let (files, info) = trash_dirs();
    let entries = list_trash_entries();
    let mut n = 0usize;
    let mut errs = Vec::new();
    for e in entries {
        let src = files.join(&e.trash_name);
        if !src.exists() {
            let _ = std::fs::remove_file(info.join(format!("{}.trashinfo", e.trash_name)));
            continue;
        }
        let dest = dedupe_restore_dest(&e.orig_path, &e.trash_name);
        match move_path(&src, &dest) {
            Ok(()) => {
                let _ = std::fs::remove_file(info.join(format!("{}.trashinfo", e.trash_name)));
                n += 1;
            }
            Err(err) => errs.push(format!("{}: {err}", e.trash_name)),
        }
    }
    let _ = LAST_TRASHED.lock().map(|mut v| v.clear());
    let _ = std::fs::remove_file(trash_undo_file());
    if errs.is_empty() { Ok(n) } else { Err(errs.join("; ")) }
}

fn restore_trash_one(trash_name: &str, overwrite: bool) -> Result<PathBuf, String> {
    let (files, info) = trash_dirs();
    let src = files.join(trash_name);
    if !src.exists() {
        return Err(format!("{trash_name}: not in Trash"));
    }
    let info_path = info.join(format!("{trash_name}.trashinfo"));
    let (orig, _) =
        parse_trashinfo(&info_path).map(|(o, d)| (percent_decode(&o), d)).unwrap_or((String::new(), String::new()));
    if orig.is_empty() {
        return Err(format!("{trash_name}: origin unknown, cannot restore safely"));
    }
    let dest = PathBuf::from(&orig);
    if dest.exists() && !overwrite {
        return Err(format!("CONFLICT:{orig}"));
    }
    if dest.exists() && overwrite {
        if dest.is_dir() {
            std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(&dest).map_err(|e| e.to_string())?;
        }
    }
    move_path(&src, &dest).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&info_path);
    Ok(dest)
}

fn delete_trash_one(trash_name: &str) -> Result<(), String> {
    let (files, info) = trash_dirs();
    let p = files.join(trash_name);
    if !p.exists() {
        return Err(format!("{trash_name}: not in Trash"));
    }
    let r = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
    r.map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(info.join(format!("{trash_name}.trashinfo")));
    Ok(())
}

fn undo_last_op() -> Result<String, String> {
    let op: Option<OpUndo> = LAST_OP.lock().map(|mut v| v.take()).map_err(|e| e.to_string())?;
    let Some(op) = op else { return Ok("Nothing to undo".to_string()) };
    if op.kind == "copy" {
        let mut n = 0usize;
        for (_, dst) in &op.pairs {
            let p = PathBuf::from(dst);
            let r = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
            if r.is_ok() {
                n += 1;
            }
        }
        Ok(format!("Undid copy of {n} item{}", if n == 1 { "" } else { "s" }))
    } else if op.kind == "move" {
        let mut n = 0usize;
        let mut errs = Vec::new();
        for (src, dst) in &op.pairs {
            match move_path(Path::new(dst), Path::new(src)) {
                Ok(()) => n += 1,
                Err(e) => errs.push(format!("{dst}: {e}")),
            }
        }
        if errs.is_empty() {
            Ok(format!("Undid move of {n} item{}", if n == 1 { "" } else { "s" }))
        } else {
            Err(errs.join("; "))
        }
    } else {
        Ok("Nothing to undo".to_string())
    }
}

fn is_archive_ext(name: &str) -> bool {
    let l = name.to_lowercase();
    l.ends_with(".zip") || l.ends_with(".tar.gz") || l.ends_with(".tgz") || l.ends_with(".tar.xz") || l.ends_with(".tar")
}

fn compress_to_archive(paths: &[String], dest: &Path) -> Result<(), String> {
    if paths.is_empty() {
        return Err("Nothing to compress".to_string());
    }
    if dest.exists() {
        return Err("Archive already exists".to_string());
    }
    let out = std::process::Command::new("tar")
        .arg("-czPf")
        .arg(dest)
        .args(paths)
        .output()
        .map_err(|e| format!("tar not available: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "Compress failed".to_string() } else { err });
    }
    Ok(())
}

fn extract_archive_here(archive: &str, dest_dir: &str) -> Result<(), String> {
    let l = archive.to_lowercase();
    if l.ends_with(".zip") {
        let out = std::process::Command::new("unzip")
            .arg("-o")
            .arg(archive)
            .arg("-d")
            .arg(dest_dir)
            .output()
            .map_err(|e| format!("unzip not available: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() { "Extract failed".to_string() } else { err });
        }
        Ok(())
    } else {
        let out = std::process::Command::new("tar")
            .arg("-xzPf")
            .arg(archive)
            .arg("-C")
            .arg(dest_dir)
            .output()
            .map_err(|e| format!("tar not available: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() { "Extract failed".to_string() } else { err });
        }
        Ok(())
    }
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
        let res = ipc::send_command(
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

fn perform_search_scoped(query: String, scope: SearchScope, dir: String, mut items_state: State<Vec<Item>>) {
    if scope == SearchScope::Current {
        let q = query.to_lowercase();
        let dir_c = dir.clone();
        use std::sync::atomic::Ordering;
        let my_gen = LOAD_GEN.fetch_add(1, Ordering::SeqCst) + 1;
        LOAD_BUSY.store(true, Ordering::SeqCst);
        let _ = LOAD_ERR.lock().map(|mut e| *e = None);
        items_state.set(Vec::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<Item>>();
        std::thread::spawn(move || {
            let mut result: Vec<Item> = Vec::new();
            let mut stack = vec![dir_c.clone()];
            let mut seen = 0usize;
            while let Some(d) = stack.pop() {
                if seen > 4000 {
                    break;
                }
                let entries = std::fs::read_dir(&d).map(|r| r.filter_map(|e| e.ok()).collect::<Vec<_>>()).unwrap_or_default();
                for e in entries {
                    seen += 1;
                    let p = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if name.to_lowercase().contains(&q) {
                        let md = e.metadata().ok();
                        let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified = md
                            .as_ref()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());
                        result.push(Item {
                            ty: if is_dir { ItemType::Folder } else { ItemType::File },
                            name,
                            path: p.to_string_lossy().to_string(),
                            size,
                            modified,
                        });
                    } else if is_dir && seen < 4000 {
                        stack.push(p.to_string_lossy().to_string());
                    }
                    if result.len() >= 300 {
                        break;
                    }
                }
                if result.len() >= 300 {
                    break;
                }
            }
            result.sort_by(|a, b| a.name.cmp(&b.name));
            let _ = tx.send(result);
        });
        spawn(async move {
            use std::sync::atomic::Ordering;
            if let Some(result) = rx.recv().await {
                if LOAD_GEN.load(Ordering::SeqCst) != my_gen {
                    return;
                }
                LOAD_BUSY.store(false, Ordering::SeqCst);
                items_state.set(result);
            } else {
                LOAD_BUSY.store(false, Ordering::SeqCst);
            }
        });
    } else {
        perform_search(query, items_state)
    }
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
    let pairs: Vec<(String, String)> = take_trash_undo();
    if pairs.is_empty() {
        return Ok(0);
    }
    let (files, info) = trash_dirs();
    let mut n = 0usize;
    let mut errs = Vec::new();
    for (uname, orig) in pairs {
        let src = files.join(&uname);
        let dest = dedupe_restore_dest(&orig, &uname);
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

fn copy_dir_recursive_cancel_progress(
    src: &PathBuf,
    dst: &PathBuf,
    cancel: State<bool>,
    on_file: &mut dyn FnMut(u64),
) -> std::io::Result<()> {
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
            copy_dir_recursive_cancel_progress(&e.path(), &dst_path, cancel, on_file)?;
        } else {
            let bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
            std::fs::copy(e.path(), dst_path)?;
            on_file(bytes);
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

fn dedupe_restore_dest(orig: &str, fallback_name: &str) -> PathBuf {
    if orig.is_empty() {
        let home = env::var("HOME").map(PathBuf::from).unwrap_or(PathBuf::from("/tmp"));
        return unique_dest(&home, fallback_name);
    }
    let p = PathBuf::from(orig);
    if !p.exists() {
        return p;
    }
    let parent = p.parent().map(|x| x.to_path_buf()).unwrap_or(PathBuf::from("/"));
    let name = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or(fallback_name.to_string());
    unique_dest(&parent, &name)
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

fn try_open_path(path: &str, mut notice: State<Option<(String, NoticeKind)>>) {
    if let Err(e) = std::process::Command::new("xdg-open").arg(path).spawn() {
        notice.set(Some((format!("Open failed: {path}: {e}"), NoticeKind::Error)));
    }
}

fn danger_button_full_local(text: impl Into<String>, mut on_press: impl FnMut() + 'static) -> impl IntoElement {
    let t = use_app_theme();
    let text_str = text.into();
    rect()
        .width(Size::fill())
        .padding((10., 16.))
        .corner_radius(8.)
        .background(t.accent_red)
        .center()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(text_str))
}

fn open_with_chooser(path: &str, mut notice: State<Option<(String, NoticeKind)>>) {
    let p = path.to_string();
    let r = std::process::Command::new("mimeopen")
        .arg("-d")
        .arg(&p)
        .spawn()
        .or_else(|_| std::process::Command::new("xdg-open").arg(&p).spawn());
    if let Err(e) = r {
        notice.set(Some((format!("Open failed: {p}: {e}"), NoticeKind::Error)));
    }
}

fn show_in_folder(path: &str, mut notice: State<Option<(String, NoticeKind)>>) {
    let parent = PathBuf::from(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or("/".to_string());
    if let Err(e) = std::process::Command::new("xdg-open").arg(&parent).spawn() {
        notice.set(Some((format!("Open location failed: {parent}: {e}"), NoticeKind::Error)));
    }
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
        speed: String::new(),
    }));
    spawn(async move {
        let started = std::time::Instant::now();
        let mut ok = 0usize;
        let mut skipped = 0usize;
        let mut cancelled = false;
        let mut err: Option<String> = None;
        let mut done_pairs: Vec<(String, String)> = Vec::new();
        let mut bytes_done: u64 = 0;
        for (i, src) in srcs.iter().enumerate() {
            if *job_cancel.read() {
                cancelled = true;
                break;
            }
            let name = PathBuf::from(src).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("file".to_string());
            let dest = PathBuf::from(&dest_dir).join(&name);
            let speed = format_speed(bytes_done, started.elapsed().as_secs_f64());
            job_progress.set(Some(JobProgress {
                label: if is_cut { "Moving".to_string() } else { "Copying".to_string() },
                done: i,
                total,
                current: name.clone(),
                speed,
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
                    let mut prog = job_progress;
                    let label = if is_cut { "Moving".to_string() } else { "Copying".to_string() };
                    let cur_name = name.clone();
                    let t0 = started;
                    let acc = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(bytes_done));
                    let acc_c = acc.clone();
                    let r = copy_dir_recursive_cancel_progress(&PathBuf::from(src), &dest, job_cancel, &mut |chunk| {
                        use std::sync::atomic::Ordering;
                        let b = acc_c.fetch_add(chunk, Ordering::Relaxed).saturating_add(chunk);
                        let speed = format_speed(b, t0.elapsed().as_secs_f64());
                        prog.set(Some(JobProgress {
                            label: label.clone(),
                            done: i,
                            total,
                            current: cur_name.clone(),
                            speed,
                        }));
                    })
                    .map_err(|e| e.to_string());
                    bytes_done = acc.load(std::sync::atomic::Ordering::Relaxed);
                    r
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
            if PathBuf::from(src).is_dir() && !is_cut {
                bytes_done = dir_byte_size(&dest).max(bytes_done);
            } else {
                bytes_done = bytes_done.saturating_add(std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0));
            }
            done_pairs.push((src.clone(), dest.to_string_lossy().to_string()));
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
            if !done_pairs.is_empty() {
                let kind = if is_cut { "move" } else { "copy" }.to_string();
                let label =
                    format!("{ok} item{} {}", if ok == 1 { "" } else { "s" }, if is_cut { "moved" } else { "copied" });
                let _ = LAST_OP.lock().map(|mut v| {
                    *v = Some(OpUndo { kind, pairs: done_pairs, label });
                });
            }
            let mut msg = format!("{ok} item{} {}", if ok == 1 { "" } else { "s" }, if is_cut { "moved" } else { "copied" });
            if skipped > 0 {
                msg.push_str(&format!(" ({skipped} skipped, already exist)"));
            }
            msg.push_str(" (Undo available)");
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
    job_progress.set(Some(JobProgress {
        label: "Trashing".to_string(),
        done: 0,
        total,
        current: String::new(),
        speed: String::new(),
    }));
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
            job_progress.set(Some(JobProgress {
                label: "Trashing".to_string(),
                done: i,
                total,
                current: p.clone(),
                speed: String::new(),
            }));
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
                remember_trashed(trashed);
            }
            notice.set(Some((format!("Trash failed: {e}"), NoticeKind::Error)));
        } else {
            if !trashed.is_empty() {
                remember_trashed(trashed);
            }
            notice.set(Some((
                format!("Moved {n} item{} to Trash (Undo available)", if n == 1 { "" } else { "s" }),
                NoticeKind::Success,
            )));
            load_dir(current_dir, items);
        }
    });
}

/// Resolve the initial directory from deep-link args.
///
/// Accepts absolute paths (`/home/user/docs`, `~/docs`), `trash`, `home`,
/// and `finick://files/<path>` / `finick://files?path=<path>` forms. The
/// `extra` value (`--path`, `?path=`, second positional) wins when it looks
/// like a path so `files files --path /tmp` works.
fn resolve_initial_path(args: &config::LaunchArgs) -> Option<String> {
    let mut candidates: Vec<&str> = Vec::new();
    if let Some(e) = args.extra.as_deref() {
        candidates.push(e);
    }
    if let Some(p) = args.page.as_deref() {
        candidates.push(p);
    }
    // Prefer an explicit raw URI carrying a network address or full path.
    if let Some(raw) = args.raw_uri.as_deref() {
        candidates.push(raw);
    }
    for cand in candidates {
        if let Some(resolved) = normalize_initial_dir(cand) {
            return Some(resolved);
        }
    }
    None
}

fn normalize_initial_dir(cand: &str) -> Option<String> {
    let t = cand.trim();
    if t.is_empty() {
        return None;
    }
    // Strip `finick://files/...` / `files:...` wrappers back to a path.
    let mut s = t;
    if let Some(idx) = t.find("://") {
        let after = &t[idx + 3..];
        let stripped = after.trim_start_matches('/');
        // `finick://files/home/user/x` -> `/home/user/x` when it looks absolute.
        let first = stripped.split('/').next().unwrap_or("");
        if first.eq_ignore_ascii_case("files") || first.eq_ignore_ascii_case("finick") {
            let tail = stripped[first.len()..].trim_start_matches('/');
            if tail.is_empty() {
                s = "/";
            } else if tail.starts_with("home/") || tail.starts_with("tmp/") || tail.starts_with("mnt/") {
                s = Box::leak(format!("/{tail}").into_boxed_str());
            } else {
                s = tail;
            }
        } else if after.starts_with("home/") || after.starts_with("tmp/") {
            s = Box::leak(format!("/{after}").into_boxed_str());
        } else {
            s = after;
        }
    } else if let Some(idx) = t.find(':') {
        let (scheme, after) = t.split_at(idx);
        if !scheme.is_empty()
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
            && (scheme.eq_ignore_ascii_case("files") || scheme.eq_ignore_ascii_case("finick"))
        {
            s = after[1..].trim_start_matches('/');
            if s.is_empty() {
                return None;
            }
        }
    }
    let s = s.trim();
    if s.eq_ignore_ascii_case("trash") || s.eq_ignore_ascii_case("trash://") {
        return Some(trash_files_path().to_string_lossy().to_string());
    }
    if s.eq_ignore_ascii_case("home") || s == "~" {
        return env::home_dir().map(|v| v.to_str().unwrap().to_string());
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = env::home_dir().and_then(|v| v.to_str().map(|x| x.to_string())) {
            return Some(format!("{home}/{rest}"));
        }
    }
    if s.starts_with('/') {
        return Some(s.trim_end_matches('/').to_string().tap_root());
    }
    // Bare `Documents` / `Downloads` relative to $HOME when that dir exists.
    if !s.contains(' ') && !s.contains("://") {
        if let Some(home) = env::home_dir().and_then(|v| v.to_str().map(|x| x.to_string())) {
            let rel = format!("{home}/{s}");
            if Path::new(&rel).is_dir() {
                return Some(rel);
            }
        }
    }
    None
}

trait TapRoot {
    fn tap_root(self) -> Self;
}

impl TapRoot for String {
    fn tap_root(self) -> Self {
        if self.is_empty() { "/".to_string() } else { self }
    }
}

pub const FILES_PAGE_SLUGS: &[&str] = &["<absolute-path>", "~/<path>", "trash", "home", "Documents", "Downloads"];

struct FilesApp {
    initial_path: Option<String>,
}

impl freya::prelude::App for FilesApp {
    fn render(&self) -> impl IntoElement {
        files_root(self.initial_path.clone())
    }
}

fn files_root(initial_path: Option<String>) -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let default_home = env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let start_dir = initial_path.unwrap_or_else(|| default_home.clone());
    let current_path = use_state(move || start_dir.clone());
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
    let last_cp = use_state({
        let p = current_path.read().clone();
        move || p.clone()
    });
    let loaded = use_state(|| false);
    let history = use_state({
        let p = current_path.read().clone();
        move || vec![p.clone()]
    });
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
    let show_perm_delete = use_state(|| false);
    let pending_perm_delete = use_state(Vec::<String>::new);
    let show_restore_conflict = use_state(|| false);
    let pending_restore = use_state(|| Option::<String>::None);
    let restore_conflict_path = use_state(String::new);
    let op_undo_tick = use_state(|| 0u64);
    let trash_orig = use_state(|| std::collections::HashMap::<String, String>::new());
    let drop_highlight = use_state(|| false);
    let show_second = use_state(|| false);
    let second_path = use_state({
        let p = current_path.read().clone();
        move || p.clone()
    });
    let second_items = use_state(Vec::<Item>::new);
    let drop_target = use_state(|| Option::<String>::None);
    let tabs = use_state({
        let p = current_path.read().clone();
        move || vec![p.clone()]
    });
    let active_tab = use_state(|| 0usize);
    let tabs_loaded = use_state(|| false);
    let search_scope = use_state(|| SearchScope::Current);
    let type_filter = use_state(|| TypeFilter::All);
    let date_filter = use_state(|| DateFilter::Any);
    let drawer_tab = use_state(|| DrawerTab::Preview);
    let show_connect = use_state(|| false);
    let connect_uri = use_state(String::new);
    let starred = use_state(load_starred);
    let recent_files = use_state(load_recent_files);
    let volumes = use_state(Vec::<VolumeInfo>::new);
    let show_volumes = use_state(|| true);
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
            use {
                notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher},
                std::sync::atomic::Ordering,
            };
            let path_now = current_path.read().clone();
            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let mut watcher: Option<RecommendedWatcher> = notify::recommended_watcher(move |res| {
                if let Ok(ev) = res {
                    let ev: notify::Event = ev;
                    match ev.kind {
                        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            let _ = tx.send(());
                        }
                        _ => {}
                    }
                }
            })
            .ok();
            if let Some(w) = watcher.as_mut() {
                let _ = w.watch(Path::new(&path_now), RecursiveMode::NonRecursive);
                WATCH_SEQ.store(1, Ordering::SeqCst);
            }
            let mut last_poll = std::time::Instant::now();
            loop {
                let got = rx.recv_timeout(std::time::Duration::from_millis(400)).is_ok();
                let seq = WATCH_SEQ.load(Ordering::SeqCst);
                if got {
                    let next = *t.read();
                    t.set(next + 1);
                    last_poll = std::time::Instant::now();
                } else if last_poll.elapsed().as_secs() >= 5 {
                    let next = *t.read();
                    t.set(next + 1);
                    last_poll = std::time::Instant::now();
                }
                let _ = seq;
                let cur = current_path.read().clone();
                if let Some(w) = watcher.as_mut() {
                    let _ = w.watch(Path::new(&cur), RecursiveMode::NonRecursive);
                }
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
            load_dir(path.clone(), items);
        }
        if path == trash_files_path().to_string_lossy().to_string() {
            let mut tm = trash_orig;
            tm.set(trash_orig_map());
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
        {
            let ai = *active_tab.read();
            let mut tv = tabs.read().clone();
            if ai < tv.len() {
                tv[ai] = cp.clone();
                let mut tbs = tabs;
                tbs.set(tv);
            }
        }
    }

    if !*loaded.read() {
        let mut l = loaded;
        l.set(true);
        load_dir(current_path.read().clone(), items);
    }
    if !*tabs_loaded.read() {
        let mut tl = tabs_loaded;
        tl.set(true);
        let mut v = volumes;
        v.set(list_volumes());
    }

    let pinned_val = pinned.read().clone();
    if !pinned_val.is_empty() {
        let _ = save_pinned(&pinned_val);
    }
    let starred_val = starred.read().clone();
    if !starred_val.is_empty() {
        let _ = save_starred(&starred_val);
    }
    let home_path = env::home_dir().map(|v| v.to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let docs_path = env::home_dir().map(|v| v.join("Documents").to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let dl_path = env::home_dir().map(|v| v.join("Downloads").to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let pic_path = env::home_dir().map(|v| v.join("Pictures").to_str().unwrap().to_string()).unwrap_or("/".to_string());
    let trash_path = trash_files_path().to_string_lossy().to_string();
    let recent_list = recent_dirs.read().clone();
    let recent_file_list = recent_files.read().clone();
    let starred_list = starred.read().clone();
    let vols = volumes.read().clone();
    let is_trash_view = cp == trash_path;

    let sidebar =
        rect()
            .width(Size::flex(0.6))
            .max_width(Size::px(260.))
            .height(Size::fill())
            .background(t.sidebar_bg)
            .border(Border::new().width(1.).fill(t.border))
            .padding(12.)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .main_align(Alignment::SpaceBetween)
                    .margin((0., 0., 4., 0.))
                    .child(nav_group_label("PLACES"))
                    .child(ghost_button("Collapse", {
                        let mut s = show_sidebar;
                        move || s.set(false)
                    })),
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
            .maybe(!vols.is_empty() || *show_volumes.read(), |el| {
                let vols_c = vols.clone();
                el.child(
                    rect()
                        .margin((4., 0., 0., 0.))
                        .content(Content::Flex)
                        .child(
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .content(Content::Flex)
                                .main_align(Alignment::SpaceBetween)
                                .cross_align(Alignment::Center)
                                .child(nav_group_label("VOLUMES"))
                                .child(ghost_button("Refresh", {
                                    let mut v = volumes;
                                    move || v.set(list_volumes())
                                })),
                        )
                        .children(vols_c.into_iter().map(|vol| {
                            let is_here = vol.mounted && !vol.mount_path.is_empty() && *current_path.read() == vol.mount_path;
                            let mp = vol.mount_path.clone();
                            let vol_c = vol.clone();
                            let frac = if vol.total > 0 { 1.0 - (vol.avail as f64 / vol.total as f64) } else { 0.0 };
                            let frac_c = frac.clamp(0.0, 1.0);
                            let label = if vol.mounted && !mp.is_empty() {
                                format!("{} ({})", vol.name, format_size(vol.avail))
                            } else {
                                format!("{} (unmounted)", vol.name)
                            };
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .content(Content::Flex)
                                .spacing(4.)
                                .child(nav_item(STORAGE, vol.name.clone(), is_here, {
                                    let mut cur = current_path;
                                    let items_c = items;
                                    let mp2 = mp.clone();
                                    let mut note = notice;
                                    move || {
                                        if !mp2.is_empty() {
                                            cur.set(mp2.clone());
                                            load_dir(mp2.clone(), items_c);
                                        } else {
                                            match mount_volume(&vol_c) {
                                                Ok(p) => {
                                                    note.set(Some((format!("Mounted: {p}"), NoticeKind::Success)));
                                                    let mut vv = volumes;
                                                    vv.set(list_volumes());
                                                    if Path::new(&p).is_dir() {
                                                        cur.set(p.clone());
                                                        load_dir(p, items_c);
                                                    }
                                                }
                                                Err(e) => note.set(Some((format!("Mount failed: {e}"), NoticeKind::Error))),
                                            }
                                        }
                                    }
                                }))
                                .child(
                                    rect()
                                        .horizontal()
                                        .content(Content::Flex)
                                        .width(Size::fill())
                                        .spacing(4.)
                                        .cross_align(Alignment::Center)
                                        .child(
                                            rect()
                                                .width(Size::flex(1.))
                                                .content(Content::Flex)
                                                .child(multi_segment_bar(vec![((frac_c * 100.0) as f32, t.accent)], false)),
                                        )
                                        .maybe(vol.mounted, |row| {
                                            let vc = vol.clone();
                                            let home_c = home_path.clone();
                                            row.child(ghost_button("Unmount", {
                                                let mut note = notice;
                                                let vc2 = vc.clone();
                                                let mut cur = current_path;
                                                let items_c = items;
                                                let home_c2 = home_c.clone();
                                                move || match unmount_volume(&vc2) {
                                                    Ok(m) => {
                                                        note.set(Some((format!("Unmounted: {m}"), NoticeKind::Success)));
                                                        let mut vv = volumes;
                                                        vv.set(list_volumes());
                                                        let cur_p = cur.read().clone();
                                                        if !vc2.mount_path.is_empty()
                                                            && (cur_p == vc2.mount_path
                                                                || cur_p.starts_with(&format!("{}/", vc2.mount_path)))
                                                        {
                                                            cur.set(home_c2.clone());
                                                            load_dir(home_c2.clone(), items_c);
                                                        }
                                                    }
                                                    Err(e) => note.set(Some((format!("Unmount failed: {e}"), NoticeKind::Error))),
                                                }
                                            }))
                                            .child(ghost_button("Eject", {
                                                let mut note = notice;
                                                let vc2 = vc.clone();
                                                let mut cur = current_path;
                                                let items_c = items;
                                                let home_c2 = home_c.clone();
                                                move || match eject_volume(&vc2) {
                                                    Ok(m) => {
                                                        note.set(Some((format!("Ejected: {m}"), NoticeKind::Success)));
                                                        let mut vv = volumes;
                                                        vv.set(list_volumes());
                                                        let cur_p = cur.read().clone();
                                                        if !vc2.mount_path.is_empty()
                                                            && (cur_p == vc2.mount_path
                                                                || cur_p.starts_with(&format!("{}/", vc2.mount_path)))
                                                        {
                                                            cur.set(home_c2.clone());
                                                            load_dir(home_c2.clone(), items_c);
                                                        }
                                                    }
                                                    Err(e) => note.set(Some((format!("Eject failed: {e}"), NoticeKind::Error))),
                                                }
                                            }))
                                        })
                                        .maybe(!vol.mounted, |row| {
                                            let vc = vol.clone();
                                            row.child(ghost_button("Mount", {
                                                let mut note = notice;
                                                let vc2 = vc.clone();
                                                let mut cur = current_path;
                                                let items_c = items;
                                                move || match mount_volume(&vc2) {
                                                    Ok(p) => {
                                                        note.set(Some((format!("Mounted: {p}"), NoticeKind::Success)));
                                                        let mut vv = volumes;
                                                        vv.set(list_volumes());
                                                        if Path::new(&p).is_dir() {
                                                            cur.set(p.clone());
                                                            load_dir(p, items_c);
                                                        }
                                                    }
                                                    Err(e) => note.set(Some((format!("Mount failed: {e}"), NoticeKind::Error))),
                                                }
                                            }))
                                        }),
                                )
                                .child(tile_sub(label))
                                .into_element()
                        })),
                )
            })
            .maybe(!recent_list.is_empty(), |el| {
                el.child(rect().margin((4., 0., 0., 0.)).content(Content::Flex).child(nav_group_label("RECENT")).children(
                    recent_list.iter().map(|path| {
                        let name =
                            PathBuf::from(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(path.clone());
                        sidebar_entry(&name, CLOCK_ICON, &current_path, path.clone(), items, pinned, clipboard)
                    }),
                ))
            })
            .maybe(!recent_file_list.is_empty(), |el| {
                el.child(
                    rect()
                        .margin((4., 0., 0., 0.))
                        .content(Content::Flex)
                        .child(nav_group_label("RECENT FILES"))
                        .children(recent_file_list.iter().map(|path| {
                            let name =
                                PathBuf::from(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(path.clone());
                            sidebar_open_entry(&name, DOCUMENT, &current_path, path.clone(), items, notice, recent_files)
                        })),
                )
            })
            .maybe(!starred_list.is_empty(), |el| {
                el.child(
                    rect()
                        .margin((4., 0., 0., 0.))
                        .content(Content::Flex)
                        .child(nav_group_label("STARRED"))
                        .children(starred_list.iter().map(|(name, path)| {
                            sidebar_open_entry(name, PIN, &current_path, path.clone(), items, notice, recent_files)
                        })),
                )
            })
            .child(if !pinned.read().is_empty() {
                rect()
                    .margin((4., 0., 0., 0.))
                    .content(Content::Flex)
                    .child(nav_group_label("PINNED"))
                    .children(
                        pinned.read().iter().map(|(name, path)| {
                            sidebar_entry(name, PIN, &current_path, path.clone(), items, pinned, clipboard)
                        }),
                    )
                    .into_element()
            } else {
                rect().into_element()
            })
            .child(
                rect()
                    .margin((12., 0., 0., 0.))
                    .vertical()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .spacing(8.)
                    .child(secondary_button_full("Add Place", {
                        let mut show = show_add_place;
                        let mut name_st = add_place_name;
                        let cp2 = current_path.read().clone();
                        move || {
                            let base = PathBuf::from(&cp2)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or("Place".to_string());
                            name_st.set(base);
                            show.set(true);
                        }
                    }))
                    .child(secondary_button_full("Connect to server", {
                        let mut show = show_connect;
                        let mut uri = connect_uri;
                        move || {
                            uri.set(String::new());
                            show.set(true);
                        }
                    })),
            );

    let show_sidebar_val = *show_sidebar.read();
    let view_mode_val = view_mode.read().clone();
    let icon_size_val = icon_size.read().clone();
    let grid_active = view_mode_val == ViewMode::Grid;
    let list_active = view_mode_val == ViewMode::List;

    let leading: Element = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .spacing(8.)
        .child(tool_button(SIDEBAR, t.text, {
            let mut s = show_sidebar;
            let next = !show_sidebar_val;
            move || s.set(next)
        }))
        .child(tool_button(CHEVRON_LEFT, t.text, {
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
        .child(tool_button(CHEVRON_RIGHT, t.text, {
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
        .child(tool_button(REFRESH_CW, t.text, {
            let current = current_path;
            let items = items;
            move || {
                load_dir(current.read().clone(), items);
            }
        }))
        .child(tool_button(ARROW_LEFT, t.text, {
            let mut current = current_path;
            let items = items;
            move || {
                let path = PathBuf::from(current.read().clone());
                if let Some(parent) = path.parent() {
                    let new_path = parent.to_string_lossy().to_string();
                    current.set(new_path.clone());
                    load_dir(new_path, items);
                }
            }
        }))
        .into_element();

    let path_field: Element = rect()
        .width(Size::fill())
        .height(Size::px(36.))
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .background(t.panel)
        .border(Border::new().width(1.).fill(t.border))
        .corner_radius(RADIUS_SM)
        .padding((2., 10.))
        .child(Input::new(uri_input).width(Size::fill()).flat().placeholder("Path or sftp:// smb:// URI").on_submit({
            let mut current = current_path;
            let items = items;
            let note = notice;
            move |val: String| {
                let v = val.trim().to_string();
                if is_network_uri(&v) {
                    open_network_uri(&v, note);
                    return;
                }
                current.set(v.clone());
                load_dir(v, items);
            }
        }))
        .into_element();

    let scope_val = *search_scope.read();
    let search_trail: Element = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .width(Size::fill())
        .spacing(8.)
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .content(Content::Flex)
                .width(Size::flex(1.))
                .child(sidebar_search(
                    search_query,
                    if scope_val == SearchScope::Current { "Search this folder" } else { "Search all locations" },
                )),
        )
        .child(segmented_control(
            vec![("Here", SearchScope::Current), ("Everywhere", SearchScope::All)],
            scope_val,
            {
                let mut s = search_scope;
                let q = search_query;
                let cp = current_path;
                let items_c = items;
                move |v: SearchScope| {
                    s.set(v);
                    let val = q.read().clone();
                    if !val.trim().is_empty() {
                        perform_search_scoped(val, v, cp.read().clone(), items_c);
                    }
                }
            },
        ))
        .child(ghost_button("Search", {
            let q = search_query;
            let cp = current_path;
            let items_c = items;
            let sc = search_scope;
            move || {
                let val = q.read().clone();
                if val.trim().is_empty() {
                    load_dir(cp.read().clone(), items_c);
                } else {
                    perform_search_scoped(val, *sc.read(), cp.read().clone(), items_c);
                }
            }
        }))
        .child(ghost_button("History", {
            let h = history;
            let hi = hist_idx;
            move || {
                let hv = h.read().clone();
                let cur = *hi.read();
                let mut menu = Menu::new();
                for (i, p) in hv.iter().enumerate().rev().take(12) {
                    let target = p.clone();
                    let mut cur_st = current_path;
                    let items_c = items;
                    let mut hi2 = hist_idx;
                    let mut hn2 = hist_nav;
                    let label = format!("{}{}", if i == cur { "● " } else { "" }, target);
                    menu = menu.child(ctx_button(label, move || {
                        hn2.set(true);
                        hi2.set(i);
                        cur_st.set(target.clone());
                        load_dir(target.clone(), items_c);
                    }));
                }
                ContextMenu::open_from_down(menu);
            }
        }))
        .maybe(!search_query.read().is_empty(), |el| {
            el.child(ghost_button("Clear", {
                let mut q = search_query;
                let cp = current_path;
                let items_c = items;
                move || {
                    q.set(String::new());
                    load_dir(cp.read().clone(), items_c);
                }
            }))
        })
        .into_element();

    let tabs_val = tabs.read().clone();
    let active_val = *active_tab.read();
    let tab_bar: Element = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .width(Size::fill())
        .spacing(4.)
        .padding((4., 12., 0., 12.))
        .children(tabs_val.iter().enumerate().map(|(i, p)| {
            let name = PathBuf::from(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(p.clone());
            let short = if name.len() > 18 { format!("{}…", &name[..17]) } else { name };
            let target = p.clone();
            let is_on = i == active_val;
            let target_c = target.clone();
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .content(Content::Flex)
                .spacing(4.)
                .child(status_chip(
                    format!("{} {}", if is_on { "●" } else { "○" }, short),
                    is_on,
                    Some(EventHandler::new(move |()| {
                        let mut cur = current_path;
                        let mut at = active_tab;
                        cur.set(target_c.clone());
                        at.set(i);
                        load_dir(target_c.clone(), items);
                    })),
                ))
                .maybe(tabs_val.len() > 1, |el| {
                    el.child(small_close_button({
                        let mut tbs = tabs;
                        let mut at = active_tab;
                        let mut cur = current_path;
                        let items_c = items;
                        move || {
                            let mut v = tbs.read().clone();
                            if v.len() <= 1 {
                                return;
                            }
                            if i >= v.len() {
                                return;
                            }
                            v.remove(i);
                            let ni = if i < active_val {
                                active_val.saturating_sub(1).min(v.len() - 1)
                            } else {
                                active_val.min(v.len() - 1)
                            };
                            at.set(ni);
                            tbs.set(v.clone());
                            cur.set(v[ni].clone());
                            load_dir(v[ni].clone(), items_c);
                        }
                    }))
                })
                .into_element()
        }))
        .child(ghost_button("+ Tab", {
            let mut tbs = tabs;
            let mut at = active_tab;
            let cpv = current_path.read().clone();
            move || {
                let mut v = tbs.read().clone();
                v.push(cpv.clone());
                at.set(v.len() - 1);
                tbs.set(v);
            }
        }))
        .into_element();

    let top_bar = top_bar_custom(leading, path_field, Some(search_trail));

    let is_grid = *view_mode.read() == ViewMode::Grid;
    let show_hidden_val = *show_hidden.read();
    let sort_val = *sort_key.read();
    let dirs_first_val = *dirs_first.read();
    let type_val = *type_filter.read();
    let date_val = *date_filter.read();
    let items_read: Vec<Item> = visible_items(&items.read(), sort_val, dirs_first_val, show_hidden_val)
        .into_iter()
        .filter(|it| matches_type_filter(it, type_val) && matches_date_filter(it, date_val))
        .collect();
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

    let filter_bar: Element = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .width(Size::fill())
        .spacing(8.)
        .padding((4., 12., 0., 12.))
        .child(segmented_control(
            vec![
                ("All", TypeFilter::All),
                ("Images", TypeFilter::Images),
                ("Docs", TypeFilter::Docs),
                ("Code", TypeFilter::Code),
                ("Audio", TypeFilter::Audio),
                ("Video", TypeFilter::Video),
                ("Archives", TypeFilter::Archives),
            ],
            type_val,
            {
                let mut s = type_filter;
                move |v: TypeFilter| s.set(v)
            },
        ))
        .child(segmented_control(
            vec![("Any time", DateFilter::Any), ("Today", DateFilter::Today), ("Week", DateFilter::Week), ("Month", DateFilter::Month)],
            date_val,
            {
                let mut s = date_filter;
                move |v: DateFilter| s.set(v)
            },
        ))
        .maybe(!search_query.read().is_empty(), |el| {
            el.child(notice_pill(
                if scope_val == SearchScope::Current { "Scoped to this folder" } else { "Searching all locations" },
                NoticeKind::Info,
            ))
        })
        .into_element();

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
            move || {
                let next = !*d.read();
                d.set(next)
            }
        }))
        .child(focus_pill("Hidden", show_hidden_val, {
            let mut h = show_hidden;
            move || {
                let next = !*h.read();
                h.set(next)
            }
        }))
        .child(segmented_control(
            vec![("S", IconSize::Small), ("M", IconSize::Medium), ("L", IconSize::Large)],
            icon_size_val,
            {
                let mut s = icon_size;
                move |v: IconSize| s.set(v)
            },
        ))
        .child(tool_button(LAYOUT_GRID, if grid_active { t.accent } else { t.text_dim }, {
            let mut vm = view_mode;
            move || vm.set(ViewMode::Grid)
        }))
        .child(tool_button(LIST, if list_active { t.accent } else { t.text_dim }, {
            let mut vm = view_mode;
            move || vm.set(ViewMode::List)
        }))
        .child(tool_button(PLUS, t.accent, {
            let path_state = current_path;
            let items_state = items;
            move || {
                let target_path = PathBuf::from(path_state.read().clone()).join("New Folder");
                let mut unique_path = target_path.clone();
                let mut counter = 1;
                while unique_path.exists() {
                    unique_path = PathBuf::from(path_state.read().clone()).join(format!("New Folder ({})", counter));
                    counter += 1;
                }
                let _ = std::fs::create_dir_all(&unique_path);
                load_dir(path_state.read().clone(), items_state);
            }
        }))
        .into_element();

    let second_on = *show_second.read();
    let second_path_val = second_path.read().clone();
    let second_count = second_items.read().iter().filter(|i| i.ty == ItemType::Folder).count();
    let second_bar: Element = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .content(Content::Flex)
        .width(Size::fill())
        .spacing(8.)
        .padding((4., 12., 0., 12.))
        .child(ghost_button(if second_on { "Hide second target" } else { "Second target" }, {
            let mut show = show_second;
            let mut sp = second_path;
            let si = second_items;
            let cur = current_path;
            move || {
                let next = !*show.read();
                show.set(next);
                if next {
                    let target = sp.read().clone();
                    let fallback = if Path::new(&target).is_dir() { target } else { cur.read().clone() };
                    sp.set(fallback.clone());
                    load_dir(fallback, si);
                }
            }
        }))
        .maybe(second_on, |el| {
            el.child(ghost_button("Up", {
                let mut sp = second_path;
                let si = second_items;
                move || {
                    let cur = sp.read().clone();
                    if let Some(parent) = PathBuf::from(&cur).parent() {
                        let np = parent.to_string_lossy().to_string();
                        sp.set(np.clone());
                        load_dir(np, si);
                    }
                }
            }))
            .child(tile_sub(format!("Target: {second_path_val} ({second_count} folders)")))
            .child(secondary_button("Copy here", {
                let sp = second_path;
                let si = second_items;
                let sel = selected_paths;
                let one = selected_item;
                let clip = clipboard;
                let mut note = notice;
                move || {
                    let mut srcs = sel.read().clone();
                    if srcs.is_empty() {
                        if let Some(it) = one.read().clone() {
                            srcs = vec![it.path];
                        } else if let Some((v, _)) = clip.read().clone() {
                            srcs = v;
                        }
                    }
                    let dest = sp.read().clone();
                    if let Some(err) = internal_drop_error(&srcs, &dest) {
                        note.set(Some((err, NoticeKind::Warn)));
                        return;
                    }
                    start_paste_job_multi(
                        srcs,
                        false,
                        false,
                        dest,
                        si,
                        note,
                        job_running,
                        job_label,
                        job_progress,
                        job_cancel,
                        clip,
                    );
                }
            }))
            .child(secondary_button("Move here", {
                let sp = second_path;
                let si = second_items;
                let sel = selected_paths;
                let one = selected_item;
                let clip = clipboard;
                let mut note = notice;
                let mut multi = selected_paths;
                let mut foc = selected_item;
                move || {
                    let mut srcs = sel.read().clone();
                    if srcs.is_empty() {
                        if let Some(it) = one.read().clone() {
                            srcs = vec![it.path];
                        } else if let Some((v, _)) = clip.read().clone() {
                            srcs = v;
                        }
                    }
                    let dest = sp.read().clone();
                    if let Some(err) = internal_drop_error(&srcs, &dest) {
                        note.set(Some((err, NoticeKind::Warn)));
                        return;
                    }
                    start_paste_job_multi(
                        srcs,
                        false,
                        true,
                        dest,
                        si,
                        note,
                        job_running,
                        job_label,
                        job_progress,
                        job_cancel,
                        clip,
                    );
                    multi.set(Vec::new());
                    foc.set(None);
                }
            }))
            .child(ghost_button("Refresh", {
                let sp = second_path;
                let si = second_items;
                move || load_dir(sp.read().clone(), si)
            }))
        })
        .into_element();

    let trash_bar: Element = if is_trash_view {
        let trash_count = items_total;
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .width(Size::fill())
            .spacing(GAP)
            .padding((4., 12., 0., 12.))
            .child(notice_pill(format!("Trash ({trash_count})"), NoticeKind::Info))
            .child(tile_sub("Restore returns each file to its origin. No silent overwrite."))
            .child(secondary_button("Restore all", {
                let items_c = items;
                let mut note = notice;
                let cp_c = current_path;
                let mut tm = trash_orig;
                move || match restore_all_trash() {
                    Ok(n) => {
                        note.set(Some((
                            format!("Restored {n} item{} to origin", if n == 1 { "" } else { "s" }),
                            NoticeKind::Success,
                        )));
                        tm.set(trash_orig_map());
                        load_dir(cp_c.read().clone(), items_c);
                    }
                    Err(e) => note.set(Some((format!("Restore failed: {e}"), NoticeKind::Error))),
                }
            }))
            .child(danger_button("Empty trash", {
                let mut show = show_perm_delete;
                let mut pend = pending_perm_delete;
                move || {
                    let all: Vec<String> = list_trash_entries().into_iter().map(|e| e.trash_name).collect();
                    pend.set(all);
                    show.set(true);
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
            .padding(GAP)
            .content(if is_grid { Content::Wrap { wrap_spacing: Some(GAP) } } else { Content::Flex })
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
                    let trash_view = is_trash_view;
                    let mut menu = Menu::new();
                    if trash_view {
                        let tn = name_c.clone();
                        menu = menu
                            .child(ctx_button("Restore to origin", {
                                let tn2 = tn.clone();
                                let mut note = notice;
                                let mut show_c = show_restore_conflict;
                                let mut pend_r = pending_restore;
                                let mut conflict_p = restore_conflict_path;
                                let mut tm = trash_orig;
                                move || match restore_trash_one(&tn2, false) {
                                    Ok(dest) => {
                                        note.set(Some((
                                            format!("Restored to {}", dest.to_string_lossy()),
                                            NoticeKind::Success,
                                        )));
                                        tm.set(trash_orig_map());
                                        load_dir(cp_c.read().clone(), items_c);
                                    }
                                    Err(e) if e.starts_with("CONFLICT:") => {
                                        conflict_p.set(e.trim_start_matches("CONFLICT:").to_string());
                                        pend_r.set(Some(tn2.clone()));
                                        show_c.set(true);
                                    }
                                    Err(e) => note.set(Some((format!("Restore failed: {e}"), NoticeKind::Error))),
                                }
                            }))
                            .child(ctx_button("Delete permanently", {
                                let tn2 = tn.clone();
                                let mut pend = pending_perm_delete;
                                let mut show = show_perm_delete;
                                move || {
                                    pend.set(vec![tn2.clone()]);
                                    show.set(true);
                                }
                            }));
                    } else {
                        menu = menu
                            .child(ctx_button(if is_folder_c { "Open" } else { "Open File" }, {
                                let pc = path_c.clone();
                                let mut ps = path_state;
                                let is2 = is_folder_c;
                                let open_notice = notice;
                                move || {
                                    if is2 {
                                        ps.set(pc.clone());
                                        load_dir(pc.clone(), items_c);
                                    } else {
                                        try_open_path(&pc, open_notice);
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
                            .child(ctx_button("Compress to .tar.gz", {
                                let pc = path_c.clone();
                                let mut note = notice;
                                let dest_dir = cp_c.read().clone();
                                let items_cc = items_c;
                                let mut jr = job_running;
                                let mut jp = job_progress;
                                let jc = job_cancel;
                                move || {
                                    let pc2 = pc.clone();
                                    let dest_dir2 = dest_dir.clone();
                                    let dest = PathBuf::from(&dest_dir2).join(format!(
                                        "{}.tar.gz",
                                        PathBuf::from(&pc2)
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or("archive".to_string())
                                    ));
                                    if dest.exists() {
                                        note.set(Some((
                                            "Archive already exists, rename first".to_string(),
                                            NoticeKind::Error,
                                        )));
                                        return;
                                    }
                                    jr.set(true);
                                    jp.set(Some(JobProgress {
                                        label: "Compressing".to_string(),
                                        done: 0,
                                        total: 1,
                                        current: pc2.clone(),
                                        speed: String::new(),
                                    }));
                                    let mut n2 = note;
                                    spawn(async move {
                                        let r = compress_to_archive(&[pc2.clone()], &dest);
                                        jr.set(false);
                                        jp.set(None);
                                        match r {
                                            Ok(()) => {
                                                n2.set(Some(("Compressed".to_string(), NoticeKind::Success)));
                                                load_dir(dest_dir2, items_cc);
                                            }
                                            Err(e) => n2.set(Some((format!("Compress failed: {e}"), NoticeKind::Error))),
                                        }
                                        let _ = jc;
                                    });
                                }
                            }));
                        if !is_folder_c && is_archive_ext(&name_c) {
                            menu = menu.child(ctx_button("Extract here", {
                                let pc = path_c.clone();
                                let dest_dir = cp_c.read().clone();
                                let mut note = notice;
                                let items_cc = items_c;
                                move || match extract_archive_here(&pc, &dest_dir) {
                                    Ok(()) => {
                                        note.set(Some(("Extracted".to_string(), NoticeKind::Success)));
                                        load_dir(dest_dir.clone(), items_cc);
                                    }
                                    Err(e) => note.set(Some((format!("Extract failed: {e}"), NoticeKind::Error))),
                                }
                            }));
                        }
                        menu = menu
                            .child(ctx_button("Open-With", {
                                let pc = path_c.clone();
                                let n = notice;
                                move || open_with_chooser(&pc, n)
                            }))
                            .child(ctx_button("Show in folder", {
                                let pc = path_c.clone();
                                let n = notice;
                                move || show_in_folder(&pc, n)
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
                            }));
                    }
                    menu
                };

                if is_grid {
                    let cached = if (is_img || is_pdf_name(&item.name)) && small_thumb_ok(item.size, idx) {
                        let hit = cached_thumb_if_exists(&path, item.modified, item.size);
                        if hit.is_none() {
                            request_cached_thumb(&path, item.modified, item.size);
                        }
                        hit
                    } else {
                        None
                    };
                    let thumb: Element = if let Some(tp) = cached {
                        rect()
                            .width(Size::px(icon_px + 16.))
                            .height(Size::px(icon_px + 16.))
                            .corner_radius(RADIUS_SM)
                            .overflow(Overflow::Clip)
                            .background(t.panel)
                            .center()
                            .child(
                                ImageViewer::new(tp)
                                    .width(Size::px(icon_px + 12.))
                                    .height(Size::px(icon_px + 12.)),
                            )
                            .into_element()
                    } else {
                        rect()
                            .width(Size::px(icon_px + 16.))
                            .height(Size::px(icon_px + 16.))
                            .center()
                            .child(icon(i_svg, icon_px, if is_folder { t.accent } else { t.text_dim }))
                            .into_element()
                    };
                    rect()
                        .vertical()
                        .width(Size::flex(1.))
                        .max_width(Size::px(200.))
                        .padding(12.)
                        .spacing(8.)
                        .corner_radius(RADIUS_MD)
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
                            let mut dt = drop_target;
                            let folder_p = path.clone();
                            let folder_flag = is_folder;
                            move |_| {
                                if ds.read().is_some() {
                                    let mut cur = ms.read().clone();
                                    if !cur.contains(&it.path) {
                                        cur.push(it.path.clone());
                                        ms.set(cur);
                                    }
                                    ss.set(Some(it.clone()));
                                }
                                if folder_flag
                                    && (!ms.read().is_empty() || ss.read().is_some())
                                    && !ms.read().contains(&folder_p)
                                {
                                    dt.set(Some(folder_p.clone()));
                                }
                            }
                        })
                        .on_pointer_leave({
                            let mut dt = drop_target;
                            let folder_p = path.clone();
                            move |_| {
                                if dt.read().clone().is_some_and(|v| v == folder_p) {
                                    dt.set(None);
                                }
                            }
                        })
                        .on_mouse_up({
                            let mut dt = drop_target;
                            let mut note = notice;
                            let mut multi = multi_state;
                            let mut foc = sel_state;
                            let folder_p = path.clone();
                            let folder_flag = is_folder;
                            let items_c = items_state;
                            move |_| {
                                if !folder_flag {
                                    return;
                                }
                                let mut srcs = multi.read().clone();
                                if srcs.is_empty() {
                                    if let Some(it) = foc.read().clone() {
                                        if it.path != folder_p {
                                            srcs = vec![it.path];
                                        }
                                    }
                                }
                                srcs.retain(|s| s != &folder_p);
                                if srcs.is_empty() {
                                    dt.set(None);
                                    return;
                                }
                                if dt.read().clone().is_none_or(|v| v != folder_p) && multi.read().is_empty() {
                                    return;
                                }
                                if let Some(err) = internal_drop_error(&srcs, &folder_p) {
                                    note.set(Some((err, NoticeKind::Warn)));
                                    dt.set(None);
                                    return;
                                }
                                let is_copy = *ctrl_held.read();
                                start_paste_job_multi(
                                    srcs,
                                    false,
                                    !is_copy,
                                    folder_p.clone(),
                                    items_c,
                                    note,
                                    job_running,
                                    job_label,
                                    job_progress,
                                    job_cancel,
                                    clipboard,
                                );
                                multi.set(Vec::new());
                                foc.set(None);
                                dt.set(None);
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
                            let open_notice = notice;
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
                                        try_open_path(&p, open_notice);
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
                                .font_size(12.)
                                .color(t.text_primary)
                                .text_align(TextAlign::Center)
                                .text(item.name.clone()),
                        )
                        .maybe(
                            is_trash_view && !trash_orig.read().get(&item.name).cloned().unwrap_or_default().is_empty(),
                            |el| {
                                el.child(
                                    label()
                                        .font_size(11.)
                                        .color(t.text_muted)
                                        .text_align(TextAlign::Center)
                                        .text(trash_orig.read().get(&item.name).cloned().unwrap_or_default()),
                                )
                            },
                        )
                        .maybe(is_cut, |el| el.child(status_chip("Cut", true, None)))
                        .maybe(is_folder && drop_target.read().clone().is_some_and(|d| d == path), |el| {
                            el.child(status_chip("Drop", true, None))
                        })
                        .map(is_folder.then(|| path.clone()), |el, folder| {
                            el.on_file_drop({
                                let items_c = items_state;
                                move |e: Event<FileEventData>| {
                                    let srcs: Vec<String> =
                                        e.file_paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
                                    if !srcs.is_empty() {
                                        start_paste_job_multi(
                                            srcs,
                                            false,
                                            false,
                                            folder.clone(),
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
                            })
                        })
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
                            let mut dt = drop_target;
                            let folder_p = path.clone();
                            let folder_flag = is_folder;
                            move |_| {
                                if ds.read().is_some() {
                                    let mut cur = ms.read().clone();
                                    if !cur.contains(&it.path) {
                                        cur.push(it.path.clone());
                                        ms.set(cur);
                                    }
                                    ss.set(Some(it.clone()));
                                }
                                if folder_flag
                                    && (!ms.read().is_empty() || ss.read().is_some())
                                    && !ms.read().contains(&folder_p)
                                {
                                    dt.set(Some(folder_p.clone()));
                                }
                            }
                        })
                        .on_pointer_leave({
                            let mut dt = drop_target;
                            let folder_p = path.clone();
                            move |_| {
                                if dt.read().clone().is_some_and(|v| v == folder_p) {
                                    dt.set(None);
                                }
                            }
                        })
                        .on_mouse_up({
                            let mut dt = drop_target;
                            let mut note = notice;
                            let mut multi = multi_state;
                            let mut foc = sel_state;
                            let folder_p = path.clone();
                            let folder_flag = is_folder;
                            let items_c = items_state;
                            move |_| {
                                if !folder_flag {
                                    return;
                                }
                                let mut srcs = multi.read().clone();
                                if srcs.is_empty() {
                                    if let Some(it) = foc.read().clone() {
                                        if it.path != folder_p {
                                            srcs = vec![it.path];
                                        }
                                    }
                                }
                                srcs.retain(|s| s != &folder_p);
                                if srcs.is_empty() {
                                    dt.set(None);
                                    return;
                                }
                                if dt.read().clone().is_none_or(|v| v != folder_p) && multi.read().is_empty() {
                                    return;
                                }
                                if let Some(err) = internal_drop_error(&srcs, &folder_p) {
                                    note.set(Some((err, NoticeKind::Warn)));
                                    dt.set(None);
                                    return;
                                }
                                let is_copy = *ctrl_held.read();
                                start_paste_job_multi(
                                    srcs,
                                    false,
                                    !is_copy,
                                    folder_p.clone(),
                                    items_c,
                                    note,
                                    job_running,
                                    job_label,
                                    job_progress,
                                    job_cancel,
                                    clipboard,
                                );
                                multi.set(Vec::new());
                                foc.set(None);
                                dt.set(None);
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
                            let open_notice = notice;
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
                                        try_open_path(&p, open_notice);
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
                                .vertical()
                                .child(label().font_size(13.).color(t.text_primary).text(item.name.clone()))
                                .maybe(
                                    is_trash_view
                                        && !trash_orig.read().get(&item.name).cloned().unwrap_or_default().is_empty(),
                                    |el| {
                                        el.child(
                                            label()
                                                .font_size(11.)
                                                .color(t.text_muted)
                                                .text(trash_orig.read().get(&item.name).cloned().unwrap_or_default()),
                                        )
                                    },
                                ),
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
                        .maybe(is_folder && drop_target.read().clone().is_some_and(|d| d == path), |el| {
                            el.child(status_chip("Drop", true, None))
                        })
                        .map(is_folder.then(|| path.clone()), |el, folder| {
                            el.on_file_drop({
                                let items_c = items_state;
                                move |e: Event<FileEventData>| {
                                    let srcs: Vec<String> =
                                        e.file_paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
                                    if !srcs.is_empty() {
                                        start_paste_job_multi(
                                            srcs,
                                            false,
                                            false,
                                            folder.clone(),
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
                            })
                        })
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

    let second_pane: Element = if second_on {
        let folders: Vec<Item> =
            second_items.read().iter().filter(|i| i.ty == ItemType::Folder).cloned().take(40).collect();
        let sp_val = second_path_val.clone();
        rect()
            .vertical()
            .width(Size::flex(0.9))
            .max_width(Size::px(300.))
            .height(Size::fill())
            .content(Content::Flex)
            .background(t.panel)
            .border(Border::new().width(1.).fill(t.border))
            .corner_radius(RADIUS_MD)
            .padding(8.)
            .spacing(4.)
            .child(tile_sub(format!("{} ({} folders)", sp_val, folders.len())))
            .child(tile_sub("Drop selection onto a folder to move, Ctrl drops a copy."))
            .children(folders.into_iter().map(|f| {
                let dest = f.path.clone();
                let label = f.name.clone();
                let is_hl = drop_target.read().clone().is_some_and(|d| d == dest);
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(8.)
                    .padding((6., 8.))
                    .corner_radius(8.)
                    .background(if is_hl { t.bg_selected } else { Color::TRANSPARENT })
                    .on_pointer_enter({
                        let mut dt = drop_target;
                        let d = dest.clone();
                        move |_| {
                            if !selected_paths.read().is_empty() || selected_item.read().is_some() {
                                dt.set(Some(d.clone()));
                            }
                        }
                    })
                    .on_pointer_leave({
                        let mut dt = drop_target;
                        let d = dest.clone();
                        move |_| {
                            if dt.read().clone().is_some_and(|v| v == d) {
                                dt.set(None);
                            }
                        }
                    })
                    .on_mouse_up({
                        let mut dt = drop_target;
                        let mut note = notice;
                        let mut multi = selected_paths;
                        let mut foc = selected_item;
                        let d = dest.clone();
                        let si = second_items;
                        move |_| {
                            let mut srcs = multi.read().clone();
                            if srcs.is_empty() {
                                if let Some(it) = foc.read().clone() {
                                    srcs = vec![it.path];
                                }
                            }
                            srcs.retain(|s| s != &d);
                            if srcs.is_empty() {
                                dt.set(None);
                                return;
                            }
                            if let Some(err) = internal_drop_error(&srcs, &d) {
                                note.set(Some((err, NoticeKind::Warn)));
                                dt.set(None);
                                return;
                            }
                            let is_copy = *ctrl_held.read();
                            start_paste_job_multi(
                                srcs,
                                false,
                                !is_copy,
                                d.clone(),
                                si,
                                note,
                                job_running,
                                job_label,
                                job_progress,
                                job_cancel,
                                clipboard,
                            );
                            multi.set(Vec::new());
                            foc.set(None);
                            dt.set(None);
                        }
                    })
                    .on_file_drop({
                        let si = second_items;
                        let d = dest.clone();
                        move |e: Event<FileEventData>| {
                            let srcs: Vec<String> =
                                e.file_paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
                            if !srcs.is_empty() {
                                start_paste_job_multi(
                                    srcs,
                                    false,
                                    false,
                                    d.clone(),
                                    si,
                                    notice,
                                    job_running,
                                    job_label,
                                    job_progress,
                                    job_cancel,
                                    clipboard,
                                );
                            }
                        }
                    })
                    .child(nav_item(FOLDER, label.clone(), false, {
                        let mut sp = second_path;
                        let si = second_items;
                        let d = dest.clone();
                        move || {
                            sp.set(d.clone());
                            load_dir(d.clone(), si);
                        }
                    }))
                    .maybe(is_hl, |el| el.child(status_chip("Drop", true, None)))
                    .into_element()
            }))
            .into_element()
    } else {
        rect().into_element()
    };

    let status_row: Element = {
        let note = notice.read().clone();
        let job = *job_running.read();
        let jl = job_label.read().clone();
        let prog = job_progress.read().clone();
        let trashed_n = pending_trash_undo_count();
        let has_op = LAST_OP.lock().map(|v| v.is_some()).unwrap_or(false);
        let _tick_op = *op_undo_tick.read();
        if job || note.is_some() || trashed_n > 0 || has_op {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .content(Content::Flex)
                .width(Size::fill())
                .padding((4., 12.))
                .maybe(job, |el| {
                    let txt = match prog.clone() {
                        Some(p) => {
                            let mut s = format!("{} {}/{} {}", p.label, p.done, p.total, p.current);
                            if !p.speed.is_empty() {
                                s.push_str(&format!(" ({})", p.speed));
                            }
                            s
                        }
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
                        .child(ghost_button("Retry", {
                            let cp = current_path;
                            let items_c = items;
                            move || load_dir(cp.read().clone(), items_c)
                        }))
                        .child(ghost_button("Reveal", {
                            let cp2 = current_path;
                            let n = notice;
                            move || show_in_folder(&cp2.read().clone(), n)
                        }))
                })
                .maybe(trashed_n > 0, |el| {
                    el.child(notice_pill(format!("{trashed_n} trashed (Undo survives restart)"), NoticeKind::Info)).child(ghost_button("Undo", {
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
                .maybe(has_op, |el| {
                    el.child(notice_pill("Copy/move can be undone", NoticeKind::Warn)).child(ghost_button("Undo op", {
                        let items_c = items;
                        let cp = current_path;
                        let mut note_st = notice;
                        let mut tick = op_undo_tick;
                        move || {
                            match undo_last_op() {
                                Ok(msg) => note_st.set(Some((msg, NoticeKind::Success))),
                                Err(e) => note_st.set(Some((format!("Undo failed: {e}"), NoticeKind::Error))),
                            }
                            tick.set(*tick.read() + 1);
                            load_dir(cp.read().clone(), items_c);
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
        let tab_val = *drawer_tab.read();
        rect()
            .width(Size::flex(0.8))
            .max_width(Size::px(340.))
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
                        small_close_button(move || sel_state.set(None))
                    }),
            )
            .child(
                rect().width(Size::fill()).content(Content::Flex).margin((0., 0., 12., 0.)).child(segmented_control_full(
                    vec![("Preview", DrawerTab::Preview), ("Details", DrawerTab::Details)],
                    tab_val,
                    {
                        let mut d = drawer_tab;
                        move |v: DrawerTab| d.set(v)
                    },
                )),
            )
            .maybe(tab_val == DrawerTab::Preview, |el| {
                el.child({
                let is_pdf_sel = !is_folder && is_pdf_name(&sel.name);
                if is_img_sel || is_pdf_sel {
                    let hit = cached_thumb_if_exists(&sel.path, sel.modified, sel.size);
                    if hit.is_none() && small_thumb_ok(sel.size, 0) {
                        request_cached_thumb(&sel.path, sel.modified, sel.size);
                    }
                    let pdf_txt = if is_pdf_sel { read_pdf_preview(&sel.path) } else { None };
                    if let Some(tp) = hit {
                        rect()
                            .vertical()
                            .width(Size::fill())
                            .content(Content::Flex)
                            .spacing(8.)
                            .margin((0., 0., 16., 0.))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::px(160.))
                                    .cross_align(Alignment::Center)
                                    .main_align(Alignment::Center)
                                    .content(Content::Flex)
                                    .corner_radius(12.)
                                    .background(t.bg_card)
                                    .overflow(Overflow::Clip)
                                    .child(ImageViewer::new(tp).width(Size::fill()).height(Size::fill())),
                            )
                            .maybe_child(pdf_txt.map(|txt| {
                                rect()
                                    .width(Size::fill())
                                    .padding(8.)
                                    .corner_radius(8.)
                                    .background(t.panel)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .child(label().font_size(12.).color(t.text).max_lines(Some(8)).text(txt))
                                    .into_element()
                            }))
                            .into_element()
                    } else if let Some(txt) = pdf_txt {
                        rect()
                            .vertical()
                            .width(Size::fill())
                            .content(Content::Flex)
                            .spacing(4.)
                            .margin((0., 0., 16., 0.))
                            .child(field_label("Preview"))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .padding(8.)
                                    .corner_radius(8.)
                                    .background(t.panel)
                                    .border(Border::new().width(1.).fill(t.border))
                                    .child(label().font_size(12.).color(t.text).max_lines(Some(12)).text(txt)),
                            )
                            .into_element()
                    } else if is_pdf_sel {
                        let p = sel.path.clone();
                        rect()
                            .vertical()
                            .width(Size::fill())
                            .content(Content::Flex)
                            .spacing(8.)
                            .margin((0., 0., 16., 0.))
                            .child(notice_pill("Preview unavailable for PDF", NoticeKind::Warn))
                            .child(tile_sub("No thumbnail yet. Install pdftoppm or open with the default app."))
                            .child(secondary_button_full("Open with default app", move || {
                                try_open_path(&p, notice);
                            }))
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
                } else if !is_folder && is_av_name(&sel.name) {
                    let p = sel.path.clone();
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .content(Content::Flex)
                        .spacing(8.)
                        .margin((0., 0., 16., 0.))
                        .child(notice_pill("Preview unavailable for audio/video", NoticeKind::Warn))
                        .child(tile_sub("Playback is not built in yet. Open with the default app."))
                        .child(secondary_button_full("Open with default app", move || {
                            try_open_path(&p, notice);
                        }))
                        .into_element()
                } else if !is_folder && read_text_preview(&sel.path).is_some() {
                    let txt = read_text_preview(&sel.path).unwrap_or_default();
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .content(Content::Flex)
                        .spacing(4.)
                        .margin((0., 0., 16., 0.))
                        .child(field_label("Preview"))
                        .child(
                            rect()
                                .width(Size::fill())
                                .padding(8.)
                                .corner_radius(8.)
                                .background(t.panel)
                                .border(Border::new().width(1.).fill(t.border))
                                .child(label().font_size(12.).color(t.text).max_lines(Some(12)).text(txt)),
                        )
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
            })
            .maybe(tab_val == DrawerTab::Details, |el| {
                el.child(
                rect()
                    .vertical()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .spacing(0.)
                    .child(
                        label()
                            .font_size(16.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_primary)
                            .margin((0., 0., 4., 0.))
                            .text(sel.name.clone()),
                    )
                    .child(label().font_size(12.).color(t.text_muted).margin((0., 0., 12., 0.)).text(format_size(sel.size)))
                    .child(
                        rect()
                            .margin((0., 0., 12., 0.))
                            .child(field_label("Path"))
                            .child(label().font_size(12.).color(t.text_primary).text(sel.path.clone())),
                    )
                    .child(
                        rect()
                            .margin((0., 0., 12., 0.))
                            .child(field_label("Type"))
                            .child(label().font_size(12.).color(t.text_primary).text(file_type_label(&sel.name, is_folder))),
                    )
                    .child(
                        rect()
                            .margin((0., 0., 12., 0.))
                            .child(field_label("Modified"))
                            .child(label().font_size(12.).color(t.text_primary).text(format_modified(sel.modified))),
                    )
                    .child(
                        rect()
                            .margin((0., 0., 12., 0.))
                            .child(field_label("Permissions"))
                            .child(label().font_size(12.).color(t.text_primary).text(format_perms(&sel.path))),
                    )
                    .maybe(
                        clipboard.read().clone().is_some_and(|(paths, is_cut)| is_cut && paths.contains(&sel.path)),
                        |inner| inner.child(status_chip("Cut", true, None)),
                    )
                    .into_element(),
            )
            })
            .child({
                let s_path = sel.path.clone();
                let mut path_state = current_path;
                let items_state = items;
                let open_notice = notice;
                let mut rf = recent_files;
                rect()
                    .margin((12., 0., 0., 0.))
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(primary_button(if is_folder { "Open Directory" } else { "Open File" }, move || {
                        if is_folder {
                            path_state.set(s_path.clone());
                            load_dir(s_path.clone(), items_state);
                        } else {
                            record_recent_file(&s_path);
                            rf.set(load_recent_files());
                            try_open_path(&s_path, open_notice);
                        }
                    }))
                    .into_element()
            })
            .child({
                let s_name = sel.name.clone();
                let s_path = sel.path.clone();
                let is_star = starred.read().iter().any(|(_, p)| p == &s_path);
                rect()
                    .margin((8., 0., 0., 0.))
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(secondary_button_full(
                        if is_star { "Unstar" } else { "Star" },
                        {
                            let mut st = starred;
                            move || {
                                let mut v = st.read().clone();
                                if v.iter().any(|(_, p)| p == &s_path) {
                                    v.retain(|(_, p)| p != &s_path);
                                } else {
                                    v.push((s_name.clone(), s_path.clone()));
                                }
                                save_starred(&v);
                                st.set(v);
                            }
                        },
                    ))
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
                        .width(Size::fill())
                        .content(Content::Flex)
                        .child(secondary_button_full(
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
                    .width(Size::fill())
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
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(secondary_button_full("Open-With", {
                        let p = s_path.clone();
                        let n = notice;
                        move || open_with_chooser(&p, n)
                    })))
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(secondary_button_full(
                        "Show in folder",
                        {
                            let p = s_path2.clone();
                            let n = notice;
                            move || show_in_folder(&p, n)
                        },
                    )))
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
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(secondary_button_full("Rename", {
                        let name_c2 = name_c.clone();
                        move || {
                            ri.set(name_c2.clone());
                            rt.set(Some(sel.clone()));
                            sr.set(true);
                        }
                    })))
                    .child(rect().width(Size::flex(1.)).content(Content::Flex).child(danger_button_full_local("Delete", {
                        let s = s_path.clone();
                        let mut pend = pending_delete;
                        let mut show = show_delete_confirm;
                        move || {
                            let mut paths = selected_paths.read().clone();
                            if !paths.contains(&s) {
                                paths.push(s.clone());
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
        rect().into_element()
    };

    let perm_delete_modal: Element = if *show_perm_delete.read() {
        let n = pending_perm_delete.read().len();
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                dialog_card(420.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text).text("Delete permanently"))
                    .child(tile_sub(format!(
                        "{n} Trash item{} will be destroyed now. This cannot be undone.",
                        if n == 1 { "" } else { "s" }
                    )))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(ghost_button("Cancel", {
                                let mut s = show_perm_delete;
                                let mut p = pending_perm_delete;
                                move || {
                                    p.set(Vec::new());
                                    s.set(false);
                                }
                            }))
                            .child(danger_button("Delete now", {
                                let mut s = show_perm_delete;
                                let mut p = pending_perm_delete;
                                let items_c = items;
                                let cp_c = current_path;
                                let mut note = notice;
                                let mut tm = trash_orig;
                                move || {
                                    let names = p.read().clone();
                                    let mut ok = 0usize;
                                    let mut err: Option<String> = None;
                                    for tn in &names {
                                        match delete_trash_one(tn) {
                                            Ok(()) => ok += 1,
                                            Err(e) => {
                                                err = Some(e);
                                                break;
                                            }
                                        }
                                    }
                                    p.set(Vec::new());
                                    s.set(false);
                                    tm.set(trash_orig_map());
                                    if let Some(e) = err {
                                        note.set(Some((format!("Delete failed: {e}"), NoticeKind::Error)));
                                    } else {
                                        note.set(Some((
                                            format!("Deleted {ok} item{} permanently", if ok == 1 { "" } else { "s" }),
                                            NoticeKind::Success,
                                        )));
                                    }
                                    load_dir(cp_c.read().clone(), items_c);
                                }
                            })),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
    };

    let restore_conflict_modal: Element = if *show_restore_conflict.read() {
        let dest = restore_conflict_path.read().clone();
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                dialog_card(420.)
                    .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text).text("Origin already exists"))
                    .child(tile_sub(format!("{dest} exists. Overwrite it, keep both with a new name, or skip.")))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(ghost_button("Skip", {
                                let mut s = show_restore_conflict;
                                let mut pr = pending_restore;
                                move || {
                                    pr.set(None);
                                    s.set(false);
                                }
                            }))
                            .child(secondary_button("Keep both", {
                                let mut s = show_restore_conflict;
                                let mut pr = pending_restore;
                                let items_c = items;
                                let cp_c = current_path;
                                let mut note = notice;
                                let mut tm = trash_orig;
                                move || {
                                    if let Some(tn) = pr.read().clone() {
                                        let (files, info) = trash_dirs();
                                        let src = files.join(&tn);
                                        let info_path = info.join(format!("{tn}.trashinfo"));
                                        let (orig, _) = parse_trashinfo(&info_path)
                                            .map(|(o, d)| (percent_decode(&o), d))
                                            .unwrap_or((String::new(), String::new()));
                                        if !orig.is_empty() {
                                            let p = PathBuf::from(&orig);
                                            let parent = p.parent().map(|x| x.to_path_buf()).unwrap_or(PathBuf::from("/"));
                                            let name =
                                                p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or(tn.clone());
                                            let alt = unique_dest(&parent, &name);
                                            match move_path(&src, &alt) {
                                                Ok(()) => {
                                                    let _ = std::fs::remove_file(&info_path);
                                                    note.set(Some((
                                                        format!("Restored as {}", alt.to_string_lossy()),
                                                        NoticeKind::Success,
                                                    )));
                                                }
                                                Err(e) => {
                                                    note.set(Some((format!("Restore failed: {e}"), NoticeKind::Error)))
                                                }
                                            }
                                            tm.set(trash_orig_map());
                                            load_dir(cp_c.read().clone(), items_c);
                                        }
                                    }
                                    pr.set(None);
                                    s.set(false);
                                }
                            }))
                            .child(danger_button("Overwrite", {
                                let mut s = show_restore_conflict;
                                let mut pr = pending_restore;
                                let items_c = items;
                                let cp_c = current_path;
                                let mut note = notice;
                                let mut tm = trash_orig;
                                move || {
                                    if let Some(tn) = pr.read().clone() {
                                        match restore_trash_one(&tn, true) {
                                            Ok(dest) => note.set(Some((
                                                format!("Restored to {}", dest.to_string_lossy()),
                                                NoticeKind::Success,
                                            ))),
                                            Err(e) => note.set(Some((format!("Restore failed: {e}"), NoticeKind::Error))),
                                        }
                                        tm.set(trash_orig_map());
                                        load_dir(cp_c.read().clone(), items_c);
                                    }
                                    pr.set(None);
                                    s.set(false);
                                }
                            })),
                    ),
            )
            .into_element()
    } else {
        rect().into_element()
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

    let connect_modal: Element = if *show_connect.read() {
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(120, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .child(
                dialog_card(420.)
                    .child(tile_head(Some(GLOBE), "Connect to server", None::<String>))
                    .child(tile_sub("Enter sftp:// or smb:// address. Full browsing is unavailable; the location opens with the system handler."))
                    .child(Input::new(connect_uri).width(Size::fill()).placeholder("sftp://host/share"))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .main_align(Alignment::End)
                            .content(Content::Flex)
                            .child(ghost_button("Cancel", {
                                let mut s = show_connect;
                                move || s.set(false)
                            }))
                            .child(secondary_button("Open", {
                                let uri = connect_uri;
                                let mut s = show_connect;
                                let note = notice;
                                move || {
                                    let v = uri.read().clone();
                                    open_network_uri(&v, note);
                                    s.set(false);
                                }
                            })),
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
                                    .padding((8., 10.))
                                    .corner_radius(8.)
                                    .background(t.bg_card)
                                    .cursor(CursorIcon::Pointer)
                                    .on_press({
                                        let mut s = show_overwrite;
                                        let mut p = pending_paste;
                                        let items_c = items;
                                        let mut note = notice;
                                        move |_| {
                                            if let Some(pv) = p.read().clone() {
                                                let mut renamed: Vec<String> = Vec::new();
                                                for src in &pv.srcs {
                                                    let name = PathBuf::from(src)
                                                        .file_name()
                                                        .map(|y| y.to_string_lossy().to_string())
                                                        .unwrap_or("file".to_string());
                                                    let dest = PathBuf::from(&pv.dest_dir).join(&name);
                                                    if dest.exists() {
                                                        let parent = PathBuf::from(&pv.dest_dir);
                                                        let alt = unique_dest(&parent, &name);
                                                        let alt_str = alt.to_string_lossy().to_string();
                                                        if pv.is_cut {
                                                            match move_path(Path::new(src), &alt) {
                                                                Ok(()) => renamed.push(alt_str),
                                                                Err(e) => note.set(Some((
                                                                    format!("Rename move failed: {e}"),
                                                                    NoticeKind::Error,
                                                                ))),
                                                            }
                                                        } else if PathBuf::from(src).is_dir() {
                                                            match copy_dir_recursive(&PathBuf::from(src), &alt) {
                                                                Ok(()) => renamed.push(alt_str),
                                                                Err(e) => note.set(Some((
                                                                    format!("Rename copy failed: {e}"),
                                                                    NoticeKind::Error,
                                                                ))),
                                                            }
                                                        } else {
                                                            match std::fs::copy(src, &alt) {
                                                                Ok(_) => renamed.push(alt_str),
                                                                Err(e) => note.set(Some((
                                                                    format!("Rename copy failed: {e}"),
                                                                    NoticeKind::Error,
                                                                ))),
                                                            }
                                                        }
                                                    } else {
                                                        renamed.push(src.clone());
                                                    }
                                                }
                                                if !renamed.is_empty() {
                                                    start_paste_job_multi(
                                                        renamed,
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
                                    .child(label().font_size(12.).color(t.text_primary).text("Rename new")),
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
                                    try_open_path(&it.path, notice);
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
                        .child(tab_bar)
                        .child(crumb_bar)
                        .child(tool_bar)
                        .child(second_bar)
                        .child(filter_bar)
                        .child(trash_bar)
                        .child(status_row)
                        .maybe(*drop_highlight.read(), |el| {
                            el.child(
                                rect()
                                    .width(Size::fill())
                                    .padding((4., 12.))
                                    .content(Content::Flex)
                                    .child(notice_pill("Drop files to copy here", NoticeKind::Info)),
                            )
                        })
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::fill())
                                .content(Content::Flex)
                                .on_file_drop({
                                    let dest = current_path.read().clone();
                                    let items_c = items;
                                    move |e: Event<FileEventData>| {
                                        let srcs: Vec<String> =
                                            e.file_paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
                                        if !srcs.is_empty() {
                                            start_paste_job_multi(
                                                srcs,
                                                false,
                                                false,
                                                dest.clone(),
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
                                })
                                .on_global_file_hover({
                                    let mut h = drop_highlight;
                                    move |_| h.set(true)
                                })
                                .on_global_file_hover_cancelled({
                                    let mut h = drop_highlight;
                                    move |_| h.set(false)
                                })
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
                                .child(content_view)
                                .maybe(second_on, |el| {
                                    el.child(
                                        rect()
                                            .width(Size::fill())
                                            .height(Size::fill())
                                            .content(Content::Flex)
                                            .padding((0., 0., 0., 8.))
                                            .child(second_pane),
                                    )
                                }),
                        )
                        .child(bottom_bar),
                )
                .maybe_child(selected_item.read().is_some().then(|| drawer)),
        )
        .child(add_place_modal)
        .child(connect_modal)
        .child(rename_modal)
        .child(delete_modal)
        .child(overwrite_modal)
        .child(perm_delete_modal)
        .child(restore_conflict_modal)
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
    let mut cp = *current_path;
    let tp = target_path.clone();
    let is_active = *cp.read() == tp;
    let name_owned = name.to_string();
    let tp_for_menu = tp.clone();
    let is_pinned = pinned.read().iter().any(|(_, p)| p == &tp_for_menu);
    let ctx = Menu::new()
        .child(ctx_button("Open", {
            let tp2 = tp_for_menu.clone();
            let mut cp2 = cp;
            move || {
                cp2.set(tp2.clone());
                load_dir(tp2.clone(), items_state);
            }
        }))
        .child(ctx_button(if is_pinned { "Unpin" } else { "Pin" }, {
            let tp2 = tp_for_menu.clone();
            let name2 = name_owned.clone();
            let mut pin = pinned;
            move || {
                let mut v = pin.read().clone();
                if v.iter().any(|(_, p)| p == &tp2) {
                    v.retain(|(_, p)| p != &tp2);
                } else {
                    v.push((name2.clone(), tp2.clone()));
                }
                save_pinned(&v);
                pin.set(v);
            }
        }));
    rect()
        .width(Size::fill())
        .on_secondary_down(move |_| ContextMenu::open_from_down(ctx.clone()))
        .child(nav_item(icon_svg, name_owned.clone(), is_active, move || {
            cp.set(tp.clone());
            load_dir(tp.clone(), items_state);
        }))
        .into_element()
}

fn sidebar_open_entry(
    name: &str,
    icon_svg: &'static str,
    current_path: &State<String>,
    target_path: String,
    items_state: State<Vec<Item>>,
    notice: State<Option<(String, NoticeKind)>>,
    recent_files: State<Vec<String>>,
) -> Element {
    let mut cp = *current_path;
    let tp = target_path.clone();
    let is_active = *cp.read() == tp;
    let name_owned = name.to_string();
    let tp2 = tp.clone();
    rect()
        .width(Size::fill())
        .child(nav_item(icon_svg, name_owned, is_active, move || {
            if Path::new(&tp2).is_file() {
                let mut rf = recent_files;
                record_recent_file(&tp2);
                rf.set(load_recent_files());
                try_open_path(&tp2, notice);
            } else if is_network_uri(&tp2) {
                open_network_uri(&tp2, notice);
            } else {
                cp.set(tp2.clone());
                load_dir(tp2.clone(), items_state);
            }
        }))
        .into_element()
}

fn main() {
    let args = config::LaunchArgs::parse();
    if args.wants_help() {
        args.print_help("files", FILES_PAGE_SLUGS);
        return;
    }
    let initial_path = resolve_initial_path(&args);
    if let Some(p) = initial_path.as_deref() {
        if !Path::new(p).exists() {
            eprintln!("[files] path does not exist: {p}");
        }
    } else if let Some(page) = args.page.as_deref() {
        eprintln!("[files] unknown page {page:?}, opening home directory");
    }
    args.maybe_arm_screenshot("files");

    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(FilesApp { initial_path }).with_title("Files").with_size(1100., 700.).with_min_size(640., 480.),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, is_dir: bool, modified: Option<u64>) -> Item {
        Item {
            ty: if is_dir { ItemType::Folder } else { ItemType::File },
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            size: 1024,
            modified,
        }
    }

    #[test]
    fn type_filter_labels_images_docs_code() {
        assert_eq!(type_label_for_filter("photo.jpg", false), TypeFilter::Images);
        assert_eq!(type_label_for_filter("notes.md", false), TypeFilter::Docs);
        assert_eq!(type_label_for_filter("main.rs", false), TypeFilter::Code);
        assert_eq!(type_label_for_filter("clip.mp4", false), TypeFilter::Video);
        assert_eq!(type_label_for_filter("song.mp3", false), TypeFilter::Audio);
        assert_eq!(type_label_for_filter("backup.zip", false), TypeFilter::Archives);
        assert_eq!(type_label_for_filter("weird.xyz", false), TypeFilter::All);
    }

    #[test]
    fn folders_pass_type_filter() {
        let dir = item("somedir", true, None);
        assert!(matches_type_filter(&dir, TypeFilter::Images));
        let img = item("a.png", false, None);
        assert!(matches_type_filter(&img, TypeFilter::Images));
        assert!(!matches_type_filter(&img, TypeFilter::Docs));
    }

    #[test]
    fn date_filter_windows() {
        let now = chrono::Local::now().timestamp() as u64;
        let old = now.saturating_sub(40 * 24 * 3600);
        let recent = item("r.txt", false, Some(now));
        let stale = item("o.txt", false, Some(old));
        assert!(matches_date_filter(&recent, DateFilter::Today));
        assert!(matches_date_filter(&recent, DateFilter::Week));
        assert!(!matches_date_filter(&stale, DateFilter::Today));
        assert!(!matches_date_filter(&stale, DateFilter::Week));
        assert!(!matches_date_filter(&stale, DateFilter::Month));
        assert!(matches_date_filter(&stale, DateFilter::Any));
        let missing = item("m.txt", false, None);
        assert!(!matches_date_filter(&missing, DateFilter::Today));
        assert!(matches_date_filter(&missing, DateFilter::Any));
    }

    #[test]
    fn thumb_gates_size_and_index() {
        assert!(small_thumb_ok(1024, 0));
        assert!(!small_thumb_ok(0, 0));
        assert!(!small_thumb_ok(THUMB_MAX_FILE_BYTES + 1, 0));
        assert!(!small_thumb_ok(1024, 60));
        assert_eq!(thumb_key("/a", Some(1), 2), thumb_key("/a", Some(1), 2));
        assert_ne!(thumb_key("/a", Some(1), 2), thumb_key("/b", Some(1), 2));
        assert!(cached_thumb_if_exists("/nope-missing", Some(1), 1024).is_none());
        assert!(cached_thumb_if_exists("/tmp/x.png", Some(1), 0).is_none());
    }

    #[test]
    fn text_preview_gates_and_pdf_names() {
        assert!(is_text_previewable("notes.md"));
        assert!(is_text_previewable("main.rs"));
        assert!(!is_text_previewable("photo.png"));
        assert!(!is_text_previewable("movie.mp4"));
        assert!(is_pdf_name("doc.PDF"));
        assert!(!is_pdf_name("doc.png"));
        assert!(is_av_name("clip.mp4"));
        assert!(!is_av_name("notes.txt"));
        assert!(read_text_preview("/definitely/missing-file-xyz.txt").is_none());
        assert!(read_pdf_preview("/definitely/missing-file-xyz.pdf").is_none());
    }

    #[test]
    fn text_preview_reads_first_chunk_of_large_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("finick_large_preview_test.txt");
        let big = "a".repeat(200 * 1024);
        std::fs::write(&path, big).unwrap();
        let preview = read_text_preview(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        let txt = preview.expect("large text file shows first N KB");
        assert!(txt.chars().count() <= TEXT_PREVIEW_MAX_CHARS + 1);
        assert!(txt.ends_with("…"));
    }

    #[test]
    fn unique_dest_never_returns_existing_path() {
        let base = std::env::temp_dir().join(format!("finick_unique_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let existing = base.join("doc.txt");
        std::fs::write(&existing, "x").unwrap();
        let out = unique_dest(&base, "doc.txt");
        assert_ne!(out, existing);
        assert!(!out.exists());
        let _ = std::fs::remove_file(&existing);
        let _ = std::fs::remove_dir(&base);
    }

    #[test]
    fn dedupe_restore_dest_avoids_silent_overwrite() {
        let base = std::env::temp_dir().join(format!("finick_dedupe_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let orig = base.join("keep.txt");
        std::fs::write(&orig, "orig").unwrap();
        let out = dedupe_restore_dest(orig.to_str().unwrap(), "keep.txt");
        assert_ne!(out, orig);
        assert!(!out.exists());
        let missing = base.join("gone.txt");
        assert_eq!(dedupe_restore_dest(missing.to_str().unwrap(), "gone.txt"), missing);
        let _ = std::fs::remove_file(&orig);
        let _ = std::fs::remove_dir(&base);
    }

    #[test]
    fn trash_undo_file_roundtrip_filters_blanks() {
        let path = std::env::temp_dir().join(format!("finick_trashundo_{}.json", std::process::id()));
        let pairs = vec![
            ("a.1".to_string(), "/tmp/a".to_string()),
            ("".to_string(), "/tmp/b".to_string()),
            ("c.1".to_string(), "".to_string()),
        ];
        assert!(save_trash_undo_to(&path, &pairs));
        let loaded = load_trash_undo_from(&path);
        assert_eq!(loaded, vec![("a.1".to_string(), "/tmp/a".to_string())]);
        let _ = std::fs::remove_file(&path);
        assert!(load_trash_undo_from(&path).is_empty());
    }

    #[test]
    fn internal_drop_validation_rejects_bad_targets() {
        let base = std::env::temp_dir().join(format!("finick_drop_{}", std::process::id()));
        let src_dir = base.join("src");
        let dest_dir = base.join("dest");
        let _ = std::fs::create_dir_all(&src_dir);
        let _ = std::fs::create_dir_all(&dest_dir);
        let src_file = src_dir.join("f.txt");
        std::fs::write(&src_file, "x").unwrap();
        let src = src_file.to_string_lossy().to_string();
        let dest = dest_dir.to_string_lossy().to_string();
        assert!(internal_drop_error(&[], &dest).is_some());
        assert!(internal_drop_error(&[src.clone()], &dest).is_none());
        assert!(internal_drop_error(&[src.clone()], &src_dir.to_string_lossy().to_string()).is_some());
        assert!(
            internal_drop_error(&[src_dir.to_string_lossy().to_string()], &src_dir.to_string_lossy().to_string())
                .is_some()
        );
        let nested = dest_dir.join("inner");
        let _ = std::fs::create_dir_all(&nested);
        assert!(internal_drop_error(&[dest.clone()], &nested.to_string_lossy().to_string()).is_some());
        assert!(internal_drop_error(&[src.clone()], "/definitely/not/a/dir/xyz".into()).is_some());
        assert!(internal_drop_error(&["/definitely/missing/src".to_string()], &dest).is_some());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn restore_single_never_overwrites_silently() {
        assert!(restore_trash_one("finick-missing-entry-xyz", false).is_err());
        let err = restore_trash_one("finick-missing-entry-xyz", false).unwrap_err();
        assert!(err.contains("not in Trash"));
    }

    #[test]
    fn copy_speed_is_honest_on_empty_or_early() {
        assert!(format_speed(0, 5.0).is_empty());
        assert!(format_speed(1024, 0.05).is_empty());
        assert!(format_speed(2048, 1.0).contains("/s"));
    }

    #[test]
    fn dir_byte_size_sums_files() {
        let base = std::env::temp_dir().join(format!("finick_dirsize_{}", std::process::id()));
        let _ = std::fs::create_dir_all(base.join("sub"));
        std::fs::write(base.join("a.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(base.join("sub").join("b.bin"), vec![0u8; 50]).unwrap();
        assert_eq!(dir_byte_size(&base), 150);
        assert_eq!(dir_byte_size(&base.join("a.bin")), 100);
        assert_eq!(dir_byte_size(&base.join("missing-xyz")), 0);
        let _ = std::fs::remove_dir_all(&base);
    }
}
