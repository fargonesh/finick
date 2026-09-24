use std::process::Command;

/// Resolves the locker binary: explicit env override, sibling of the
/// current executable, well-known install locations, then PATH.
pub fn locker_binary() -> Option<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("FINICK_LOCKER_BIN").map(std::path::PathBuf::from) {
        let cand = if dir.is_file() { dir } else { dir.join("locker") };
        if cand.is_file() {
            return Some(cand);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["locker", "finick-locker"] {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
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
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).map(|d| d.join("locker")).find(|p| p.is_file())
    })
}

pub fn trigger_lock() {
    if let Some(exe) = locker_binary() {
        match Command::new(&exe).spawn() {
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
    // Last resort: rely on PATH inside hyprland / the shell.
    let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", "locker"]).spawn();
    if let Err(e) = Command::new("locker").spawn() {
        eprintln!("[finick] fallback locker spawn failed: {e}");
    }
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
