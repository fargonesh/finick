use {
    config::ty::App,
    freya::prelude::*,
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
        process::Command,
        sync::{
            OnceLock,
            atomic::{AtomicU64, Ordering},
        },
    },
    ui::*,
};

static SEARCH_GEN: AtomicU64 = AtomicU64::new(0);

/// Maps lowercase executable basename -> raw Icon value from the .desktop entry
/// that launches it. Built once per process by scanning the standard
/// applications directories (bounded, name/exec/icon lines only).
static DESKTOP_ICON_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();

fn desktop_icon_map() -> &'static HashMap<String, String> {
    DESKTOP_ICON_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        let mut app_dirs: Vec<PathBuf> = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            app_dirs.push(PathBuf::from(format!("{home}/.local/share/applications")));
            app_dirs.push(PathBuf::from(format!("{home}/.nix-profile/share/applications")));
        }
        for base in [
            "/usr/share/applications",
            "/usr/local/share/applications",
            "/run/current-system/sw/share/applications",
        ] {
            app_dirs.push(PathBuf::from(base));
        }
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            app_dirs.push(PathBuf::from(xdg).join("applications"));
        }
        let mut files: Vec<PathBuf> = Vec::new();
        for dir in &app_dirs {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for entry in entries.filter_map(|e| e.ok()).take(500) {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "desktop") {
                    files.push(p);
                    if files.len() >= 1500 {
                        break;
                    }
                }
            }
        }
        for path in files {
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            let mut in_entry = false;
            let mut icon: Option<String> = None;
            let mut execs: Vec<String> = Vec::new();
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with('[') && t.ends_with(']') {
                    in_entry = t == "[Desktop Entry]";
                    continue;
                }
                if !in_entry {
                    continue;
                }
                if let Some((k, v)) = t.split_once('=') {
                    match k.trim() {
                        "Icon" if icon.is_none() => {
                            let v = v.trim().to_string();
                            if !v.is_empty() {
                                icon = Some(v);
                            }
                        }
                        "Exec" => {
                            let first = v.trim().split_whitespace().next().unwrap_or("").trim_matches('"');
                            let base = first.rsplit('/').next().unwrap_or(first);
                            if !base.is_empty() {
                                execs.push(base.to_lowercase());
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(icon) = icon {
                for exec in execs {
                    map.entry(exec).or_insert_with(|| icon.clone());
                }
            }
        }
        map
    })
}

fn theme_icon_path(stem: &str) -> Option<PathBuf> {
    let stem = stem.trim();
    if stem.is_empty() {
        return None;
    }
    let lower = stem.to_lowercase();
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(format!("{home}/.icons")));
        dirs.push(PathBuf::from(format!("{home}/.local/share/icons")));
        // Icons the index service rasterized from desktop entries.
        dirs.push(config::finick_root().join("icons"));
    }
    for base in ["/usr/share/icons", "/usr/local/share/icons", "/usr/share/pixmaps", "/run/current-system/sw/share/icons"]
    {
        dirs.push(PathBuf::from(base));
    }
    for dir in &dirs {
        for ext in ["png", "svg", "xpm"] {
            let candidate = dir.join(format!("{stem}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
            // Case-insensitive match for the finick cache (desktop Name casing).
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.to_lowercase() == format!("{lower}.{ext}") {
                        let p = entry.path();
                        if p.is_file() {
                            return Some(p);
                        }
                    }
                }
            }
        }
        for theme in ["hicolor", "Adwaita", "gnome", "breeze", "Papirus"] {
            for size in ["48x48", "64x64", "scalable", "symbolic"] {
                for ext in ["png", "svg", "xpm"] {
                    let candidate = dir.join(theme).join(size).join("apps").join(format!("{stem}.{ext}"));
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

fn desktop_file_icon(desktop_path: &str) -> Option<PathBuf> {
    let content = std::fs::read_to_string(desktop_path).ok()?;
    let mut in_entry = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_entry = t == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(icon) = t.strip_prefix("Icon=") {
            let icon = icon.trim();
            if icon.is_empty() {
                return None;
            }
            if icon.contains('/') {
                let p = PathBuf::from(icon);
                if p.is_file() {
                    return Some(p);
                }
                if !icon.ends_with(".png") && !icon.ends_with(".svg") && !icon.ends_with(".xpm") {
                    for ext in ["png", "svg", "xpm"] {
                        let p = PathBuf::from(format!("{icon}.{ext}"));
                        if p.is_file() {
                            return Some(p);
                        }
                    }
                }
                return None;
            }
            return theme_icon_path(icon);
        }
    }
    None
}

/// Best-effort icon for a result row. Index rows from .desktop entries already
/// carry a cached png; PATH binaries and bare files do not, so fall back to
/// the owning .desktop entry and then to a theme lookup by binary name.
pub fn effective_icon_path(r: &index::ty::SearchResult) -> Option<PathBuf> {
    if let Some(p) = r.icon.clone() {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if r.is_desktop && r.path.ends_with(".desktop") {
        if let Some(p) = desktop_file_icon(&r.path) {
            return Some(p);
        }
    }
    let bin = Path::new(&r.path).file_name()?.to_string_lossy().to_lowercase();
    if let Some(icon) = desktop_icon_map().get(bin.as_str()) {
        if icon.contains('/') {
            let p = PathBuf::from(icon);
            if p.is_file() {
                return Some(p);
            }
        } else if let Some(p) = theme_icon_path(icon) {
            return Some(p);
        }
    }
    theme_icon_path(&bin)
}

fn row_icon(path: &Path, size: f32) -> Element {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext == "svg" {
        SvgViewer::new(path.to_path_buf()).width(Size::px(size)).height(Size::px(size)).into_element()
    } else {
        ImageViewer::new(path.to_path_buf()).width(Size::px(size)).height(Size::px(size)).into_element()
    }
}

pub fn launcher_lock_path() -> PathBuf {
    PathBuf::from("/tmp/finick-launcher.lock")
}

fn pid_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

pub fn check_single_instance_toggle() -> bool {
    let lock = launcher_lock_path();
    if let Ok(old) = std::fs::read_to_string(&lock) {
        match old.trim().parse::<u32>() {
            Ok(pid) if pid != std::process::id() && pid_alive(pid) => {
                let _ = Command::new("kill").arg(pid.to_string()).output();
                for _ in 0..30 {
                    if !pid_alive(pid) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                let _ = std::fs::remove_file(&lock);
                return true;
            }
            _ => {
                let _ = std::fs::remove_file(&lock);
            }
        }
    }
    let _ = std::fs::write(&lock, std::process::id().to_string());
    focus_launcher_best_effort();
    false
}

fn exit_launcher(code: i32) -> ! {
    let _ = std::fs::remove_file(launcher_lock_path());
    std::process::exit(code);
}

pub fn ensure_launcher_window_rules() {
    let lua = r#"
        hl.window_rule({
            name = "finick-launcher",
            match = { class = "^(launcher)$" },
            float = true,
            pin = true,
            border_size = 0,
            no_shadow = true,
            no_anim = true,
            no_blur = true,
            rounding = 20,
        })
    "#;
    let _ = Command::new("hyprctl").args(["eval", lua]).output();
}

/// Best-effort Hyprland focus steal for the freshly spawned launcher.
///
/// Honest stub: there is no cross-compositor focus protocol here, only
/// `hyprctl` window rules plus bring-to-top/focus dispatches. Cannot be
/// verified live in this environment (no Hyprland socket), so failures are
/// silently ignored and the window still maps centered.
pub fn focus_launcher_best_effort() {
    ensure_launcher_window_rules();
    std::thread::spawn(|| {
        let _ = Command::new("hyprctl").args(["dispatch", "bringactivetotop", "class:^(launcher)$"]).output();
        let _ = Command::new("hyprctl").args(["dispatch", "focuswindow", "class:^(launcher)$"]).output();
    });
}

/// Clamps a selection index into `[0, len)`. Returns 0 for empty lists so
/// stale indexes from a previous, longer result set can never panic or
/// point past the end after a new search lands.
pub fn clamp_selection(len: usize, cur: usize) -> usize {
    if len == 0 { 0 } else { cur.min(len - 1) }
}

fn notify_usage_daemon(r: &index::ty::SearchResult) {
    let owned = r.clone();
    std::thread::spawn(move || {
        let req = index::ty::Request::RecordUsage { result: owned };
        let _ = ipc::send_command::<index::ty::Request, index::ty::SearchResult, fn(index::ty::SearchResult)>(
            App::IndexService,
            &req,
            None,
        );
    });
}

fn bump_usage(r: &index::ty::SearchResult) {
    index::bump_recent(r);
    notify_usage_daemon(r);
}

struct CalcParser {
    chars: Vec<char>,
    pos: usize,
}

impl CalcParser {
    fn new(s: &str) -> Self {
        Self { chars: s.chars().collect(), pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn parse(&mut self) -> Option<f64> {
        let v = self.parse_add()?;
        self.eat_ws();
        if self.pos != self.chars.len() {
            return None;
        }
        Some(v)
    }

    fn parse_add(&mut self) -> Option<f64> {
        let mut v = self.parse_mul()?;
        loop {
            self.eat_ws();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    v += self.parse_mul()?;
                }
                Some('-') => {
                    self.pos += 1;
                    v -= self.parse_mul()?;
                }
                _ => break,
            }
        }
        Some(v)
    }

    fn parse_mul(&mut self) -> Option<f64> {
        let mut v = self.parse_pow()?;
        loop {
            self.eat_ws();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    v *= self.parse_pow()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let r = self.parse_pow()?;
                    v /= r;
                }
                Some('%') => {
                    self.pos += 1;
                    let r = self.parse_pow()?;
                    v %= r;
                }
                _ => break,
            }
        }
        Some(v)
    }

    fn parse_pow(&mut self) -> Option<f64> {
        let mut v = self.parse_unary()?;
        self.eat_ws();
        if self.peek() == Some('^') {
            self.pos += 1;
            let e = self.parse_pow()?;
            v = v.powf(e);
        }
        Some(v)
    }

    fn parse_unary(&mut self) -> Option<f64> {
        self.eat_ws();
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                Some(-self.parse_unary()?)
            }
            Some('+') => {
                self.pos += 1;
                self.parse_unary()
            }
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> Option<f64> {
        self.eat_ws();
        if self.peek() == Some('(') {
            self.pos += 1;
            let v = self.parse_add()?;
            self.eat_ws();
            if self.peek() != Some(')') {
                return None;
            }
            self.pos += 1;
            return Some(v);
        }
        let start = self.pos;
        let mut dot = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == '.' && !dot {
                dot = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if start == self.pos {
            return None;
        }
        self.chars[start..self.pos].iter().collect::<String>().parse().ok()
    }
}

pub fn try_eval_expr(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    if !t.chars().all(|c| c.is_ascii_digit() || "+-*/%^(). \t".contains(c)) {
        return None;
    }
    if !t.chars().any(|c| "+-*/%^".contains(c)) {
        return None;
    }
    let v = CalcParser::new(t).parse()?;
    if !v.is_finite() {
        return None;
    }
    Some(v)
}

fn format_calc(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProviderKind {
    Calculator,
    FinickCommand,
    ShellCommand,
}

pub fn provider_kind(query: &str) -> Option<ProviderKind> {
    let t = query.trim();
    if t.starts_with('=') || t.starts_with("calc ") {
        return Some(ProviderKind::Calculator);
    }
    if t.starts_with(':') {
        return Some(ProviderKind::FinickCommand);
    }
    if t.starts_with('!') {
        return Some(ProviderKind::ShellCommand);
    }
    None
}

fn calc_result(query: &str) -> Option<index::ty::SearchResult> {
    let t = query.trim();
    let expr = if let Some(rest) = t.strip_prefix('=') {
        rest.trim()
    } else if let Some(rest) = t.strip_prefix("calc ") {
        rest.trim()
    } else {
        return None;
    };
    let v = try_eval_expr(expr)?;
    let s = format_calc(v);
    Some(index::ty::SearchResult {
        name: format!("= {s}"),
        path: format!("calc:{s}"),
        is_desktop: false,
        is_executable: false,
        icon: None,
        is_dir: false,
        size: None,
        modified: None,
    })
}

fn finick_cmd_results(query: &str) -> Vec<index::ty::SearchResult> {
    let all = [
        ("Lock screen", "finick:lock", "Lock the session now"),
        ("Settings", "finick:settings", "Open Finick settings"),
        ("Files", "finick:files", "Open the file manager"),
    ];
    let t = query.trim().to_lowercase();
    let needle = t.strip_prefix(':').unwrap_or(&t);
    all.iter()
        .filter(|(name, _, _)| name.to_lowercase().contains(needle) || needle.is_empty())
        .map(|(name, path, _)| index::ty::SearchResult {
            name: format!(":{name}"),
            path: path.to_string(),
            is_desktop: false,
            is_executable: true,
            icon: None,
            is_dir: false,
            size: None,
            modified: None,
        })
        .collect()
}

fn shell_cmd_result(query: &str) -> Option<index::ty::SearchResult> {
    let t = query.trim();
    let cmd = t.strip_prefix('!')?.trim();
    if cmd.is_empty() {
        return None;
    }
    Some(index::ty::SearchResult {
        name: format!("Run command: {cmd}"),
        path: format!("shell:{cmd}"),
        is_desktop: false,
        is_executable: true,
        icon: None,
        is_dir: false,
        size: None,
        modified: None,
    })
}

pub fn shell_command_for_query(query: &str) -> Option<String> {
    let t = query.trim();
    let cmd = t.strip_prefix('!')?.trim();
    if cmd.is_empty() { None } else { Some(cmd.to_string()) }
}

pub fn provider_results(query: &str) -> Vec<index::ty::SearchResult> {
    match provider_kind(query) {
        Some(ProviderKind::Calculator) => calc_result(query).into_iter().collect(),
        Some(ProviderKind::FinickCommand) => finick_cmd_results(query),
        Some(ProviderKind::ShellCommand) => shell_cmd_result(query).into_iter().collect(),
        None => Vec::new(),
    }
}

fn split_exec_quoted(exec: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut esc = false;
    for c in exec.chars() {
        if esc {
            cur.push(c);
            esc = false;
            continue;
        }
        if c == '\\' && quote != Some('\'') {
            esc = true;
            continue;
        }
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(cur.clone());
                    cur.clear();
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_field_code(tok: &str) -> bool {
    matches!(tok, "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%i" | "%c" | "%k" | "%v" | "%m")
        || (tok.starts_with('%') && tok.len() == 2)
}

fn tryexec_ok(val: &str) -> bool {
    let t = val.trim();
    if t.is_empty() {
        return true;
    }
    let first = t.split_whitespace().next().unwrap_or(t);
    if first.starts_with('/') {
        return PathBuf::from(first).exists();
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join(first).exists() {
                return true;
            }
        }
    }
    false
}

fn default_terminal() -> String {
    if let Ok(t) = std::env::var("TERMINAL") {
        if !t.trim().is_empty() {
            return t;
        }
    }
    for cand in ["foot", "ghostty", "kitty", "alacritty", "x-terminal-emulator", "xterm"] {
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                if dir.join(cand).exists() {
                    return cand.to_string();
                }
            }
        }
    }
    "x-terminal-emulator".to_string()
}

struct DesktopLaunch {
    args: Vec<String>,
    terminal: bool,
}

fn parse_desktop_launch(desktop_path: &str) -> Option<DesktopLaunch> {
    let content = std::fs::read_to_string(desktop_path).ok()?;
    let mut in_entry = false;
    let mut exec: Option<String> = None;
    let mut try_exec: Option<String> = None;
    let mut terminal = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_entry = t == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            match k.trim() {
                "Exec" if exec.is_none() => exec = Some(v.trim().to_string()),
                "TryExec" if try_exec.is_none() => try_exec = Some(v.trim().to_string()),
                "Terminal" => terminal = v.trim().eq_ignore_ascii_case("true"),
                "NoDisplay" | "Hidden" if v.trim().eq_ignore_ascii_case("true") => return None,
                _ => {}
            }
        }
    }
    if let Some(te) = try_exec {
        if !tryexec_ok(&te) {
            return None;
        }
    }
    let raw = exec?;
    let mut args: Vec<String> = split_exec_quoted(&raw).into_iter().filter(|a| !is_field_code(a)).collect();
    if args.is_empty() {
        return None;
    }
    if args[0].contains('/') && !PathBuf::from(&args[0]).exists() {
        let base = PathBuf::from(&args[0]).file_name().map(|s| s.to_string_lossy().to_string());
        if let Some(b) = base {
            args[0] = b;
        }
    }
    Some(DesktopLaunch { args, terminal })
}

fn dispatch_exec(args: &[String]) -> bool {
    if args.is_empty() {
        return false;
    }
    let cmd = args.join(" ");
    run_hyprctl_exec(&cmd)
}

fn run_hyprctl_exec(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return false;
    }
    Command::new("hyprctl").args(["dispatch", "exec", "--", trimmed]).spawn().is_ok()
}

fn run_xdg_open(path: &str) -> bool {
    if path.trim().is_empty() {
        return false;
    }
    Command::new("xdg-open").arg(path).spawn().is_ok()
}

fn copy_to_clipboard(text: &str) {
    for (bin, args) in [("wl-copy", Vec::new()), ("xclip", vec!["-selection", "clipboard"])] {
        if Command::new(bin).args(&args).arg(text).spawn().is_ok() {
            return;
        }
    }
}

/// Returns true when the item was handed off successfully (caller should close
/// via `dismiss` so the PanelShell exit animation plays; the actual process
/// exit happens in `on_dismiss`).
/// Returns false when the spawn failed (caller should surface an error, not exit).
pub fn launch_item(r: &index::ty::SearchResult) -> bool {
    if let Some(val) = r.path.strip_prefix("calc:") {
        copy_to_clipboard(val);
        return true;
    }
    if let Some(shell_cmd) = r.path.strip_prefix("shell:") {
        let ok = run_hyprctl_exec(shell_cmd);
        if ok {
            bump_usage(r);
        }
        return ok;
    }
    if let Some(cmd) = r.path.strip_prefix("finick:") {
        let ok = match cmd {
            "lock" => {
                if let Some(exe) = system::locker_binary() {
                    Command::new(&exe).spawn().is_ok()
                } else if Command::new("finick-lock").spawn().is_ok() {
                    true
                } else if Command::new("locker").spawn().is_ok() {
                    true
                } else {
                    Command::new("loginctl").args(["lock-session"]).spawn().is_ok()
                }
            }
            "settings" => run_hyprctl_exec("finick-settings"),
            "files" => {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
                run_xdg_open(&home)
            }
            _ => false,
        };
        return ok;
    }
    let ok = if r.is_desktop && r.path.ends_with(".desktop") {
        if let Some(launch) = parse_desktop_launch(&r.path) {
            if launch.terminal {
                let term = default_terminal();
                let mut full = vec![term, "-e".to_string()];
                full.extend(launch.args);
                dispatch_exec(&full)
            } else {
                dispatch_exec(&launch.args)
            }
        } else {
            run_xdg_open(&r.path)
        }
    } else if r.is_dir {
        run_xdg_open(&r.path)
    } else if r.is_executable {
        run_hyprctl_exec(&r.path)
    } else {
        run_xdg_open(&r.path)
    };
    if ok {
        bump_usage(r);
    }
    ok
}

pub fn launch_item_secondary(r: &index::ty::SearchResult) -> bool {
    if r.path.starts_with("calc:") || r.path.starts_with("finick:") || r.path.starts_with("shell:") {
        return launch_item(r);
    }
    let p = PathBuf::from(&r.path);
    let target = if r.is_dir { p } else { p.parent().map(|x| x.to_path_buf()).unwrap_or(PathBuf::from("/")) };
    let target_s = target.to_string_lossy().to_string();
    let ok = run_xdg_open(&target_s);
    if ok {
        bump_usage(r);
    }
    ok
}

fn recent_results() -> Vec<index::ty::SearchResult> {
    index::load_recents()
        .into_iter()
        .map(|e| index::ty::SearchResult {
            name: e.name,
            path: e.path,
            is_desktop: e.is_desktop,
            is_executable: e.is_executable,
            icon: e.icon,
            is_dir: e.is_dir,
            size: None,
            modified: None,
        })
        .collect()
}

fn search(query: String, results: State<Vec<index::ty::SearchResult>>, selected: State<usize>, my_gen: u64) {
    let providers = provider_results(&query);
    if provider_kind(&query).is_some() && !providers.is_empty() {
        let mut selected = selected;
        let mut results = results;
        if SEARCH_GEN.load(Ordering::SeqCst) == my_gen {
            selected.set(0);
            results.set(providers);
        }
        return;
    }
    if query.trim().is_empty() {
        let mut selected = selected;
        let mut results = results;
        if SEARCH_GEN.load(Ordering::SeqCst) == my_gen {
            selected.set(0);
            results.set(recent_results());
        }
        return;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let q = query.clone();
    std::thread::spawn(move || {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let inner_tx2 = inner_tx.clone();
        let res = ipc::send_command(
            App::IndexService,
            &index::ty::Request::Search { query: q },
            Some(move |r: index::ty::SearchResult| {
                let _ = inner_tx2.send(r);
            }),
        );
        drop(inner_tx);
        let mut items: Vec<index::ty::SearchResult> = inner_rx.into_iter().collect();
        if res.is_err() {}
        items.truncate(50);
        let _ = tx.send((providers, items));
    });
    spawn(async move {
        let mut selected = selected;
        let mut results = results;
        if let Some((providers, items)) = rx.recv().await {
            if SEARCH_GEN.load(Ordering::SeqCst) != my_gen {
                return;
            }
            let mut merged = providers;
            merged.extend(items);
            merged.truncate(50);
            selected.set(0);
            results.set(merged);
        }
    });
}

fn format_size(size: Option<u64>) -> Option<String> {
    let s = size?;
    if s < 1024 {
        Some(format!("{s} B"))
    } else if s < 1024 * 1024 {
        Some(format!("{:.1} KB", s as f64 / 1024.0))
    } else if s < 1024 * 1024 * 1024 {
        Some(format!("{:.1} MB", s as f64 / 1024.0 / 1024.0))
    } else {
        Some(format!("{:.1} GB", s as f64 / 1024.0 / 1024.0 / 1024.0))
    }
}

fn format_modified(modified: Option<u64>) -> Option<String> {
    let ts = modified? as i64;
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.format("%Y-%m-%d").to_string())
}

fn row_meta(r: &index::ty::SearchResult) -> String {
    if let Some(cmd) = r.path.strip_prefix("shell:") {
        return format!("! {cmd}  •  Run via hyprctl");
    }
    if let Some(val) = r.path.strip_prefix("calc:") {
        return format!("= {val}  •  Copy result");
    }
    if r.path.starts_with("finick:") {
        return "Finick command  •  Runs immediately".to_string();
    }
    let mut meta = r.path.clone();
    let extra: Vec<String> = [format_size(r.size), format_modified(r.modified)].into_iter().flatten().collect();
    if !extra.is_empty() {
        meta.push_str(&format!("  •  {}", extra.join("  •  ")));
    }
    meta
}

fn kind_icon(r: &index::ty::SearchResult) -> &'static str {
    if r.path.starts_with("calc:") {
        COPY
    } else if r.path.starts_with("finick:") {
        GENERAL
    } else if r.path.starts_with("shell:") {
        TERMINAL
    } else if r.is_desktop {
        APPS
    } else if r.is_dir {
        FOLDER
    } else if r.is_executable {
        TERMINAL
    } else {
        FILE
    }
}

fn kind_label(r: &index::ty::SearchResult) -> &'static str {
    if r.path.starts_with("calc:") {
        "Calc"
    } else if r.path.starts_with("finick:") {
        "Cmd"
    } else if r.path.starts_with("shell:") {
        "Run"
    } else if r.is_desktop {
        "App"
    } else if r.is_dir {
        "Folder"
    } else if r.is_executable {
        "Exec"
    } else {
        "File"
    }
}

fn secondary_hint(r: &index::ty::SearchResult) -> &'static str {
    if r.path.starts_with("calc:") {
        "⏎ Copy"
    } else if r.path.starts_with("shell:") {
        "⏎ Run command"
    } else if r.path.starts_with("finick:") {
        "⏎ Run"
    } else if r.is_dir {
        "⏎ Open  ·  ⇧⏎ Reveal"
    } else {
        "⏎ Open  ·  ⇧⏎ Reveal in folder"
    }
}

pub fn launcher_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let query = use_state(String::new);
    let results = use_state(Vec::<index::ty::SearchResult>::new);
    let selected = use_state(|| 0usize);
    let launch_error = use_state(|| Option::<String>::None);
    let dismiss = use_state(|| false);
    use_hook(move || {
        crate::state::spawn_panel_focus_watcher(&["launcher"], "launcher", dismiss);
    });
    use_hook(move || {
        spawn(async move {
            focus_launcher_best_effort();
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            let _ = Command::new("hyprctl").args(["dispatch", "focuswindow", "class:^(launcher)$"]).output();
        });
    });
    let r_side = results;
    let s_side = selected;
    let q_side = query;
    use_side_effect(move || {
        let q = q_side.read().clone();
        let my = SEARCH_GEN.fetch_add(1, Ordering::SeqCst) + 1;
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if SEARCH_GEN.load(Ordering::SeqCst) != my {
                return;
            }
            search(q, r_side, s_side, my);
        });
    });
    let results_len = results.read().len();
    let sel = clamp_selection(results_len, *selected.read());
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(Color::TRANSPARENT)
        .padding(12.)
        .center()
        .content(Content::Flex)
        .on_global_key_down({
            let mut sel_state = selected;
            let mut q_state = query;
            let res_state = results;
            let mut err_state = launch_error;
            let mut dismiss_state = dismiss;
            move |e: Event<KeyboardEventData>| {
                if *dismiss_state.read() {
                    return;
                }
                let len = res_state.read().len();
                let cur = clamp_selection(len, *sel_state.read());
                if e.data().modifiers.ctrl() {
                    match e.data().key.clone() {
                        Key::Character(ref s) if s == "j" => {
                            if len > 0 {
                                sel_state.set((cur + 1) % len);
                            }
                            return;
                        }
                        Key::Character(ref s) if s == "k" => {
                            if len > 0 {
                                sel_state.set(if cur == 0 { len - 1 } else { cur - 1 });
                            }
                            return;
                        }
                        _ => {}
                    }
                }
                match e.data().key.clone() {
                    Key::Named(NamedKey::Escape) => dismiss_state.set(true),
                    Key::Named(NamedKey::ArrowDown) => {
                        if len > 0 {
                            sel_state.set((cur + 1) % len)
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if len > 0 {
                            sel_state.set(if cur == 0 { len - 1 } else { cur - 1 })
                        }
                    }
                    Key::Named(NamedKey::Tab) => {
                        if let Some(item) = res_state.read().get(cur).cloned() {
                            if !item.path.starts_with("calc:")
                                && !item.path.starts_with("finick:")
                                && !item.path.starts_with("shell:")
                            {
                                q_state.set(item.name.clone());
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if e.data().modifiers.shift() {
                            if let Some(item) = res_state.read().get(cur).cloned() {
                                if !launch_item_secondary(&item) {
                                    err_state.set(Some("Could not reveal — xdg-open failed.".to_string()));
                                } else {
                                    dismiss_state.set(true);
                                }
                            }
                            return;
                        }
                        if let Some(item) = res_state.read().get(cur).cloned() {
                            if !launch_item(&item) {
                                err_state.set(Some("Launch failed — hyprctl/xdg-open unavailable.".to_string()));
                            } else {
                                dismiss_state.set(true);
                            }
                        } else if !q_state.read().trim().is_empty() {
                            let q = q_state.read().trim().to_string();
                            if let Some(v) = q.strip_prefix('=').and_then(try_eval_expr) {
                                copy_to_clipboard(&format_calc(v));
                                dismiss_state.set(true);
                                return;
                            }
                            // Do not silently exec arbitrary text: require explicit `!` prefix.
                            if let Some(cmd) = shell_command_for_query(&q) {
                                if !run_hyprctl_exec(&cmd) {
                                    err_state.set(Some("Could not run command — hyprctl failed.".to_string()));
                                } else {
                                    dismiss_state.set(true);
                                }
                            } else {
                                err_state.set(Some("Prefix with ! to run as command (e.g. ! htop).".to_string()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
        .child(
            PanelShell::new(dismiss.into_readable())
                .duration(300)
                .scale_from(1.0)
                .on_dismiss(EventHandler::new(move |()| exit_launcher(0)))
                .child(
                rect()
                    .width(Size::px(640.))
                    .corner_radius(20.)
                    .background(t.panel)
                    .border(Border::new().width(1.).fill(t.border))
                    .shadow(Shadow::new().blur(32.).color(Color::from_argb(90, 0, 0, 0)))
                    .padding(14.)
                    .spacing(10.)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .content(Content::Flex)
                                    .child(sidebar_search_with_focus(
                                        query,
                                        "Search apps, files…  (= calc, : cmd, ! run)",
                                        true,
                                    )),
                            )
                            .maybe(!query.read().is_empty(), |el| {
                                let mut q = query;
                                el.child(small_close_button(move || q.set(String::new())))
                            }),
                    )
                    .maybe_child(launch_error.read().clone().map(|msg| {
                        rect()
                            .width(Size::fill())
                            .corner_radius(10.)
                            .background(t.accent_red)
                            .padding((8., 12.))
                            .child(label().font_size(12.).font_weight(FontWeight::SEMI_BOLD).color(Color::WHITE).text(msg))
                            .into_element()
                    }))
                    .child(rect().width(Size::fill()).height(Size::px(320.)).content(Content::Flex).child(
                        if results_len == 0 {
                            if query.read().trim().is_empty() {
                                empty_state(APPS, "Type to search apps & files", "Recents appear here").into_element()
                            } else if shell_command_for_query(&query.read()).is_some() {
                                empty_state(
                                    TERMINAL,
                                    format!("Run command: {}", shell_command_for_query(&query.read()).unwrap_or_default()),
                                    "Press Enter to run via hyprctl",
                                )
                                .into_element()
                            } else {
                                empty_state(APPS, "No results", "Prefix with ! to run as command (e.g. ! htop)").into_element()
                            }
                        } else {
                            ScrollView::new()
                                .width(Size::fill())
                                .height(Size::fill())
                                .child(rect().width(Size::fill()).vertical().spacing(4.).children(
                                    results.read().iter().enumerate().map(|(idx, item)| {
                                        let is_sel = idx == sel;
                                        let bg = if is_sel { t.bg_active } else { Color::TRANSPARENT };
                                        let border = if is_sel { t.accent } else { Color::TRANSPARENT };
                                        let item_for_press = item.clone();
                                        let item_for_icon = item.clone();
                                        let item_for_label = item.clone();
                                        let item_for_meta = item.clone();
                                        let mut err_press = launch_error;
                                        let mut dismiss_press = dismiss;
                                        let icon_path = effective_icon_path(&item_for_icon);
                                        let mut meta_text = row_meta(&item_for_meta);
                                        if is_sel {
                                            meta_text.push_str(&format!("  •  {}", secondary_hint(&item_for_meta)));
                                        }
                                        rect()
                                            .width(Size::fill())
                                            .height(Size::px(56.))
                                            .corner_radius(12.)
                                            .background(bg)
                                            .border(Border::new().width(1.).fill(border))
                                            .padding((8., 12.))
                                            .horizontal()
                                            .cross_align(Alignment::Center)
                                            .content(Content::Flex)
                                            .spacing(12.)
                                            .cursor(CursorIcon::Pointer)
                                            .on_press(move |_| {
                                                if *dismiss_press.read() {
                                                    return;
                                                }
                                                if !launch_item(&item_for_press) {
                                                    err_press.set(Some(
                                                        "Launch failed — hyprctl/xdg-open unavailable.".to_string(),
                                                    ));
                                                } else {
                                                    dismiss_press.set(true);
                                                }
                                            })
                                            .child(
                                                rect()
                                                    .width(Size::px(36.))
                                                    .height(Size::px(36.))
                                                    .corner_radius(10.)
                                                    .background(if is_sel { t.panel } else { t.bg })
                                                    .border(Border::new().width(1.).fill(t.border))
                                                    .center()
                                                    .child(if let Some(p) = icon_path {
                                                        row_icon(&p, 20.)
                                                    } else {
                                                        icon(
                                                            kind_icon(&item_for_icon),
                                                            18.,
                                                            if is_sel { t.accent } else { t.text_dim },
                                                        )
                                                        .into_element()
                                                    }),
                                            )
                                            .child(
                                                rect()
                                                    .width(Size::fill())
                                                    .vertical()
                                                    .content(Content::Flex)
                                                    .child(
                                                        label()
                                                            .font_size(13.)
                                                            .font_weight(FontWeight::SEMI_BOLD)
                                                            .color(t.text)
                                                            .text(item_for_label.name.clone()),
                                                    )
                                                    .child(
                                                        label().font_size(11.).color(t.text_dim).text(meta_text),
                                                    ),
                                            )
                                            .child(status_chip(kind_label(item).to_string(), is_sel, None))
                                            .into_element()
                                    }),
                                ))
                                .into_element()
                        },
                    ))
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .content(Content::Flex)
                            .padding((8., 4., 0., 4.))
                            .child(label().font_size(11.).color(t.text_dim).text(
                                "↑↓ Navigate  Tab Complete  ⏎ Launch  ⇧⏎ Reveal in folder  ! Run  Esc Close",
                            ))
                            .child(label().font_size(11.).color(t.text_dim).text(format!("{} results", results_len))),
                    ),
            ),
        )
        .into_element()
}

pub fn launcher_window_config() -> WindowConfig {
    WindowConfig::new(launcher_app)
        .with_title("launcher")
        .with_app_id("launcher")
        .with_size(700., 540.)
        .with_min_size(640., 460.)
        .with_decorations(false)
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_basic_arithmetic() {
        assert_eq!(try_eval_expr("2+2"), Some(4.0));
        assert_eq!(try_eval_expr(" 10 - 3 "), Some(7.0));
        assert_eq!(try_eval_expr("3*4+2"), Some(14.0));
    }

    #[test]
    fn eval_respects_precedence_and_parens() {
        assert_eq!(try_eval_expr("2+3*4"), Some(14.0));
        assert_eq!(try_eval_expr("(2+3)*4"), Some(20.0));
        assert_eq!(try_eval_expr("2^3"), Some(8.0));
        assert_eq!(try_eval_expr("-4+10%3"), Some(-3.0));
    }

    #[test]
    fn eval_rejects_non_expressions() {
        assert_eq!(try_eval_expr(""), None);
        assert_eq!(try_eval_expr("42"), None);
        assert_eq!(try_eval_expr("rm -rf /"), None);
        assert_eq!(try_eval_expr("2+"), None);
        assert_eq!(try_eval_expr("(2+3"), None);
        assert_eq!(try_eval_expr("1/0"), None);
    }

    #[test]
    fn provider_routing_by_prefix() {
        assert_eq!(provider_kind("= 2+2"), Some(ProviderKind::Calculator));
        assert_eq!(provider_kind("calc 2+2"), Some(ProviderKind::Calculator));
        assert_eq!(provider_kind(":settings"), Some(ProviderKind::FinickCommand));
        assert_eq!(provider_kind("! htop"), Some(ProviderKind::ShellCommand));
        assert_eq!(provider_kind("firefox"), None);
        assert_eq!(provider_kind("  ! htop "), Some(ProviderKind::ShellCommand));
    }

    #[test]
    fn shell_command_requires_command_text() {
        assert_eq!(shell_command_for_query("! htop"), Some("htop".to_string()));
        assert_eq!(shell_command_for_query("!"), None);
        assert_eq!(shell_command_for_query("!   "), None);
        assert_eq!(shell_command_for_query("plain"), None);
    }

    #[test]
    fn calc_provider_row_copies_result() {
        let rows = provider_results("= 2+3*4");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "= 14");
        assert!(rows[0].path.starts_with("calc:"));
        assert!(provider_results("= nope").is_empty());
    }

    #[test]
    fn finick_command_provider_filters() {
        let rows = provider_results(":set");
        assert!(rows.iter().any(|r| r.path == "finick:settings"));
        assert_eq!(provider_results(":zzz-no-match").len(), 0);
    }

    #[test]
    fn exec_splitting_handles_quotes() {
        assert_eq!(split_exec_quoted("firefox --new-window"), vec!["firefox", "--new-window"]);
        assert_eq!(split_exec_quoted("\"my app\" --flag 'a b'"), vec!["my app", "--flag", "a b"]);
        assert!(is_field_code("%F"));
        assert!(is_field_code("%u"));
        assert!(!is_field_code("--file"));
    }

    #[test]
    fn desktop_launch_parses_exec_and_flags() {
        let dir = std::env::temp_dir().join("finick-launcher-test");
        let _ = std::fs::create_dir_all(&dir);
        let good = dir.join("good.desktop");
        std::fs::write(&good, "[Desktop Entry]\nName=Demo\nExec=myapp --flag %F\nTerminal=true\n").unwrap();
        let launch = parse_desktop_launch(&good.to_string_lossy()).unwrap();
        assert_eq!(launch.args, vec!["myapp", "--flag"]);
        assert!(launch.terminal);

        let hidden = dir.join("hidden.desktop");
        std::fs::write(&hidden, "[Desktop Entry]\nName=Hidden\nExec=myapp\nNoDisplay=true\n").unwrap();
        assert!(parse_desktop_launch(&hidden.to_string_lossy()).is_none());

        let missing_tryexec = dir.join("tryexec.desktop");
        std::fs::write(&missing_tryexec, "[Desktop Entry]\nName=X\nExec=myapp\nTryExec=/nonexistent-finick-binary-xyz\n").unwrap();
        assert!(parse_desktop_launch(&missing_tryexec.to_string_lossy()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selection_clamps_after_shorter_results() {
        assert_eq!(clamp_selection(0, 3), 0);
        assert_eq!(clamp_selection(5, 2), 2);
        assert_eq!(clamp_selection(5, 9), 4);
    }

    fn test_result(name: &str, path: &str) -> index::ty::SearchResult {
        index::ty::SearchResult {
            name: name.to_string(),
            path: path.to_string(),
            is_desktop: false,
            is_executable: true,
            icon: None,
            is_dir: false,
            size: None,
            modified: None,
        }
    }

    #[test]
    fn effective_icon_prefers_index_cache() {
        let dir = std::env::temp_dir().join("finick-launcher-icon-test");
        let _ = std::fs::create_dir_all(&dir);
        let png = dir.join("app.png");
        std::fs::write(&png, b"fake").unwrap();
        let mut r = test_result("demo", "/usr/bin/demo");
        r.icon = Some(png.to_string_lossy().to_string());
        assert_eq!(effective_icon_path(&r), Some(png));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_file_icon_resolves_absolute_path() {
        let dir = std::env::temp_dir().join("finick-launcher-icon-test-desktop");
        let _ = std::fs::create_dir_all(&dir);
        let png = dir.join("icon.png");
        std::fs::write(&png, b"fake").unwrap();
        let desktop = dir.join("app.desktop");
        std::fs::write(&desktop, format!("[Desktop Entry]\nName=Demo\nExec=demo\nIcon={}\n", png.display()))
            .unwrap();
        assert_eq!(desktop_file_icon(&desktop.to_string_lossy()), Some(png));
        let no_icon = dir.join("plain.desktop");
        std::fs::write(&no_icon, "[Desktop Entry]\nName=Plain\nExec=plain\n").unwrap();
        assert_eq!(desktop_file_icon(&no_icon.to_string_lossy()), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn effective_icon_never_panics_without_cache() {
        let r = test_result("no-such-app-finick-xyz", "/tmp/no-such-app-finick-xyz");
        let _ = effective_icon_path(&r);
    }
}
