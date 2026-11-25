//! Extendable topbar pills.
//!
//! Each pill is a small JSON file in `~/.config/finick/topbar-pills/*.json`:
//!
//! ```json
//! {
//!   "id": "now-playing",
//!   "label": "♪",
//!   "command": "playerctl metadata --format '{{artist}} - {{title}}'",
//!   "on_click": "playerctl play-pause",
//!   "interval_secs": 2,
//!   "max_chars": 32
//! }
//! ```
//!
//! `command` runs via `sh -c`, so a pill can be any language: bash, python,
//! or JS via `command: "node ~/.config/finick/topbar-pills/lyrics.js"`.
//! Empty output (or a failing command) hides the pill. `on_click` is optional
//! and runs detached when the pill is pressed. Only install pills you trust:
//! commands run as your user with your environment.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Where pill definitions live.
pub fn pills_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|c| c.join("finick").join("topbar-pills"))
        .unwrap_or_else(|| PathBuf::from("/tmp/finick-topbar-pills"))
}

pub const DEFAULT_INTERVAL_SECS: u64 = 5;
pub const DEFAULT_MAX_CHARS: usize = 32;
pub const MAX_PILLS: usize = 5;

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct TopbarPillDef {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub on_click: String,
    #[serde(default)]
    pub interval_secs: u64,
    #[serde(default)]
    pub max_chars: usize,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl TopbarPillDef {
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs.clamp(1, 3600))
    }

    pub fn max_chars(&self) -> usize {
        if self.max_chars == 0 {
            DEFAULT_MAX_CHARS
        } else {
            self.max_chars.clamp(8, 80)
        }
    }

    pub fn is_valid(&self) -> bool {
        self.enabled && !self.id.trim().is_empty() && !self.command.trim().is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopbarPillOutput {
    pub id: String,
    pub text: String,
    pub action: Option<String>,
}

impl TopbarPillOutput {
    pub fn has_action(&self) -> bool {
        self.action.as_ref().is_some_and(|a| !a.trim().is_empty())
    }
}

/// Load pill definitions from disk, sorted by id for a stable order.
/// Creates the directory on first run so users can discover it.
pub fn load_pill_defs() -> Vec<TopbarPillDef> {
    load_pill_defs_from(&pills_dir())
}

fn load_pill_defs_from(dir: &std::path::Path) -> Vec<TopbarPillDef> {
    let _ = std::fs::create_dir_all(dir);
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut defs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut def) = serde_json::from_str::<TopbarPillDef>(&raw) else {
            continue;
        };
        def.id = def.id.trim().to_string();
        def.command = def.command.trim().to_string();
        def.on_click = def.on_click.trim().to_string();
        if def.interval_secs == 0 {
            def.interval_secs = DEFAULT_INTERVAL_SECS;
        }
        if def.max_chars == 0 {
            def.max_chars = DEFAULT_MAX_CHARS;
        }
        if def.is_valid() {
            defs.push(def);
        }
    }
    defs.sort_by(|a, b| a.id.cmp(&b.id));
    defs.truncate(MAX_PILLS * 2);
    defs
}

/// Run a pill command via `sh -c` and return cleaned single-line output.
/// Failures and empty output yield an empty string (caller hides the pill).
pub fn run_pill_command(cmd: &str) -> String {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return String::new();
    }
    let out = std::process::Command::new("sh").args(["-c", cmd]).output();
    let Ok(out) = out else {
        return String::new();
    };
    if !out.status.success() {
        return String::new();
    }
    clean_output(&String::from_utf8_lossy(&out.stdout))
}

/// Collapse raw command output to a single display line.
pub fn clean_output(raw: &str) -> String {
    let mut s: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    s = s.trim().to_string();
    if s.len() > 512 {
        s.truncate(512);
        s = s.trim().to_string();
    }
    s
}

/// Truncate to `max` chars with an ellipsis. ASCII-safe and char-boundary-safe.
pub fn truncate_chars(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Combine an optional label prefix with raw output, truncated to fit.
pub fn render_text(def: &TopbarPillDef, raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    let full = if def.label.trim().is_empty() {
        raw.to_string()
    } else {
        format!("{} {raw}", def.label.trim())
    };
    truncate_chars(&full, def.max_chars())
}

#[derive(Clone)]
struct CachedPill {
    text: String,
    at: Instant,
}

static PILL_CACHE: Mutex<Option<HashMap<String, CachedPill>>> = Mutex::new(None);

fn cache_text(id: &str) -> Option<(String, Instant)> {
    PILL_CACHE
        .lock()
        .ok()?
        .as_ref()?
        .get(id)
        .map(|c| (c.text.clone(), c.at))
}

fn store_text(id: &str, text: String) {
    if let Ok(mut g) = PILL_CACHE.lock() {
        let map = g.get_or_insert_with(HashMap::new);
        map.insert(id.to_string(), CachedPill { text, at: Instant::now() });
    }
}

/// Poll all pill definitions, re-running only commands whose interval elapsed.
/// Blocking: call from `spawn_blocking`. Never shows errors; failed or empty
/// commands keep their previous text for one cycle, then hide when re-run.
pub fn poll_pill_outputs() -> Vec<TopbarPillOutput> {
    let defs = load_pill_defs();
    let mut out = Vec::new();
    for def in defs.iter().take(MAX_PILLS) {
        let due = match cache_text(&def.id) {
            None => true,
            Some((_, at)) => at.elapsed() >= def.interval(),
        };
        let text = if due {
            let raw = run_pill_command(&def.command);
            let text = render_text(def, &raw);
            store_text(&def.id, text.clone());
            text
        } else {
            cache_text(&def.id).map(|(t, _)| t).unwrap_or_default()
        };
        if text.is_empty() {
            continue;
        }
        let action = if def.on_click.trim().is_empty() {
            None
        } else {
            Some(def.on_click.clone())
        };
        out.push(TopbarPillOutput { id: def.id.clone(), text, action });
    }
    out
}

/// Run an `on_click` command detached. Failures are logged, not shown.
pub fn run_pill_action(cmd: &str) {
    let cmd = cmd.trim().to_string();
    if cmd.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        let ok = std::process::Command::new("sh").args(["-c", &cmd]).output().map(|o| o.status.success()).unwrap_or(false);
        if !ok {
            eprintln!("topbar pill action failed: {cmd}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_chars_ascii_and_unicode() {
        assert_eq!(truncate_chars("hello", 10), "hello");
        assert_eq!(truncate_chars("hello world", 5), "hell…");
        assert_eq!(truncate_chars("♪ music title", 4), "♪ m…");
        assert_eq!(truncate_chars("", 5), "");
    }

    #[test]
    fn test_render_text_label_and_truncation() {
        let def = TopbarPillDef {
            id: "x".to_string(),
            command: "echo hi".to_string(),
            label: "♪".to_string(),
            on_click: String::new(),
            interval_secs: 2,
            max_chars: 8,
            enabled: true,
        };
        assert_eq!(render_text(&def, "hello world"), "♪ hello…");
        let bare = TopbarPillDef { label: String::new(), ..def.clone() };
        assert_eq!(render_text(&bare, "hello world"), "hello w…");
        assert_eq!(render_text(&def, "   "), "");
    }

    #[test]
    fn test_clean_output_collapses_whitespace() {
        assert_eq!(clean_output("  Artist  -  Title\n"), "Artist - Title");
        assert_eq!(clean_output("a\nb\nc"), "a b c");
        assert_eq!(clean_output(""), "");
    }

    #[test]
    fn test_run_pill_command_success_and_failure() {
        assert_eq!(run_pill_command("echo hello"), "hello");
        assert_eq!(run_pill_command("false"), "");
        assert_eq!(run_pill_command(""), "");
    }

    #[test]
    fn test_load_pill_defs_from_filters_invalid() {
        let dir = std::env::temp_dir().join(format!("finick-pills-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("good.json"), r#"{"id":"b-now","command":"echo hi","label":"♪"}"#).unwrap();
        std::fs::write(dir.join("disabled.json"), r#"{"id":"a-off","command":"echo x","enabled":false}"#).unwrap();
        std::fs::write(dir.join("empty.json"), r#"{"id":"c-empty","command":""}"#).unwrap();
        std::fs::write(dir.join("broken.json"), r#"{"id": nope"#).unwrap();
        std::fs::write(dir.join("notes.txt"), "ignore me").unwrap();
        let defs = load_pill_defs_from(&dir);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].id, "b-now");
        assert_eq!(defs[0].interval_secs, DEFAULT_INTERVAL_SECS);
        assert_eq!(defs[0].max_chars, DEFAULT_MAX_CHARS);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_def_validation_and_clamps() {
        let mut def = TopbarPillDef {
            id: " ".to_string(),
            command: "echo hi".to_string(),
            label: String::new(),
            on_click: String::new(),
            interval_secs: 0,
            max_chars: 0,
            enabled: true,
        };
        assert!(!def.is_valid());
        def.id = "ok".to_string();
        assert!(def.is_valid());
        assert_eq!(def.max_chars(), DEFAULT_MAX_CHARS);
        def.max_chars = 500;
        assert_eq!(def.max_chars(), 80);
        def.interval_secs = 0;
        assert_eq!(def.interval(), Duration::from_secs(1));
    }
}
