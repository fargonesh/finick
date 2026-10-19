use std::process::Command;

/// Escape a string for embedding in a double-quoted Lua string.
pub fn lua_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

fn dispatch_output_ok(out: &std::process::Output) -> bool {
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout} {stderr}").to_lowercase();
    // Hyprland Lua errors surface as "error", "fail", or Lua stack traces
    // like "attempt to perform arithmetic on a nil value". Legacy `dispatch
    // exec` on Lua configs fails exactly this way, so treat it as failure
    // and let the caller fall back to the other syntax.
    if combined.contains("error") || combined.contains("fail") || combined.contains("attempt to") || combined.contains("nil value") {
        // ...unless stdout positively confirms success.
        let ok = stdout.trim().to_lowercase();
        if ok == "ok" || ok.starts_with("ok") {
            return true;
        }
        return false;
    }
    true
}

/// Run `hyprctl dispatch <arg>`, returning true only when Hyprland reports success.
/// `spawn().is_ok()` is NOT sufficient: the process spawns fine even when the
/// dispatcher errors (e.g. legacy `exec` under a Lua config).
fn try_dispatch(arg: &str) -> bool {
    match Command::new("hyprctl").args(["dispatch", arg]).output() {
        Ok(out) => dispatch_output_ok(&out),
        Err(_) => false,
    }
}

fn try_dispatch_two(a: &str, b: &str) -> bool {
    match Command::new("hyprctl").args(["dispatch", a, b]).output() {
        Ok(out) => dispatch_output_ok(&out),
        Err(_) => false,
    }
}

fn try_eval(lua: &str) -> bool {
    match Command::new("hyprctl").args(["eval", lua]).output() {
        Ok(out) => dispatch_output_ok(&out),
        Err(_) => false,
    }
}

/// Execute a shell command via Hyprland, compatible with both config providers:
/// - Lua (>=0.55): `hyprctl dispatch hl.dsp.exec_cmd("<cmd>")`
/// - Legacy (hyprlang): `hyprctl dispatch exec -- <cmd>`
///
/// Legacy `dispatch exec` on a Lua config is wrapped as
/// `hl.dispatch(exec ...)` and fails with
/// "attempt to perform arithmetic on a nil value (global 'exec')".
/// Trying the Lua form first fixes launcher/app launches on new Hyprland
/// while the fallback keeps old Hyprland working.
pub fn hypr_exec(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return false;
    }
    let escaped = lua_escape(trimmed);
    let lua_arg = format!(r#"hl.dsp.exec_cmd("{escaped}")"#);
    if try_dispatch(&lua_arg) {
        return true;
    }
    // `eval hl.dispatch(...)` is equivalent to `dispatch hl.dsp...`; try it
    // too since some builds accept one form but not the other.
    if try_eval(&format!("hl.dispatch({lua_arg})")) {
        return true;
    }
    // Legacy fallback for hyprlang configs.
    match Command::new("hyprctl").args(["dispatch", "exec", "--", trimmed]).output() {
        Ok(out) => dispatch_output_ok(&out),
        Err(_) => false,
    }
}

/// Focus a window by Hyprland selector (e.g. `class:^(launcher)$` or `address:0x...`).
pub fn hypr_focus_window(selector: &str) -> bool {
    let s = selector.trim();
    if s.is_empty() {
        return false;
    }
    let escaped = lua_escape(s);
    // New API: hl.dsp.focus({ window = "..." })
    let lua_arg = format!(r#"hl.dsp.focus({{ window = "{escaped}" }})"#);
    if try_dispatch(&lua_arg) {
        return true;
    }
    if try_eval(&format!("hl.dispatch({lua_arg})")) {
        return true;
    }
    try_dispatch_two("focuswindow", s)
}

/// Bring a window to top by selector.
pub fn hypr_bring_to_top(selector: &str) -> bool {
    let s = selector.trim();
    if s.is_empty() {
        // No-arg form: just raise current/active.
        if try_dispatch(r#"hl.dsp.window.bring_to_top()"#) {
            return true;
        }
        return try_dispatch("bringactivetotop");
    }
    let escaped = lua_escape(s);
    let lua_arg = format!(r#"hl.dsp.window.bring_to_top({{ window = "{escaped}" }})"#);
    if try_dispatch(&lua_arg) {
        return true;
    }
    if try_eval(&format!("hl.dispatch({lua_arg})")) {
        return true;
    }
    try_dispatch_two("bringactivetotop", s)
}

/// Close a window by selector.
pub fn hypr_close_window(selector: &str) -> bool {
    let s = selector.trim();
    if s.is_empty() {
        return false;
    }
    let escaped = lua_escape(s);
    let lua_arg = format!(r#"hl.dsp.window.close({{ window = "{escaped}" }})"#);
    if try_dispatch(&lua_arg) {
        return true;
    }
    if try_eval(&format!("hl.dispatch({lua_arg})")) {
        return true;
    }
    try_dispatch_two("closewindow", s)
}

/// Switch to a workspace by id/name.
pub fn hypr_workspace(id: &str) -> bool {
    let s = id.trim();
    if s.is_empty() {
        return false;
    }
    let escaped = lua_escape(s);
    let lua_arg = format!(r#"hl.dsp.focus({{ workspace = "{escaped}" }})"#);
    if try_dispatch(&lua_arg) {
        return true;
    }
    if try_eval(&format!("hl.dispatch({lua_arg})")) {
        return true;
    }
    try_dispatch_two("workspace", s)
}

/// Install/refresh a keybind, Lua-first with legacy fallback.
/// Returns Ok(true) when a working bind is confirmed.
pub fn hypr_bind(combo: &str, exec_cmd: &str) -> Result<bool, String> {
    let escaped = lua_escape(exec_cmd);
    let lua = format!(r#"hl.bind("{combo}", hl.dsp.exec_cmd("{escaped}"))"#);
    match Command::new("hyprctl").args(["eval", &lua]).output() {
        Ok(out) if dispatch_output_ok(&out) => return Ok(true),
        Ok(out) => {
            let detail = format!(
                "{} {}",
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
            // Fall through to legacy `keyword bind` before erroring.
            let legacy = format!("{combo}, exec, {exec_cmd}");
            match Command::new("hyprctl").args(["keyword", "bind", &legacy]).output() {
                Ok(leg) if dispatch_output_ok(&leg) => return Ok(true),
                _ => return Err(format!("hyprctl eval bind failed: {}", detail.trim())),
            }
        }
        Err(e) => return Err(format!("hyprctl eval bind failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_escape_quotes_and_backslashes() {
        assert_eq!(lua_escape(r#"a"b\c"#), r#"a\"b\\c"#);
    }

    #[test]
    fn exec_cmd_arg_is_quoted() {
        let cmd = "/run/current-system/sw/bin/launcher";
        let escaped = lua_escape(cmd);
        let arg = format!(r#"hl.dsp.exec_cmd("{escaped}")"#);
        // Must be valid Lua: the path is inside quotes, never bare `exec /run/...`.
        assert!(arg.contains('"'));
        assert!(!arg.contains("exec /run"));
    }
}
