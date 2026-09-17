use std::process::Command;

pub fn trigger_lock() {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("locker")))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "locker".to_string());
    if std::path::Path::new(&exe).exists() {
        let _ = Command::new(&exe).spawn();
        return;
    }
    let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", "locker"]).spawn();
    let _ = Command::new("locker").spawn();
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
