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
    /// Run the screenshot inside an isolated Xvfb virtual display.
    /// Implies `--screenshot`. The harness spawns Xvfb, re-executes this
    /// binary under it (X11), captures, then tears everything down so
    /// product shots never include the host desktop.
    pub screenshot_virtual: bool,
    /// Virtual display size in pixels (only used with `--screenshot-virtual`).
    pub screenshot_width: u32,
    /// Virtual display height in pixels (only used with `--screenshot-virtual`).
    pub screenshot_height: u32,
    /// Main-window width override in pixels (`--window-size`). `None` keeps
    /// the app default. Orthogonal to `--screenshot`: combine both for
    /// deterministic product-shot geometry.
    pub window_width: Option<u32>,
    /// Main-window height override in pixels (`--window-size`).
    pub window_height: Option<u32>,
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
            screenshot_virtual: false,
            screenshot_width: 1280,
            screenshot_height: 800,
            window_width: None,
            window_height: None,
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
                "--screenshot-virtual" | "--virtual-display" | "--screenshot-headless" => {
                    out.screenshot = true;
                    out.screenshot_virtual = true;
                }
                "--screenshot-size" | "--screenshot-geometry" | "--virtual-display-size" => {
                    if let Some(val) = iter.next() {
                        if let Some((w, h)) = parse_dimensions(&val) {
                            out.screenshot_width = w;
                            out.screenshot_height = h;
                            out.screenshot = true;
                            out.screenshot_virtual = true;
                        }
                    }
                }
                "--window-size" | "--window-geometry" => {
                    if let Some(val) = iter.next() {
                        if let Some((w, h)) = parse_dimensions(&val) {
                            out.window_width = Some(w);
                            out.window_height = Some(h);
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
                s if s.starts_with("--screenshot-size=")
                | s.starts_with("--screenshot-geometry=")
                | s.starts_with("--virtual-display-size=") => {
                    if let Some(eq) = s.find('=') {
                        if let Some((w, h)) = parse_dimensions(s[eq + 1..].trim()) {
                            out.screenshot_width = w;
                            out.screenshot_height = h;
                            out.screenshot = true;
                            out.screenshot_virtual = true;
                        }
                    }
                }
                s if s.starts_with("--window-size=") | s.starts_with("--window-geometry=") => {
                    if let Some(eq) = s.find('=') {
                        if let Some((w, h)) = parse_dimensions(s[eq + 1..].trim()) {
                            out.window_width = Some(w);
                            out.window_height = Some(h);
                        }
                    }
                }
                s if s.starts_with("--screenshot-virtual") || s.starts_with("--virtual-display") => {
                    // Covers `--screenshot-virtual`, `--screenshot-virtual=true`,
                    // `--virtual-display`, `--virtual-display-size` (size parsed above).
                    out.screenshot = true;
                    out.screenshot_virtual = true;
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
        eprintln!("      --screenshot-virtual    Run inside an isolated Xvfb virtual display");
        eprintln!("                              (implies --screenshot; clean product shots, headless-safe)");
        eprintln!("      --screenshot-size <WxH> Virtual display size [default: 1280x800]");
        eprintln!("      --window-size <WxH>     Main-window size override (debugging / deterministic shots)");
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
    /// first paint, captures via `grim` (falling back to `hyprshot`, then
    /// `import -window root` for X11/Xvfb), prints the output path, and exits
    /// the process. Returns true when armed.
    ///
    /// If `--screenshot-virtual` was passed (and this process is not already
    /// inside the virtual display), this instead spawns an isolated Xvfb
    /// server, re-executes the current binary under it, waits for the child
    /// to capture and exit, tears down Xvfb, and exits with the child's
    /// status. The child sees `FINICK_SCREENSHOT_VIRTUAL=1` and runs the
    /// normal screenshot flow under X11.
    pub fn maybe_arm_screenshot(&self, app_name: &str) -> bool {
        if !self.screenshot {
            return false;
        }
        if self.screenshot_virtual && std::env::var("FINICK_SCREENSHOT_VIRTUAL").as_deref() != Ok("1") {
            let out_path = self.screenshot_path.clone().unwrap_or_else(|| {
                PathBuf::from(format!("/tmp/{app_name}-screenshot.png"))
            });
            let delay = self.screenshot_delay_ms;
            let width = self.screenshot_width;
            let height = self.screenshot_height;
            let app_label = app_name.to_string();
            run_virtual_screenshot_harness(&out_path, delay, width, height, &app_label);
            return true;
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

    /// Main-window size override from `--window-size`, if one was given.
    /// Apps apply this to their root `WindowConfig::with_size` after the
    /// default so debugging and product shots use deterministic geometry.
    pub fn window_size_override(&self) -> Option<(f64, f64)> {
        match (self.window_width, self.window_height) {
            (Some(w), Some(h)) => Some((w as f64, h as f64)),
            _ => None,
        }
    }
}

/// Parse `1280x800` (also `1280,800` / `1280 800`) into `(width, height)`.
fn parse_dimensions(raw: &str) -> Option<(u32, u32)> {
    let cleaned = raw.trim().replace([',', ' '], "x");
    let mut parts = cleaned.split('x');
    let w: u32 = parts.next()?.trim().parse().ok()?;
    let h: u32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || w < 320 || h < 200 || w > 8192 || h > 8192 {
        return None;
    }
    Some((w, h))
}

/// Marker env set inside the re-executed virtual-display child.
const VIRTUAL_MARKER: &str = "FINICK_SCREENSHOT_VIRTUAL";

/// Parent-side harness for `--screenshot-virtual`: boot Xvfb on a free
/// display, re-run this binary under it, wait for the child (which performs
/// its own `--screenshot` capture), then clean up and exit.
fn run_virtual_screenshot_harness(
    out_path: &std::path::Path,
    delay_ms: u64,
    width: u32,
    height: u32,
    app_label: &str,
) {
    match run_virtual_harness_inner(out_path, delay_ms, width, height, app_label) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("[{app_label}] screenshot failed: {e}");
            std::process::exit(1);
        }
    }
}

fn run_virtual_harness_inner(
    out_path: &std::path::Path,
    delay_ms: u64,
    width: u32,
    height: u32,
    app_label: &str,
) -> Result<i32, String> {
    let xvfb = find_tool("Xvfb")
        .ok_or_else(|| "no virtual display tool found (install `xorg-xserver` for Xvfb)".to_string())?;
    let display = find_free_display()
        .ok_or_else(|| "no free X display found (tried :99-:139)".to_string())?;
    let display_str = format!(":{display}");

    // Boot Xvfb.
    let mut xvfb_child = std::process::Command::new(&xvfb)
        .args([
            display_str.clone(),
            "-screen".to_string(),
            "0".to_string(),
            format!("{width}x{height}x24"),
            "-ac".to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start Xvfb: {e}"))?;
    if !wait_for_x_socket(display, std::time::Duration::from_secs(10)) {
        let _ = xvfb_child.kill();
        return Err(format!("Xvfb {display_str} did not become ready in time"));
    }

    // Re-execute this binary under the virtual display, stripping the
    // virtual-display flags so the child does not recurse.
    let exe =
        std::env::current_exe().map_err(|e| format!("cannot find current executable: {e}"))?;
    let mut child_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    for arg in std::env::args().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "--screenshot-virtual" | "--virtual-display" | "--screenshot-headless" => continue,
            "--screenshot-size" | "--screenshot-geometry" | "--virtual-display-size" => {
                skip_next = true;
                continue;
            }
            s if s.starts_with("--screenshot-virtual")
                || s.starts_with("--virtual-display")
                || s.starts_with("--screenshot-size=")
                || s.starts_with("--screenshot-geometry=")
                || s.starts_with("--virtual-display-size=") =>
            {
                continue;
            }
            other => child_args.push(other.to_string()),
        }
    }
    // Ensure the child still captures even if the user only passed the
    // virtual flag (which implies --screenshot).
    if !child_args.iter().any(|a| a.starts_with("--screenshot")) {
        child_args.push("--screenshot".to_string());
    }

    // Bound total wait: capture delay + generous grace for launch/paint.
    let timeout = std::time::Duration::from_millis(delay_ms.max(200) + 60_000);
    let child_status = (|| {
        let mut child = std::process::Command::new(&exe)
            .args(&child_args)
            .env("DISPLAY", &display_str)
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("HYPRLAND_INSTANCE_SIGNATURE")
            .env("XDG_SESSION_TYPE", "x11")
            .env(VIRTUAL_MARKER, "1")
            .spawn()
            .map_err(|e| format!("failed to re-execute under Xvfb: {e}"))?;
        let start = std::time::Instant::now();
        loop {
            match child.try_wait().map_err(|e| format!("failed to wait for child: {e}"))? {
                Some(status) => return Ok::<std::process::ExitStatus, String>(status),
                None => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        return Err(format!(
                            "timed out waiting for screenshot child after {}s",
                            timeout.as_secs()
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    })();

    let _ = xvfb_child.kill();
    let _ = xvfb_child.wait();

    match child_status {
        Ok(status) => {
            if status.success() && out_path.is_file() {
                println!("[{app_label}] screenshot saved to {}", out_path.display());
                Ok(0)
            } else if status.success() {
                Err(format!(
                    "child exited 0 but {} was not created (check Xvfb/import output)",
                    out_path.display()
                ))
            } else {
                Err(format!(
                    "screenshot child exited with {}",
                    status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".to_string())
                ))
            }
        }
        Err(e) => Err(e),
    }
}

fn find_free_display() -> Option<u32> {
    for n in 99..140 {
        let socket = PathBuf::from(format!("/tmp/.X11-unix/X{n}"));
        let lock = PathBuf::from(format!("/tmp/.X{n}-lock"));
        if !socket.exists() && !lock.exists() {
            return Some(n);
        }
    }
    None
}

fn wait_for_x_socket(display: u32, timeout: std::time::Duration) -> bool {
    let socket = PathBuf::from(format!("/tmp/.X11-unix/X{display}"));
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if socket.exists() {
            // Give Xvfb a beat to accept connections.
            std::thread::sleep(std::time::Duration::from_millis(300));
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
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
/// `hyprshot -m output`, then X11 tools (`import -window root`, `scrot`,
/// `maim`) for Xvfb virtual displays. Returns the written path on success.
fn capture_screen(out_path: &PathBuf) -> Result<PathBuf, String> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
    }
    if wayland_display_present() {
        if let Some(grim) = find_tool("grim") {
            match std::process::Command::new(grim).arg(out_path).status() {
                Ok(status) if status.success() && out_path.is_file() => {
                    return Ok(out_path.clone());
                }
                Ok(status) => eprintln!("[screenshot] grim exited with {status}, trying fallback"),
                Err(e) => eprintln!("[screenshot] failed to run grim: {e}, trying fallback"),
            }
        }
        if let Some(hyprshot) = find_tool("hyprshot") {
            match std::process::Command::new(hyprshot)
                .args(["-m", "output", "-o"])
                .arg(out_path.to_string_lossy().as_ref())
                .status()
            {
                Ok(status) if status.success() && out_path.is_file() => {
                    return Ok(out_path.clone());
                }
                Ok(status) => eprintln!("[screenshot] hyprshot exited with {status}, trying fallback"),
                Err(e) => eprintln!("[screenshot] failed to run hyprshot: {e}, trying fallback"),
            }
        }
    }
    // X11 fallback (Xvfb virtual display): ImageMagick `import` ships with
    // devenv already; `scrot`/`maim` are tried when present.
    if std::env::var("DISPLAY").is_ok() {
        if let Some(import) = find_tool("import") {
            match std::process::Command::new(import)
                .args(["-window", "root"])
                .arg(out_path)
                .status()
            {
                Ok(status) if status.success() && out_path.is_file() => {
                    return Ok(out_path.clone());
                }
                Ok(status) => eprintln!("[screenshot] import exited with {status}, trying fallback"),
                Err(e) => eprintln!("[screenshot] failed to run import: {e}, trying fallback"),
            }
        }
        if let Some(scrot) = find_tool("scrot") {
            match std::process::Command::new(scrot).arg(out_path).status() {
                Ok(status) if status.success() && out_path.is_file() => {
                    return Ok(out_path.clone());
                }
                Ok(status) => eprintln!("[screenshot] scrot exited with {status}, trying fallback"),
                Err(e) => eprintln!("[screenshot] failed to run scrot: {e}, trying fallback"),
            }
        }
        if let Some(maim) = find_tool("maim") {
            match std::process::Command::new(maim).arg(out_path).status() {
                Ok(status) if status.success() && out_path.is_file() => {
                    return Ok(out_path.clone());
                }
                Ok(status) => eprintln!("[screenshot] maim exited with {status}"),
                Err(e) => eprintln!("[screenshot] failed to run maim: {e}"),
            }
        }
    }
    Err("no screenshot tool found (install `grim` or `hyprshot` for Wayland, `imagemagick` for Xvfb)".to_string())
}

fn wayland_display_present() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok_and(|v| !v.is_empty())
        || std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok_and(|v| !v.is_empty())
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

    #[test]
    fn screenshot_virtual_implies_screenshot() {
        let a = parsed(&["--screenshot-virtual"]);
        assert!(a.screenshot);
        assert!(a.screenshot_virtual);
        assert_eq!((a.screenshot_width, a.screenshot_height), (1280, 800));
    }

    #[test]
    fn virtual_display_alias_sets_virtual() {
        let a = parsed(&["--virtual-display"]);
        assert!(a.screenshot);
        assert!(a.screenshot_virtual);
    }

    #[test]
    fn screenshot_size_sets_virtual_and_geometry() {
        let a = parsed(&["--screenshot-size", "1920x1080"]);
        assert!(a.screenshot);
        assert!(a.screenshot_virtual);
        assert_eq!((a.screenshot_width, a.screenshot_height), (1920, 1080));
    }

    #[test]
    fn screenshot_size_equals_form() {
        let a = parsed(&["--screenshot-size=800x600"]);
        assert!(a.screenshot_virtual);
        assert_eq!((a.screenshot_width, a.screenshot_height), (800, 600));
    }

    #[test]
    fn screenshot_size_invalid_ignored() {
        let a = parsed(&["--screenshot-size", "bogus"]);
        assert!(!a.screenshot_virtual);
    }

    #[test]
    fn parse_screenshot_size_accepts_variants() {
        assert_eq!(parse_dimensions("1280x800"), Some((1280, 800)));
        assert_eq!(parse_dimensions("1280,800"), Some((1280, 800)));
        assert_eq!(parse_dimensions("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_dimensions("bogus"), None);
        assert_eq!(parse_dimensions("10x10"), None);
    }

    #[test]
    fn screenshot_and_size_combine() {
        let a = parsed(&["--screenshot", "--screenshot-size=1920x1080"]);
        assert!(a.screenshot);
        assert!(a.screenshot_virtual);
        assert_eq!((a.screenshot_width, a.screenshot_height), (1920, 1080));
    }

    #[test]
    fn window_size_space_and_equals_forms() {
        let a = parsed(&["--window-size", "1600x900"]);
        assert_eq!((a.window_width, a.window_height), (Some(1600), Some(900)));
        assert_eq!(a.window_size_override(), Some((1600.0, 900.0)));
        assert!(!a.screenshot);
        let b = parsed(&["--window-size=800x600"]);
        assert_eq!((b.window_width, b.window_height), (Some(800), Some(600)));
        let c = parsed(&["--window-geometry=1024x768"]);
        assert_eq!((c.window_width, c.window_height), (Some(1024), Some(768)));
    }

    #[test]
    fn window_size_invalid_ignored() {
        let a = parsed(&["--window-size", "bogus"]);
        assert_eq!(a.window_width, None);
        assert_eq!(a.window_size_override(), None);
    }

    #[test]
    fn window_size_default_is_none() {
        let a = parsed(&["--screenshot"]);
        assert_eq!(a.window_size_override(), None);
    }
}
