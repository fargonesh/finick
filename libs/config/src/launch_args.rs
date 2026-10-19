use std::path::PathBuf;

/// Parsed command-line launch options shared by all Finick GUI apps.
///
/// Supports deep-linking an app onto a specific page (for URI handlers and
/// `finick://` style invocations) plus a `--screenshot` debug flag that
/// captures the screen after the window has had time to paint, then exits.
///
/// Accepted forms (all equivalent for the page part):
/// - `settings --page wifi`
/// - `settings --page finick://settings/wifi`
/// - `settings wifi`
/// - `settings finick://settings/wifi`
/// - `settings settings:wifi`
#[derive(Debug, Clone)]
pub struct LaunchArgs {
    /// Normalized page slug (lowercase, `-` separated), if one was given.
    pub page: Option<String>,
    /// The raw URI the page was extracted from, if any.
    pub raw_uri: Option<String>,
    /// Extra free-form value attached to the page (search query, file path...).
    /// Populated from `--search`/`--query` or `?search=` / `?query=` / `?path=`.
    pub extra: Option<String>,
    /// Take a screenshot after launch, then exit.
    pub screenshot: bool,
    /// Where to write the screenshot. Defaults to `/tmp/<app>-screenshot.png`.
    pub screenshot_path: Option<PathBuf>,
    /// Milliseconds to wait for first paint before capturing.
    pub screenshot_delay_ms: u64,
}

impl Default for LaunchArgs {
    fn default() -> Self {
        Self {
            page: None,
            raw_uri: None,
            extra: None,
            screenshot: false,
            screenshot_path: None,
            screenshot_delay_ms: 1500,
        }
    }
}

impl LaunchArgs {
    /// Parse from the current process arguments (skipping argv[0]).
    pub fn parse() -> Self {
        Self::parse_from(std::env::args().skip(1))
    }

    /// Parse from an explicit iterator (used by tests and by `main` helpers).
    pub fn parse_from<I>(args: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut out = Self::default();
        let mut positionals: Vec<String> = Vec::new();
        let mut iter = args.into_iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--screenshot" => out.screenshot = true,
                "--page" | "--uri" | "-p" => {
                    if let Some(val) = iter.next() {
                        ingest_page_value(&mut out, &val);
                    }
                }
                "--search" | "--query" | "--path" => {
                    if let Some(val) = iter.next() {
                        out.extra = Some(val);
                    }
                }
                "--screenshot-to" | "--screenshot-path" | "--screenshot-out" => {
                    if let Some(val) = iter.next() {
                        out.screenshot = true;
                        out.screenshot_path = Some(PathBuf::from(val));
                    }
                }
                "--screenshot-delay-ms" | "--screenshot-delay" => {
                    if let Some(val) = iter.next() {
                        if let Ok(ms) = val.parse::<u64>() {
                            out.screenshot_delay_ms = ms;
                            out.screenshot = true;
                        }
                    }
                }
                "--help" | "-h" => {
                    out.extra = Some(String::from("__help__"));
                }
                s if s.starts_with("--page=") => ingest_page_value(&mut out, &s["--page=".len()..]),
                s if s.starts_with("--uri=") => ingest_page_value(&mut out, &s["--uri=".len()..]),
                s if s.starts_with("--search=") => out.extra = Some(s["--search=".len()..].to_string()),
                s if s.starts_with("--query=") => out.extra = Some(s["--query=".len()..].to_string()),
                s if s.starts_with("--path=") => out.extra = Some(s["--path=".len()..].to_string()),
                s if s.starts_with("--screenshot-to=") => {
                    out.screenshot = true;
                    out.screenshot_path = Some(PathBuf::from(&s["--screenshot-to=".len()..]));
                }
                s if s.starts_with("--screenshot-path=") => {
                    out.screenshot = true;
                    out.screenshot_path = Some(PathBuf::from(&s["--screenshot-path=".len()..]));
                }
                s if s.starts_with("--screenshot-delay-ms=") => {
                    if let Ok(ms) = s["--screenshot-delay-ms=".len()..].parse::<u64>() {
                        out.screenshot_delay_ms = ms;
                        out.screenshot = true;
                    }
                }
                s if s.starts_with("--screenshot") => {
                    // Tolerate `--screenshot=/path.png` and bare `--screenshot*`.
                    if let Some(eq) = s.find('=') {
                        let val = s[eq + 1..].trim();
                        if !val.is_empty() {
                            out.screenshot = true;
                            out.screenshot_path = Some(PathBuf::from(val));
                        }
                    } else {
                        out.screenshot = true;
                    }
                }
                s if s == "--" => {
                    // Everything after `--` is a positional (a path with dashes, ...).
                    positionals.extend(iter.by_ref());
                    break;
                }
                s if s.starts_with('-') => {
                    // Unknown flags are ignored so desktop-file field codes
                    // (`%U`, `%F`, ...) and future flags do not break launch.
                }
                other => positionals.push(other.to_string()),
            }
        }
        // First positional that looks like a page/URI wins; a second one
        // becomes `extra` (search query / path) when no explicit flag set it.
        // Desktop-file field codes (`%U`, `%F`, ...) are dropped entirely.
        let mut positional_extra: Option<String> = None;
        for pos in positionals {
            if is_desktop_field_code(&pos) {
                continue;
            }
            if out.page.is_none() && looks_like_page(&pos) {
                ingest_page_value(&mut out, &pos);
            } else if out.extra.is_none() && out.extra.as_deref() != Some("__help__") {
                positional_extra = Some(pos);
            }
        }
        if out.extra.is_none() {
            out.extra = positional_extra;
        }
        out
    }

    /// True when `--help` was passed.
    pub fn wants_help(&self) -> bool {
        self.extra.as_deref() == Some("__help__")
    }

    /// Print `app --help` text listing the valid page slugs.
    pub fn print_help(&self, app_name: &str, pages: &[&str]) {
        eprintln!("Usage: {app_name} [OPTIONS] [PAGE|URI]");
        eprintln!();
        eprintln!("Open {app_name} on a specific page (for URI handlers).");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  [PAGE|URI]  Page slug, path, or URI (e.g. `wifi`, `finick://{app_name}/wifi`)");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -p, --page <PAGE|URI>       Open on the given page");
        eprintln!("      --uri <URI>             Same as --page (URI handler entry point)");
        eprintln!("      --search, --query <Q>   Extra query/value for the page (store search, ...)");
        eprintln!("      --path <PATH>           Extra path value for the page (files, ...)");
        eprintln!("      --screenshot            Capture the screen after paint, then exit (debug)");
        eprintln!("      --screenshot-to <FILE>  Screenshot output path (implies --screenshot)");
        eprintln!("      --screenshot-delay-ms <MS>  Wait before capturing [default: 1500]");
        eprintln!("  -h, --help                Show this help");
        if !pages.is_empty() {
            eprintln!();
            eprintln!("Pages:");
            for p in pages {
                eprintln!("  {p}");
            }
        }
    }

    /// If `--screenshot` was passed, spawn a background thread that waits for
    /// first paint, captures via `grim` (falling back to `hyprshot`), prints
    /// the output path, and exits the process. Returns true when armed.
    pub fn maybe_arm_screenshot(&self, app_name: &str) -> bool {
        if !self.screenshot {
            return false;
        }
        let out_path = self.screenshot_path.clone().unwrap_or_else(|| {
            PathBuf::from(format!("/tmp/{app_name}-screenshot.png"))
        });
        let delay = self.screenshot_delay_ms;
        let app_label = app_name.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay.max(200)));
            match capture_screen(&out_path) {
                Ok(path) => {
                    println!("[{app_label}] screenshot saved to {}", path.display());
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("[{app_label}] screenshot failed: {e}");
                    std::process::exit(1);
                }
            }
        });
        true
    }
}

fn ingest_page_value(out: &mut LaunchArgs, raw: &str) {
    let raw = raw.trim();
    if raw.is_empty() || raw == "%U" || raw == "%F" || raw == "%u" || raw == "%f" {
        return;
    }
    if let Some((page, extra)) = split_uri_page_and_extra(raw) {
        if !page.is_empty() {
            out.page = Some(page);
            out.raw_uri = Some(raw.to_string());
        }
        if let Some(x) = extra {
            if out.extra.is_none() {
                out.extra = Some(x);
            }
        }
    }
}

fn is_desktop_field_code(s: &str) -> bool {
    let t = s.trim();
    matches!(t, "%U" | "%F" | "%u" | "%f" | "%i" | "%c" | "%k" | "%D" | "%d" | "%N" | "%n" | "%v" | "%m")
}

fn looks_like_page(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    // Desktop-file field codes are not pages.
    if is_desktop_field_code(t) {
        return false;
    }
    // Absolute/relative paths are handled by files as pages too; accept them.
    if t.starts_with('/') || t.starts_with("~/") || t == "~" || t.starts_with("./") || t.starts_with("../") {
        return true;
    }
    // URIs and slugs are pages; multi-word free text is not (it is a query).
    !(t.contains(' ') && !t.contains("://") && !t.contains(':') && !t.contains('/') && !t.contains('?'))
}

/// Split a raw page/URI value into `(normalized_page, extra_query)`.
///
/// Handles `finick://app/page`, `app://page`, `app:page`, bare `page`,
/// absolute file paths, `?search=` / `?query=` / `?path=` query params, and
/// trailing `#fragments`. Returns `(String, Option<String>)`.
fn split_uri_page_and_extra(raw: &str) -> Option<(String, Option<String>)> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    // Absolute paths stay intact (files app); paths are case-sensitive so
    // only strip trailing slashes, never lowercase.
    if t.starts_with('/') || t.starts_with("~/") || t == "~" {
        let (path, query_extra) = split_query_extra(t);
        let trimmed = path.trim_end_matches('/');
        let page = if trimmed.is_empty() { "/".to_string() } else { trimmed.to_string() };
        return Some((page, query_extra));
    }
    // Strip `scheme://` or `scheme:` prefix, keeping the remainder.
    let mut rest = t;
    let mut from_uri = false;
    if let Some(idx) = t.find("://") {
        rest = &t[idx + 3..];
        from_uri = true;
    } else if let Some(idx) = t.find(':') {
        let (scheme, after) = t.split_at(idx);
        // A Windows-style `C:\...` or a clock time is not a scheme.
        if !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') && after.len() > 1 {
            rest = &after[1..];
            from_uri = true;
        }
    }
    rest = rest.trim_start_matches('/');
    let (path_part, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let path_part = path_part.split('#').next().unwrap_or("").trim();
    let mut extra: Option<String> = None;
    if let Some(q) = query {
        extra = query_param(q, "search")
            .or_else(|| query_param(q, "query"))
            .or_else(|| query_param(q, "path"));
    }
    // `finick://settings/wifi` -> segments ["settings", "wifi"]: drop a
    // leading app-name segment so handlers can pass full URIs verbatim.
    let mut segments: Vec<&str> = path_part.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
    if from_uri && segments.len() >= 2 && is_app_segment(segments[0]) {
        segments.remove(0);
    }
    // Keep nested paths (files: `docs/notes`) joined; pages use the full tail.
    let page = normalize_page_key(&segments.join("/"));
    if page.is_empty() {
        return None;
    }
    Some((page, extra))
}

fn split_query_extra(t: &str) -> (String, Option<String>) {
    match t.split_once('?') {
        Some((p, q)) => {
            let extra = query_param(q, "search")
                .or_else(|| query_param(q, "query"))
                .or_else(|| query_param(q, "path"));
            (p.to_string(), extra)
        }
        None => (t.to_string(), None),
    }
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };
        if k.eq_ignore_ascii_case(key) && !v.is_empty() {
            return Some(percent_decode(v));
        }
    }
    None
}

fn is_app_segment(s: &str) -> bool {
    matches!(
        normalize_page_key(s).as_str(),
        "settings" | "files" | "store" | "apps" | "launcher" | "locker" | "overlay" | "finick"
    )
}

/// Lowercase, trim slashes, collapse `_`/` ` to `-`.
pub fn normalize_page_key(s: &str) -> String {
    s.trim()
        .trim_matches('/')
        .replace('_', "-")
        .replace(' ', "-")
        .to_lowercase()
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn find_tool(name: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    for dir in ["/usr/bin", "/usr/local/bin", "/bin"] {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Capture the full screen to `out_path` via `grim`, falling back to
/// `hyprshot -m output`. Returns the written path on success.
fn capture_screen(out_path: &PathBuf) -> Result<PathBuf, String> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
    }
    if let Some(grim) = find_tool("grim") {
        let status = std::process::Command::new(grim)
            .arg(out_path)
            .status()
            .map_err(|e| format!("failed to run grim: {e}"))?;
        if status.success() && out_path.is_file() {
            return Ok(out_path.clone());
        }
        return Err(format!("grim exited with {status}"));
    }
    if let Some(hyprshot) = find_tool("hyprshot") {
        let status = std::process::Command::new(hyprshot)
            .args(["-m", "output", "-o"])
            .arg(out_path.to_string_lossy().as_ref())
            .status()
            .map_err(|e| format!("failed to run hyprshot: {e}"))?;
        if status.success() && out_path.is_file() {
            return Ok(out_path.clone());
        }
        return Err(format!("hyprshot exited with {status}"));
    }
    Err("no screenshot tool found (install `grim` or `hyprshot`)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> LaunchArgs {
        LaunchArgs::parse_from(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn page_flag_bare_slug() {
        let a = parsed(&["--page", "wifi"]);
        assert_eq!(a.page.as_deref(), Some("wifi"));
    }

    #[test]
    fn positional_uri_strips_scheme_and_app() {
        let a = parsed(&["finick://settings/wifi"]);
        assert_eq!(a.page.as_deref(), Some("wifi"));
        assert_eq!(a.raw_uri.as_deref(), Some("finick://settings/wifi"));
    }

    #[test]
    fn scheme_colon_form() {
        let a = parsed(&["settings:bluetooth"]);
        assert_eq!(a.page.as_deref(), Some("bluetooth"));
    }

    #[test]
    fn query_param_becomes_extra() {
        let a = parsed(&["finick://store/installed?search=firefox"]);
        assert_eq!(a.page.as_deref(), Some("installed"));
        assert_eq!(a.extra.as_deref(), Some("firefox"));
    }

    #[test]
    fn search_flag_and_screenshot() {
        let a = parsed(&["--page", "installed", "--search", "gimp", "--screenshot"]);
        assert_eq!(a.page.as_deref(), Some("installed"));
        assert_eq!(a.extra.as_deref(), Some("gimp"));
        assert!(a.screenshot);
    }

    #[test]
    fn absolute_path_is_page_for_files() {
        let a = parsed(&["/home/user/Documents"]);
        assert_eq!(a.page.as_deref(), Some("/home/user/Documents"));
    }

    #[test]
    fn desktop_field_codes_ignored() {
        let a = parsed(&["%U"]);
        assert_eq!(a.page, None);
        assert_eq!(a.extra, None);
    }

    #[test]
    fn screenshot_to_implies_screenshot() {
        let a = parsed(&["--screenshot-to", "/tmp/x.png"]);
        assert!(a.screenshot);
        assert_eq!(a.screenshot_path, Some(PathBuf::from("/tmp/x.png")));
    }
}
