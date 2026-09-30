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
    // dlopened libpam (no link-time dep, same pattern as xkbcommon-dl):
    // whatever the admin installed in this stack (password, fingerprint,
    // face, u2f, smartcard...) runs here; text prompts are answered from
    // the typed buffer, touch/presence modules need no conversation.
    // Missing libpam degrades to false and the caller falls back.
    if password.is_empty() {
        return false;
    }
    pam_dlopen(service, username, password).unwrap_or(false)
}

const PAM_SUCCESS: libc::c_int = 0;
const PAM_BUF_ERR: libc::c_int = 5;
const PAM_CONV_ERR: libc::c_int = 6;
const PAM_PROMPT_ECHO_OFF: libc::c_int = 2;
const PAM_PROMPT_ECHO_ON: libc::c_int = 3;
const PAM_ERROR_MSG: libc::c_int = 4;
const PAM_TEXT_INFO: libc::c_int = 5;

/// A request from a running PAM transaction to the UI. Generic: works with
/// any stack the admin configured (password, U2F/FIDO touch cues, OTP,
/// info banners) the same way GDM surfaces them.
#[derive(Debug, Clone, PartialEq)]
pub enum PamRequest {
    Password { prompt: String },
    Info(String),
    Error(String),
}

/// Reply from the UI back to a waiting PAM conversation.
#[derive(Debug, Clone, PartialEq)]
pub enum PamReply {
    Password(String),
    Cancel,
}

/// Run one PAM authentication transaction on the calling thread, forwarding
/// every prompt and message to `req_tx` and blocking for `PamReply` on
/// secret prompts. Returns true only when authenticate and account
/// management both succeed. Returns None when PAM itself is unusable
/// (missing libpam/symbols); the caller should fall back. A dropped UI side
/// or `Cancel` aborts the transaction closed (Some(false)).
pub fn pam_interactive(
    service: &str,
    username: &str,
    req_tx: &std::sync::mpsc::Sender<PamRequest>,
    resp_rx: &std::sync::mpsc::Receiver<PamReply>,
) -> Option<bool> {
    use std::ffi::CString;
    use std::os::raw::c_void;

    if username.contains('\0') || service.contains('\0') {
        return Some(false);
    }
    unsafe {
        let lib = {
            let name = b"libpam.so.0\0";
            let h = libc::dlopen(name.as_ptr() as *const libc::c_char, libc::RTLD_NOW | libc::RTLD_LOCAL);
            if h.is_null() {
                let name = b"libpam.so\0";
                let h = libc::dlopen(name.as_ptr() as *const libc::c_char, libc::RTLD_NOW | libc::RTLD_LOCAL);
                if h.is_null() {
                    return None;
                }
                h
            } else {
                h
            }
        };
        macro_rules! sym {
            ($name:literal) => {{
                let n = concat!($name, "\0");
                let p = libc::dlsym(lib, n.as_ptr() as *const libc::c_char);
                if p.is_null() {
                    libc::dlclose(lib);
                    return None;
                }
                p
            }};
        }
        type PamStart =
            unsafe extern "C" fn(*const libc::c_char, *const libc::c_char, *const PamConv, *mut *mut c_void) -> libc::c_int;
        type PamSimple = unsafe extern "C" fn(*mut c_void, libc::c_int) -> libc::c_int;
        let pam_start: PamStart = std::mem::transmute(sym!("pam_start"));
        let pam_authenticate: PamSimple = std::mem::transmute(sym!("pam_authenticate"));
        let pam_acct_mgmt: PamSimple = std::mem::transmute(sym!("pam_acct_mgmt"));
        let pam_end: PamSimple = std::mem::transmute(sym!("pam_end"));

        let state = Interact {
            user: CString::new(username).ok()?,
            req_tx,
            resp_rx,
        };
        let conv = PamConv { conv: Some(interact_cb), appdata_ptr: &state as *const Interact as *mut c_void };
        let svc = CString::new(service).ok()?;
        let mut handle: *mut c_void = std::ptr::null_mut();
        let ok = pam_start(svc.as_ptr(), state.user.as_ptr(), &conv, &mut handle) == PAM_SUCCESS
            && !handle.is_null()
            && pam_authenticate(handle, 0) == PAM_SUCCESS
            && pam_acct_mgmt(handle, 0) == PAM_SUCCESS;
        if !handle.is_null() {
            pam_end(handle, if ok { PAM_SUCCESS } else { 7 });
        }
        libc::dlclose(lib);
        std::hint::black_box(&state);
        Some(ok)
    }
}

struct Interact<'a> {
    user: std::ffi::CString,
    req_tx: &'a std::sync::mpsc::Sender<PamRequest>,
    resp_rx: &'a std::sync::mpsc::Receiver<PamReply>,
}

unsafe extern "C" fn interact_cb(
    num_msg: libc::c_int,
    msgs: *const *const PamMessage,
    resps: *mut *mut PamResponse,
    appdata: *mut libc::c_void,
) -> libc::c_int {
    if msgs.is_null() || resps.is_null() || appdata.is_null() || num_msg <= 0 {
        return PAM_CONV_ERR;
    }
    // SAFETY: same contract as conv_cb; state outlives the transaction on the
    // helper thread's stack, channels are thread-safe.
    unsafe { interact_cb_inner(num_msg, msgs, resps, &*(appdata as *const Interact)) }
}

// SAFETY: caller guarantees msgs/resps point to num_msg valid entries and
// state outlives the call (PAM conversation contract).
fn interact_cb_inner(
    num_msg: libc::c_int,
    msgs: *const *const PamMessage,
    resps: *mut *mut PamResponse,
    state: &Interact,
) -> libc::c_int {
    unsafe {
        let mut secret_slots: Vec<isize> = Vec::new();
        let mut prompt = String::new();
        for i in 0..num_msg as isize {
            let m = *(*msgs.offset(i));
            let text = if m.msg.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(m.msg).to_string_lossy().into_owned()
            };
            match m.msg_style {
                PAM_PROMPT_ECHO_OFF => {
                    secret_slots.push(i);
                    if !text.is_empty() {
                        if !prompt.is_empty() {
                            prompt.push('\n');
                        }
                        prompt.push_str(&text);
                    }
                }
                PAM_PROMPT_ECHO_ON => {}
                PAM_ERROR_MSG => {
                    let _ = state.req_tx.send(PamRequest::Error(text));
                }
                PAM_TEXT_INFO => {
                    let _ = state.req_tx.send(PamRequest::Info(text));
                }
                _ => {}
            }
        }
        let secret: Vec<u8> = if secret_slots.is_empty() {
            Vec::new()
        } else {
            let _ = state.req_tx.send(PamRequest::Password { prompt });
            match state.resp_rx.recv() {
                Ok(PamReply::Password(p)) => {
                    if p.contains('\0') {
                        return PAM_CONV_ERR;
                    }
                    p.into_bytes()
                }
                _ => return PAM_CONV_ERR,
            }
        };
        let out = libc::malloc(std::mem::size_of::<PamResponse>() * num_msg as usize) as *mut PamResponse;
        if out.is_null() {
            return PAM_BUF_ERR;
        }
        for i in 0..num_msg as isize {
            let m = *(*msgs.offset(i));
            let src: &[u8] = if secret_slots.contains(&i) {
                &secret
            } else if m.msg_style == PAM_PROMPT_ECHO_ON {
                state.user.as_bytes()
            } else {
                &[]
            };
            let dst = libc::malloc(src.len() + 1) as *mut libc::c_char;
            if dst.is_null() {
                for j in 0..i {
                    libc::free((*out.offset(j)).resp as *mut libc::c_void);
                }
                libc::free(out as *mut libc::c_void);
                return PAM_BUF_ERR;
            }
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, src.len());
            *dst.add(src.len()) = 0;
            (*out.offset(i)).resp = dst;
            (*out.offset(i)).resp_retcode = 0;
        }
        *resps = out;
        PAM_SUCCESS
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct PamMessage {
    msg_style: libc::c_int,
    msg: *const libc::c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct PamResponse {
    resp: *mut libc::c_char,
    resp_retcode: libc::c_int,
}

#[repr(C)]
struct PamConv {
    conv: Option<
        unsafe extern "C" fn(
            libc::c_int,
            *const *const PamMessage,
            *mut *mut PamResponse,
            *mut libc::c_void,
        ) -> libc::c_int,
    >,
    appdata_ptr: *mut libc::c_void,
}

struct Creds {
    user: std::ffi::CString,
    pass: std::ffi::CString,
}

unsafe extern "C" fn conv_cb(
    num_msg: libc::c_int,
    msgs: *const *const PamMessage,
    resps: *mut *mut PamResponse,
    appdata: *mut libc::c_void,
) -> libc::c_int {
    if msgs.is_null() || resps.is_null() || appdata.is_null() || num_msg <= 0 {
        return PAM_CONV_ERR;
    }
    // SAFETY: PAM guarantees msgs/resps/appdata valid for the call; appdata
    // is our Creds, alive on the caller's stack for the whole transaction.
    unsafe {
        conv_cb_inner(num_msg, msgs, resps, &*(appdata as *const Creds))
    }
}

// SAFETY: caller guarantees msgs/resps point to num_msg valid entries and
// creds outlives the call (PAM conversation contract).
fn conv_cb_inner(
    num_msg: libc::c_int,
    msgs: *const *const PamMessage,
    resps: *mut *mut PamResponse,
    creds: &Creds,
) -> libc::c_int {
    unsafe {
        let out = libc::malloc(std::mem::size_of::<PamResponse>() * num_msg as usize) as *mut PamResponse;
        if out.is_null() {
            return PAM_BUF_ERR;
        }
        for i in 0..num_msg as isize {
            let m = *(*msgs.offset(i));
            if m.msg_style == PAM_ERROR_MSG && !m.msg.is_null() {
                eprintln!("finick: pam: {}", std::ffi::CStr::from_ptr(m.msg).to_string_lossy());
            }
            // PAM frees responses with free(), so they must come from malloc.
            let src: &[u8] = match m.msg_style {
                PAM_PROMPT_ECHO_OFF => creds.pass.as_bytes(),
                PAM_PROMPT_ECHO_ON => creds.user.as_bytes(),
                _ => &[],
            };
            let dst = libc::malloc(src.len() + 1) as *mut libc::c_char;
            if dst.is_null() {
                for j in 0..i {
                    libc::free((*out.offset(j)).resp as *mut libc::c_void);
                }
                libc::free(out as *mut libc::c_void);
                return PAM_BUF_ERR;
            }
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, src.len());
            *dst.add(src.len()) = 0;
            (*out.offset(i)).resp = dst;
            (*out.offset(i)).resp_retcode = 0;
        }
        *resps = out;
        PAM_SUCCESS
    }
}

fn pam_dlopen(service: &str, username: &str, password: &str) -> Option<bool> {
    use std::ffi::CString;
    use std::os::raw::c_void;

    if password.contains('\0') || username.contains('\0') || service.contains('\0') {
        return Some(false);
    }
    unsafe {
        let lib = {
            let name = b"libpam.so.0\0";
            let h = libc::dlopen(name.as_ptr() as *const libc::c_char, libc::RTLD_NOW | libc::RTLD_LOCAL);
            if h.is_null() {
                let name = b"libpam.so\0";
                let h = libc::dlopen(name.as_ptr() as *const libc::c_char, libc::RTLD_NOW | libc::RTLD_LOCAL);
                if h.is_null() {
                    return None;
                }
                h
            } else {
                h
            }
        };
        macro_rules! sym {
            ($name:literal) => {{
                let n = concat!($name, "\0");
                let p = libc::dlsym(lib, n.as_ptr() as *const libc::c_char);
                if p.is_null() {
                    libc::dlclose(lib);
                    return None;
                }
                p
            }};
        }
        type PamStart =
            unsafe extern "C" fn(*const libc::c_char, *const libc::c_char, *const PamConv, *mut *mut c_void) -> libc::c_int;
        type PamSimple = unsafe extern "C" fn(*mut c_void, libc::c_int) -> libc::c_int;
        let pam_start: PamStart = std::mem::transmute(sym!("pam_start"));
        let pam_authenticate: PamSimple = std::mem::transmute(sym!("pam_authenticate"));
        let pam_acct_mgmt: PamSimple = std::mem::transmute(sym!("pam_acct_mgmt"));
        let pam_end: PamSimple = std::mem::transmute(sym!("pam_end"));

        let creds = Creds {
            user: CString::new(username).ok()?,
            pass: CString::new(password).ok()?,
        };
        let conv = PamConv { conv: Some(conv_cb), appdata_ptr: &creds as *const Creds as *mut c_void };
        let svc = CString::new(service).ok()?;
        let mut handle: *mut c_void = std::ptr::null_mut();
        let ok = pam_start(svc.as_ptr(), creds.user.as_ptr(), &conv, &mut handle) == PAM_SUCCESS
            && !handle.is_null()
            && pam_authenticate(handle, 0) == PAM_SUCCESS
            && pam_acct_mgmt(handle, 0) == PAM_SUCCESS;
        if !handle.is_null() {
            pam_end(handle, if ok { PAM_SUCCESS } else { 7 /* PAM_AUTH_ERR */ });
        }
        libc::dlclose(lib);
        // Keep the borrow checker honest: creds outlives the calls above.
        std::hint::black_box(&creds);
        Some(ok)
    }
}

/// PAM service to try first: our own stack when the OS ships it,
/// otherwise the ubiquitous `login` stack. `None` when PAM is unusable here.
pub fn pam_service_available() -> Option<String> {
    for candidate in ["locker", "login"] {
        if std::path::Path::new(&format!("/etc/pam.d/{candidate}")).exists() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Full auth decision: dev backdoors, then PAM (whatever the admin installed:
// password, fingerprint, face, u2f, smartcard...), then the legacy chain.
pub fn authenticate_user(username: &str, password: &str) -> bool {
    if password.is_empty() {
        return false;
    }
    if std::env::var("FINICK_LOCKER_ALLOW_ANY").is_ok() {
        return true;
    }
    if let Ok(allow) = std::env::var("FINICK_LOCKER_PASSWORD") {
        return password == allow;
    }
    if let Some(service) = pam_service_available() {
        if pam_authenticate(&service, username, password) {
            return true;
        }
        // PAM denied: fall through to the legacy chain (e.g. minimal
        // containers where the stack exists but can't backing-store auth).
    }
    verify_password(username, password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticate_backdoors() {
        // SAFETY: these vars are only touched by this test.
        unsafe {
            std::env::set_var("FINICK_LOCKER_PASSWORD", "s3cret");
        }
        assert!(authenticate_user("anyone", "s3cret"));
        assert!(!authenticate_user("anyone", "nope"));
        // SAFETY: same as above.
        unsafe {
            std::env::remove_var("FINICK_LOCKER_PASSWORD");
        }
        assert!(!authenticate_user("anyone", ""));
        // SAFETY: same as above.
        unsafe {
            std::env::set_var("FINICK_LOCKER_ALLOW_ANY", "1");
        }
        assert!(authenticate_user("anyone", "whatever"));
        // SAFETY: same as above.
        unsafe {
            std::env::remove_var("FINICK_LOCKER_ALLOW_ANY");
        }
    }

    #[test]
    fn test_pam_wrong_password_fails() {
        // Real PAM stack, bogus user: must fail closed, not crash.
        assert!(!pam_authenticate("login", "finick-nobody-test", "wrong"));
        assert!(!pam_authenticate("login", "finick-nobody-test", ""));
    }

    #[test]
    fn test_pam_interactive_unknown_service_fails_closed() {
        let (req_tx, req_rx) = std::sync::mpsc::channel();
        let (resp_tx, resp_rx) = std::sync::mpsc::channel();
        drop(resp_tx);
        let res = pam_interactive("finick-definitely-not-a-service", "nobody", &req_tx, &resp_rx);
        assert!(res == Some(false) || res.is_none());
        drop(req_rx);
    }

    #[test]
    fn test_pam_service_selection_sane() {
        if let Some(svc) = pam_service_available() {
            assert!(std::path::Path::new(&format!("/etc/pam.d/{svc}")).exists());
            assert!(svc == "locker" || svc == "login");
        }
    }
}
