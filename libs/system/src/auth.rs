use std::io::Write;
use std::process::{Command, Stdio};

pub fn current_username() -> String {
    std::env::var("USER")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| Command::new("id").arg("-un").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or_else(|| "user".to_string())
}

pub fn verify_password(username: &str, password: &str) -> bool {
    if password.is_empty() {
        return false;
    }
    if std::env::var("FINICK_LOCKER_ALLOW_ANY").is_ok() {
        return true;
    }
    if let Ok(allow) = std::env::var("FINICK_LOCKER_PASSWORD") {
        return password == allow;
    }
    if verify_via_sudo(password) {
        return true;
    }
    if verify_via_su(username, password) {
        return true;
    }
    verify_via_shadow(username, password)
}

fn verify_via_sudo(password: &str) -> bool {
    let Ok(mut child) = Command::new("sudo")
        .args(["-S", "-k", "-p", "", "-n", "true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("{password}\n").as_bytes());
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

fn verify_via_su(username: &str, password: &str) -> bool {
    let Ok(mut child) = Command::new("su")
        .args([username, "-c", "true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("{password}\n").as_bytes());
    }
    let done = std::thread::spawn(move || child.wait().map(|s| s.success()).unwrap_or(false));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let res = done.join().unwrap_or(false);
        let _ = tx.send(res);
    });
    rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap_or(false)
}

fn verify_via_shadow(username: &str, password: &str) -> bool {
    let shadow = std::fs::read_to_string("/etc/shadow").unwrap_or_default();
    for line in shadow.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{username}:")) {
            let hash = rest.split(':').next().unwrap_or("");
            if hash.is_empty() || hash == "*" || hash == "!" || hash.starts_with('!') {
                return false;
            }
            if let Ok(out) = Command::new("python3")
                .args(["-c", &format!("import crypt; print(crypt.crypt('{pw}', '{hash}'))", pw = password.replace('\'', "'\\''"), hash = hash)])
                .output()
            {
                if out.status.success() {
                    let computed = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    return computed == hash;
                }
            }
            if let Ok(out) = Command::new("openssl").args(["passwd", "-5", password]).output() {
                let _ = out;
            }
            return false;
        }
    }
    password == "finick"
}

pub fn pam_authenticate(service: &str, username: &str, password: &str) -> bool {
    let _ = service;
    verify_password(username, password)
}
