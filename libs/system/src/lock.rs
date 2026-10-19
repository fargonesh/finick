use std::process::Command;

/// Preferred lock binaries, in order: the ext-session-lock client first,
/// then the Freya window-layer locker kept as the `finished`-event fallback.
const LOCK_BINARIES: [&str; 2] = ["finick-lock", "locker"];

fn join_if_file(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    for name in LOCK_BINARIES {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

pub fn locker_binary() -> Option<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("FINICK_LOCKER_BIN").map(std::path::PathBuf::from) {
        if dir.is_file() {
            return Some(dir);
        }
        if let Some(found) = join_if_file(&dir) {
            return Some(found);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Some(found) = join_if_file(dir) {
                return Some(found);
            }
        }
    for base in [
        "/run/current-system/sw/bin/finick-lock",
        "/run/current-system/sw/bin/locker",
        "/usr/local/bin/finick-lock",
        "/usr/local/bin/locker",
        "/usr/bin/finick-lock",
        "/usr/bin/locker",
    ] {
        let cand = std::path::PathBuf::from(base);
        if cand.is_file() {
            return Some(cand);
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        for base in [".cargo/bin", ".local/bin"] {
            if let Some(found) = join_if_file(&home.join(base)) {
                return Some(found);
            }
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .flat_map(|d| LOCK_BINARIES.iter().map(move |n| d.join(*n)))
            .find(|p| p.is_file())
    })
}

fn spawn_locker(locker: &Option<std::path::PathBuf>) {
    if let Some(exe) = locker {
        match Command::new(exe).spawn() {
            Ok(_) => return,
            Err(e) => eprintln!("[finick] failed to spawn locker at {}: {e}", exe.display()),
        }
        let exe_str = exe.to_string_lossy().to_string();
        if crate::hypr::hypr_exec(&exe_str) {
            return;
        }
        eprintln!("[finick] hyprctl exec locker failed for {exe_str}");
    } else {
        eprintln!("[finick] locker binary not found (set FINICK_LOCKER_BIN to override)");
    }
    // PATH fallbacks, session-lock client first.
    for name in LOCK_BINARIES {
        if crate::hypr::hypr_exec(name) {
            return;
        }
    }
    for name in LOCK_BINARIES {
        if let Err(e) = Command::new(name).spawn() {
            eprintln!("[finick] fallback {name} spawn failed: {e}");
        } else {
            return;
        }
    }
}

pub fn trigger_lock() {
    if std::env::var("FINICK_LOCKER_DISABLED").is_ok() {
        eprintln!("[finick] locker disabled via FINICK_LOCKER_DISABLED");
        return;
    }
    static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let last = LAST.load(std::sync::atomic::Ordering::Relaxed);
    if now.wrapping_sub(last) < 5 {
        eprintln!("[finick] locker trigger debounced ({}s since last)", now.wrapping_sub(last));
        return;
    }
    LAST.store(now, std::sync::atomic::Ordering::Relaxed);
    if std::env::var("WAYLAND_DISPLAY").is_err() && std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        eprintln!("[finick] no wayland session, skipping lock");
        return;
    }
    spawn_locker(&locker_binary());
}

pub fn locker_lockfile() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("finick-locker.lock")
}

pub fn locker_locked() -> bool {
    locker_lockfile().is_file()
}

pub fn locker_process_alive() -> bool {
    if let Ok(old) = std::fs::read_to_string(locker_lockfile())
        && let Ok(pid) = old.trim().parse::<i32>() {
            let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
            if LOCK_BINARIES.iter().any(|n| cmdline.contains(n)) {
                return true;
            }
        }
    false
}

pub fn watch_lid_close<F: Fn() + Send + 'static>(on_close: F) {
    std::thread::spawn(move || {
        let mut last_closed = false;
        let paths = ["/proc/acpi/button/lid/LID0/state", "/proc/acpi/button/lid/LID/state"];
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let mut closed = false;
            for p in &paths {
                if let Ok(c) = std::fs::read_to_string(p)
                    && c.to_lowercase().contains("closed") { closed = true; break; }
            }
            if closed && !last_closed { on_close(); }
            last_closed = closed;
            if std::env::var("FINICK_WATCH_LID").is_err() && !closed { break; }
        }
    });
}
