use std::process::Command;

pub fn locker_binary() -> Option<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("FINICK_LOCKER_BIN").map(std::path::PathBuf::from) {
        if dir.is_file() {
            return Some(dir);
        }
        let cand = dir.join("locker");
        if cand.is_file() {
            return Some(cand);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join("locker");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    for base in [
        "/run/current-system/sw/bin/locker",
        "/usr/local/bin/locker",
        "/usr/bin/locker",
    ] {
        let cand = std::path::PathBuf::from(base);
        if cand.is_file() {
            return Some(cand);
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        for cand in [home.join(".cargo/bin/locker"), home.join(".local/bin/locker")] {
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    std::env::var_os("PATH")
        .and_then(|paths| std::env::split_paths(&paths).map(|d| d.join("locker")).find(|p| p.is_file()))
}

fn spawn_locker(locker: &Option<std::path::PathBuf>) {
    if let Some(exe) = locker {
        match Command::new(exe).spawn() {
            Ok(_) => return,
            Err(e) => eprintln!("[finick] failed to spawn locker at {}: {e}", exe.display()),
        }
        let exe_str = exe.to_string_lossy().to_string();
        match Command::new("hyprctl").args(["dispatch", "exec", "--", &exe_str]).spawn() {
            Ok(_) => return,
            Err(e) => eprintln!("[finick] hyprctl dispatch exec locker failed: {e}"),
        }
    } else {
        eprintln!("[finick] locker binary not found (set FINICK_LOCKER_BIN to override)");
    }
    let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", "locker"]).spawn();
    if let Err(e) = Command::new("locker").spawn() {
        eprintln!("[finick] fallback locker spawn failed: {e}");
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
    if let Ok(old) = std::fs::read_to_string(locker_lockfile()) {
        if let Ok(pid) = old.trim().parse::<i32>() {
            let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
            if cmdline.contains("locker") {
                return true;
            }
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
                if let Ok(c) = std::fs::read_to_string(p) {
                    if c.to_lowercase().contains("closed") { closed = true; break; }
                }
            }
            if closed && !last_closed { on_close(); }
            last_closed = closed;
            if std::env::var("FINICK_WATCH_LID").is_err() && !closed { break; }
        }
    });
}
