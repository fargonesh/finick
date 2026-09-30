use {
    config::ty::App,
    freya::prelude::*,
    std::{
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    },
    ui::*,
};

static SEARCH_GEN: AtomicU64 = AtomicU64::new(0);

pub fn launcher_lock_path() -> PathBuf {
    PathBuf::from("/tmp/finick-launcher.lock")
}

fn pid_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

pub fn check_single_instance_toggle() -> bool {
    let lock = launcher_lock_path();
    if let Ok(old) = std::fs::read_to_string(&lock) {
        if let Ok(pid) = old.trim().parse::<u32>() {
            if pid != std::process::id() && pid_alive(pid) {
                let _ = Command::new("kill").arg(pid.to_string()).output();
                let _ = std::fs::remove_file(&lock);
                return true;
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

pub fn focus_launcher_best_effort() {
    std::thread::spawn(|| {
        let _ = Command::new("hyprctl")
            .args(["dispatch", "focuswindow", "class:^(launcher)$"])
            .output();
    });
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Calculator,
    FinickCommand,
}

pub fn provider_kind(query: &str) -> Option<ProviderKind> {
    let t = query.trim();
    if t.starts_with('=') || t.starts_with("calc ") {
        return Some(ProviderKind::Calculator);
    }
    if t.starts_with(':') {
        return Some(ProviderKind::FinickCommand);
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

pub fn provider_results(query: &str) -> Vec<index::ty::SearchResult> {
    match provider_kind(query) {
        Some(ProviderKind::Calculator) => calc_result(query).into_iter().collect(),
        Some(ProviderKind::FinickCommand) => finick_cmd_results(query),
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

fn dispatch_exec(args: &[String]) {
    if args.is_empty() {
        return;
    }
    let cmd = args.join(" ");
    let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &cmd]).spawn();
}

fn copy_to_clipboard(text: &str) {
    for (bin, args) in [("wl-copy", Vec::new()), ("xclip", vec!["-selection", "clipboard"])] {
        if Command::new(bin).args(&args).arg(text).spawn().is_ok() {
            return;
        }
    }
}

pub fn launch_item(r: &index::ty::SearchResult) {
    if let Some(val) = r.path.strip_prefix("calc:") {
        copy_to_clipboard(val);
        exit_launcher(0);
    }
    if let Some(cmd) = r.path.strip_prefix("finick:") {
        match cmd {
            "lock" => {
                if Command::new("locker").spawn().is_err() {
                    let _ = Command::new("loginctl").args(["lock-session"]).spawn();
                }
            }
            "settings" => {
                let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", "finick-settings"]).spawn();
            }
            "files" => {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
                let _ = Command::new("xdg-open").arg(home).spawn();
            }
            _ => {}
        }
        exit_launcher(0);
    }
    index::bump_recent(r);
    if r.is_desktop && r.path.ends_with(".desktop") {
        if let Some(launch) = parse_desktop_launch(&r.path) {
            if launch.terminal {
                let term = default_terminal();
                let mut full = vec![term, "-e".to_string()];
                full.extend(launch.args);
                dispatch_exec(&full);
            } else {
                dispatch_exec(&launch.args);
            }
            exit_launcher(0);
        }
        let _ = Command::new("xdg-open").arg(&r.path).spawn();
    } else if r.is_dir {
        let _ = Command::new("xdg-open").arg(&r.path).spawn();
    } else if r.is_executable {
        let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &r.path]).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(&r.path).spawn();
    }
    exit_launcher(0);
}

pub fn launch_item_secondary(r: &index::ty::SearchResult) {
    if r.path.starts_with("calc:") || r.path.starts_with("finick:") {
        launch_item(r);
        return;
    }
    index::bump_recent(r);
    let p = PathBuf::from(&r.path);
    let target = if r.is_dir { p } else { p.parent().map(|x| x.to_path_buf()).unwrap_or(PathBuf::from("/")) };
    let _ = Command::new("xdg-open").arg(&target).spawn();
    exit_launcher(0);
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
        let res = ipsea::send_command(
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

pub fn launcher_app() -> Element {
    let _st = use_init_app_theme(get_theme());
    let t = use_app_theme();
    let query = use_state(String::new);
    let results = use_state(Vec::<index::ty::SearchResult>::new);
    let selected = use_state(|| 0usize);
    let input_id = use_hook(AccessibilityId::new_unique);
    use_hook(move || {
        let id = input_id;
        spawn(async move {
            focus_launcher_best_effort();
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            id.request_focus();
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
    let sel = *selected.read();
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(Color::from_argb(90, 0, 0, 0))
        .padding(24.)
        .center()
        .content(Content::Flex)
        .on_global_key_down({
            let mut sel_state = selected;
            let mut q_state = query;
            let res_state = results;
            move |e: Event<KeyboardEventData>| {
                let len = res_state.read().len();
                let cur = *sel_state.read();
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
                    Key::Named(NamedKey::Escape) => exit_launcher(0),
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
                            if !item.path.starts_with("calc:") && !item.path.starts_with("finick:") {
                                q_state.set(item.name.clone());
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if e.data().modifiers.shift() {
                            if let Some(item) = res_state.read().get(cur).cloned() {
                                launch_item_secondary(&item);
                            }
                            return;
                        }
                        if let Some(item) = res_state.read().get(cur).cloned() {
                            launch_item(&item);
                        } else if !q_state.read().trim().is_empty() {
                            let q = q_state.read().trim().to_string();
                            if let Some(v) = q.strip_prefix('=').and_then(try_eval_expr) {
                                copy_to_clipboard(&format_calc(v));
                                exit_launcher(0);
                            }
                            let _ = Command::new("hyprctl").args(["dispatch", "exec", "--", &q]).spawn();
                            exit_launcher(0);
                        }
                    }
                    _ => {}
                }
            }
        })
        .child(
            FadeSlideIn::new().child(
                rect()
                    .width(Size::px(640.))
                    .corner_radius(20.)
                    .background(t.panel)
                    .border(Border::new().width(1.).fill(t.border))
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
                                    .child(sidebar_search(query, "Search apps, files…  (= calc, : cmd)")),
                            )
                            .maybe(!query.read().is_empty(), |el| {
                                let mut q = query;
                                el.child(small_close_button(move || q.set(String::new())))
                            }),
                    )
                    .child(rect().width(Size::fill()).height(Size::px(320.)).content(Content::Flex).child(
                        if results_len == 0 {
                            if query.read().trim().is_empty() {
                                empty_state(APPS, "Type to search apps & files", "Recents appear here").into_element()
                            } else {
                                empty_state(APPS, "No results", "Press Enter to run as command").into_element()
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
                                        let icon_path = item.icon.clone().unwrap_or_default();
                                        let has_img = !icon_path.is_empty() && PathBuf::from(&icon_path).exists();
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
                                            .on_press(move |_| launch_item(&item_for_press))
                                            .child(
                                                rect()
                                                    .width(Size::px(36.))
                                                    .height(Size::px(36.))
                                                    .corner_radius(10.)
                                                    .background(if is_sel { t.panel } else { t.bg })
                                                    .border(Border::new().width(1.).fill(t.border))
                                                    .center()
                                                    .child(if has_img {
                                                        ImageViewer::new(PathBuf::from(icon_path.clone()))
                                                            .width(Size::px(20.))
                                                            .height(Size::px(20.))
                                                            .into_element()
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
                                                        label()
                                                            .font_size(11.)
                                                            .color(t.text_dim)
                                                            .text(row_meta(&item_for_label)),
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
                                "↑↓/Ctrl+J/K Navigate  Tab Complete  ⏎ Launch  ⇧⏎ Reveal  Esc Close",
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
