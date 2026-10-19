use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    Screenshot,
    Screencast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureTarget {
    FullScreen,
    Monitor(String),
    Region,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOptions {
    pub delay_seconds: u32,
    pub include_cursor: bool,
    pub record_audio: bool,
    pub copy_to_clipboard: bool,
    pub custom_output_dir: Option<PathBuf>,
    pub custom_filename: Option<String>,
    pub accent_hex: Option<String>,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            delay_seconds: 0,
            include_cursor: false,
            record_audio: false,
            copy_to_clipboard: true,
            custom_output_dir: None,
            custom_filename: None,
            accent_hex: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub success: bool,
    pub file_path: Option<PathBuf>,
    pub copied_to_clipboard: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStatus {
    pub pid: u32,
    pub start_timestamp: u64,
    pub duration_secs: u64,
    pub file_path: PathBuf,
    pub target: String,
    pub tool: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredRecordingState {
    pub pid: u32,
    pub start_timestamp: u64,
    pub file_path: PathBuf,
    pub target: String,
    pub tool: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaptureToolsAvailability {
    pub has_grim: bool,
    pub has_slurp: bool,
    pub has_wl_screenrec: bool,
    pub has_wf_recorder: bool,
    pub has_wl_copy: bool,
    pub has_hyprctl: bool,
}

pub fn find_tool(name: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let standard_dirs = ["/usr/bin", "/usr/local/bin", "/bin"];
    for dir in standard_dirs {
        let candidate = Path::new(dir).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(entries) = fs::read_dir("/nix/store") {
        let pattern1 = format!("-{}-", name);
        let pattern2 = format!("-{}", name);
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.contains(&pattern1) || file_name.ends_with(&pattern2) {
                let bin = entry.path().join("bin").join(name);
                if bin.is_file() {
                    return Some(bin);
                }
            }
        }
    }

    None
}

pub fn check_tools() -> CaptureToolsAvailability {
    CaptureToolsAvailability {
        has_grim: find_tool("grim").is_some(),
        has_slurp: find_tool("slurp").is_some(),
        has_wl_screenrec: find_tool("wl-screenrec").is_some(),
        has_wf_recorder: find_tool("wf-recorder").is_some(),
        has_wl_copy: find_tool("wl-copy").is_some(),
        has_hyprctl: find_tool("hyprctl").is_some(),
    }
}

pub fn default_screenshot_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_PICTURES_DIR") {
        let p = PathBuf::from(dir).join("Screenshots");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join("Pictures").join("Screenshots");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let p = PathBuf::from("/tmp/Screenshots");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn default_screencast_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_VIDEOS_DIR") {
        let p = PathBuf::from(dir).join("Screencasts");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join("Videos").join("Screencasts");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let p = PathBuf::from("/tmp/Screencasts");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn generate_screenshot_filename() -> String {
    let now = chrono::Local::now();
    format!("Screenshot_{}.png", now.format("%Y-%m-%d_%H-%M-%S"))
}

pub fn generate_screencast_filename() -> String {
    let now = chrono::Local::now();
    format!("Screencast_{}.mp4", now.format("%Y-%m-%d_%H-%M-%S"))
}

pub fn send_notification(summary: &str, body: &str) {
    let _ = Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
            "susssasa{sv}i",
            "finick-capture",
            "0",
            "camera-photo",
            summary,
            body,
            "0",
            "0",
            "4000",
        ])
        .status();
}

pub fn copy_image_to_clipboard(path: &Path) -> Result<(), String> {
    if let Some(wl_copy) = find_tool("wl-copy") {
        let file = fs::File::open(path).map_err(|e| format!("Failed to read file for clipboard: {e}"))?;
        let status = Command::new(wl_copy)
            .args(["--type", "image/png"])
            .stdin(Stdio::from(file))
            .status()
            .map_err(|e| format!("Failed to run wl-copy: {e}"))?;
        if status.success() {
            return Ok(());
        }
    }
    Err("wl-copy not available or failed".to_string())
}

pub fn get_region_geometry(accent_hex: Option<&str>) -> Result<Option<String>, String> {
    let slurp = find_tool("slurp").ok_or_else(|| "slurp is required for region selection but was not found".to_string())?;

    let mut cmd = Command::new(slurp);
    cmd.arg("-d");

    if let Some(raw_hex) = accent_hex {
        let hex = raw_hex.trim().trim_start_matches('#');
        if hex.len() == 6 {
            cmd.args(["-b", "#00000033"]);
            cmd.args(["-c", &format!("#{hex}ff")]);
            cmd.args(["-s", &format!("#{hex}22")]);
            cmd.args(["-B", &format!("#{hex}55")]);
            cmd.args(["-w", "2"]);
        }
    }

    let output = cmd.output().map_err(|e| format!("Failed to run slurp: {e}"))?;
    if !output.status.success() {
        return Ok(None);
    }

    let geom = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if geom.is_empty() {
        Ok(None)
    } else {
        Ok(Some(geom))
    }
}

pub fn take_screenshot(target: &CaptureTarget, options: &CaptureOptions) -> Result<CaptureResult, String> {
    if options.delay_seconds > 0 {
        std::thread::sleep(std::time::Duration::from_secs(options.delay_seconds as u64));
    }

    let grim = find_tool("grim").ok_or_else(|| "grim is required for screenshots but was not found".to_string())?;

    let region_geom = if *target == CaptureTarget::Region {
        match get_region_geometry(options.accent_hex.as_deref())? {
            Some(g) => Some(g),
            None => {
                return Ok(CaptureResult {
                    success: false,
                    file_path: None,
                    copied_to_clipboard: false,
                    message: "Region selection cancelled".to_string(),
                });
            }
        }
    } else {
        None
    };

    let out_dir = options.custom_output_dir.clone().unwrap_or_else(default_screenshot_dir);
    let _ = fs::create_dir_all(&out_dir);
    let filename = options.custom_filename.clone().unwrap_or_else(generate_screenshot_filename);
    let file_path = out_dir.join(filename);

    let mut cmd = Command::new(grim);

    if options.include_cursor {
        cmd.arg("-c");
    }

    match target {
        CaptureTarget::FullScreen => {}
        CaptureTarget::Monitor(name) => {
            cmd.args(["-o", name]);
        }
        CaptureTarget::Region => {
            if let Some(geom) = &region_geom {
                cmd.args(["-g", geom]);
            }
        }
    }

    cmd.arg(&file_path);

    let status = cmd.status().map_err(|e| format!("Failed to execute grim: {e}"))?;
    if !status.success() {
        return Err(format!("grim exited with status {status}"));
    }

    if !file_path.exists() || fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0) == 0 {
        return Err("Screenshot file was not created or is empty".to_string());
    }

    let mut copied = false;
    if options.copy_to_clipboard {
        if copy_image_to_clipboard(&file_path).is_ok() {
            copied = true;
        }
    }

    send_notification(
        "Screenshot Captured",
        &format!("Saved to {}", file_path.display()),
    );

    Ok(CaptureResult {
        success: true,
        file_path: Some(file_path),
        copied_to_clipboard: copied,
        message: "Screenshot captured successfully".to_string(),
    })
}

fn recording_state_file_path() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from("/tmp").join(format!("finick_screencast_{uid}.json"))
}

fn is_process_running(pid: u32) -> bool {
    let proc_path = PathBuf::from("/proc").join(pid.to_string());
    if proc_path.exists() {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    } else {
        false
    }
}

pub fn get_active_screencast() -> Option<RecordingStatus> {
    let state_file = recording_state_file_path();
    if !state_file.exists() {
        return None;
    }

    let content = fs::read_to_string(&state_file).ok()?;
    let stored: StoredRecordingState = serde_json::from_str(&content).ok()?;

    if is_process_running(stored.pid) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let duration = now.saturating_sub(stored.start_timestamp);
        Some(RecordingStatus {
            pid: stored.pid,
            start_timestamp: stored.start_timestamp,
            duration_secs: duration,
            file_path: stored.file_path,
            target: stored.target,
            tool: stored.tool,
        })
    } else {
        let _ = fs::remove_file(&state_file);
        None
    }
}

pub fn start_screencast(target: &CaptureTarget, options: &CaptureOptions) -> Result<RecordingStatus, String> {
    if let Some(active) = get_active_screencast() {
        return Err(format!("A screencast is already recording (PID {})", active.pid));
    }

    let rec_tool = if let Some(p) = find_tool("wl-screenrec") {
        ("wl-screenrec", p)
    } else if let Some(p) = find_tool("wf-recorder") {
        ("wf-recorder", p)
    } else {
        return Err("No screencast recorder found (install wl-screenrec or wf-recorder)".to_string());
    };

    let region_geom = if *target == CaptureTarget::Region {
        match get_region_geometry(options.accent_hex.as_deref())? {
            Some(g) => Some(g),
            None => return Err("Region selection cancelled".to_string()),
        }
    } else {
        None
    };

    let out_dir = options.custom_output_dir.clone().unwrap_or_else(default_screencast_dir);
    let _ = fs::create_dir_all(&out_dir);
    let filename = options.custom_filename.clone().unwrap_or_else(generate_screencast_filename);
    let file_path = out_dir.join(filename);

    let mut cmd = Command::new(&rec_tool.1);

    let target_name = match target {
        CaptureTarget::FullScreen => "Full Screen".to_string(),
        CaptureTarget::Monitor(name) => format!("Monitor: {name}"),
        CaptureTarget::Region => "Selected Region".to_string(),
    };

    if rec_tool.0 == "wl-screenrec" {
        cmd.args(["-f", &file_path.to_string_lossy()]);
        cmd.arg("--low-power=off");

        if !options.include_cursor {
            cmd.arg("--no-cursor");
        }
        if options.record_audio {
            cmd.arg("--audio");
        }

        match target {
            CaptureTarget::FullScreen => {}
            CaptureTarget::Monitor(mon) => {
                cmd.args(["-o", mon]);
            }
            CaptureTarget::Region => {
                if let Some(geom) = &region_geom {
                    cmd.args(["-g", geom]);
                }
            }
        }
    } else {
        cmd.args(["-f", &file_path.to_string_lossy()]);
        if options.record_audio {
            cmd.arg("-a");
        }
        match target {
            CaptureTarget::FullScreen => {}
            CaptureTarget::Monitor(mon) => {
                cmd.args(["-o", mon]);
            }
            CaptureTarget::Region => {
                if let Some(geom) = &region_geom {
                    cmd.args(["-g", geom]);
                }
            }
        }
    }

    cmd.stdout(Stdio::null()).stderr(Stdio::null());

    let child = cmd.spawn().map_err(|e| format!("Failed to spawn {}: {e}", rec_tool.0))?;
    let pid = child.id();

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let state = StoredRecordingState {
        pid,
        start_timestamp: now,
        file_path: file_path.clone(),
        target: target_name.clone(),
        tool: rec_tool.0.to_string(),
    };

    let state_file = recording_state_file_path();
    let _ = fs::write(&state_file, serde_json::to_string(&state).unwrap_or_default());

    send_notification(
        "Recording Started",
        &format!("Recording {} to {}", target_name, file_path.file_name().and_then(|f| f.to_str()).unwrap_or("")),
    );

    Ok(RecordingStatus {
        pid,
        start_timestamp: now,
        duration_secs: 0,
        file_path,
        target: target_name,
        tool: rec_tool.0.to_string(),
    })
}

pub fn stop_screencast() -> Result<CaptureResult, String> {
    let active = match get_active_screencast() {
        Some(a) => a,
        None => return Err("No active screencast in progress".to_string()),
    };

    let state_file = recording_state_file_path();
    let _ = fs::remove_file(&state_file);

    unsafe {
        libc::kill(active.pid as i32, libc::SIGINT);
    }

    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !is_process_running(active.pid) {
            break;
        }
    }

    if is_process_running(active.pid) {
        unsafe {
            libc::kill(active.pid as i32, libc::SIGTERM);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    let file_exists = active.file_path.exists() && fs::metadata(&active.file_path).map(|m| m.len()).unwrap_or(0) > 0;

    send_notification(
        "Recording Saved",
        &format!("Saved to {}", active.file_path.display()),
    );

    Ok(CaptureResult {
        success: file_exists,
        file_path: Some(active.file_path),
        copied_to_clipboard: false,
        message: if file_exists {
            "Screencast recording completed successfully".to_string()
        } else {
            "Recording ended but output file was not found".to_string()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_check() {
        let tools = check_tools();
        assert!(tools.has_grim);
        assert!(tools.has_slurp);
        assert!(tools.has_wl_screenrec);
    }

    #[test]
    fn test_filename_generation() {
        let sc_name = generate_screenshot_filename();
        assert!(sc_name.starts_with("Screenshot_"));
        assert!(sc_name.ends_with(".png"));

        let vid_name = generate_screencast_filename();
        assert!(vid_name.starts_with("Screencast_"));
        assert!(vid_name.ends_with(".mp4"));
    }

    #[test]
    fn test_directories() {
        let pic_dir = default_screenshot_dir();
        assert!(pic_dir.exists());

        let vid_dir = default_screencast_dir();
        assert!(vid_dir.exists());
    }
}
