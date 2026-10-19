use {
    serde::{Deserialize, Serialize},
    std::{
        fs,
        path::PathBuf,
        process::Command,
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct StoreDecls {
    #[serde(default)]
    pub nix_packages: Vec<String>,
    #[serde(default)]
    pub flatpaks: Vec<String>,
    /// Dotted program options (`programs.firefox.enable`) with JSON values.
    #[serde(default)]
    pub program_options: std::collections::HashMap<String, serde_json::Value>,
}

pub fn store_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(format!("{home}/.config/finick/store"))
    } else {
        PathBuf::from("/tmp/.finick/store")
    }
}

pub fn state_path() -> PathBuf {
    store_dir().join("store.json")
}

pub fn generated_file() -> PathBuf {
    store_dir().join("apps.nix")
}

pub fn load_decls() -> StoreDecls {
    let path = state_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(decls) = serde_json::from_str::<StoreDecls>(&content) {
            return decls;
        }
    }
    StoreDecls::default()
}

pub fn save_decls(decls: &StoreDecls) -> Result<(), String> {
    let dir = store_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir store: {e}"))?;
    let content = serde_json::to_string_pretty(decls).map_err(|e| e.to_string())?;
    fs::write(state_path(), content).map_err(|e| format!("write store.json: {e}"))?;
    write_generated_file(decls)
}

fn nix_escape_attr(attr: &str) -> String {
    attr.trim().trim_start_matches("nixpkgs#").to_string()
}

pub fn render_nix(decls: &StoreDecls) -> String {
    let mut pkgs: Vec<String> = decls.nix_packages.iter().map(|a| nix_escape_attr(a)).collect();
    pkgs.sort();
    pkgs.dedup();
    let mut flats: Vec<String> = decls.flatpaks.clone();
    flats.sort();
    flats.dedup();
    let pkgs_body = if pkgs.is_empty() {
        String::from("      # managed by Finick Apps — add packages from the Store")
    } else {
        pkgs.iter().map(|p| format!("      {p}")).collect::<Vec<_>>().join("\n")
    };
    let flat_body = flats.iter().map(|f| format!("    \"{f}\"")).collect::<Vec<_>>().join("\n");
    let flat_section = if flats.is_empty() {
        String::new()
    } else {
        format!("\n  programs.finick.store.flatpaks = [\n{flat_body}\n  ];\n")
    };
    let mut opts: Vec<String> = decls
        .program_options
        .iter()
        .filter_map(|(path, value)| crate::store_opts::nix_literal(value).map(|lit| format!("  {path} = {lit};")))
        .collect();
    opts.sort();
    let opts_section = if opts.is_empty() { String::new() } else { format!("\n{}\n", opts.join("\n")) };
    format!(
        "{{ pkgs, ... }}:\n{{\n  home.packages = with pkgs; [\n{pkgs_body}\n  ];\n{flat_section}{opts_section}}}\n"
    )
}

fn write_generated_file(decls: &StoreDecls) -> Result<(), String> {
    let dir = store_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir store: {e}"))?;
    let tmp = generated_file().with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, render_nix(decls)).map_err(|e| format!("write tmp apps.nix: {e}"))?;
    fs::rename(&tmp, generated_file()).or_else(|_| {
        let content = render_nix(decls);
        fs::write(generated_file(), content).map_err(|e| format!("write apps.nix: {e}"))
    })
}

pub fn add_nix_package(attr: &str) -> Result<StoreDecls, String> {
    let mut decls = load_decls();
    let a = nix_escape_attr(attr);
    if !decls.nix_packages.iter().any(|x| nix_escape_attr(x) == a) {
        decls.nix_packages.push(a);
    }
    save_decls(&decls)?;
    Ok(decls)
}

pub fn remove_nix_package(attr: &str) -> Result<StoreDecls, String> {
    let mut decls = load_decls();
    let a = nix_escape_attr(attr);
    decls.nix_packages.retain(|x| nix_escape_attr(x) != a);
    save_decls(&decls)?;
    Ok(decls)
}

pub fn add_flatpak(app_id: &str) -> Result<StoreDecls, String> {
    let mut decls = load_decls();
    let id = app_id.trim().to_string();
    if !decls.flatpaks.contains(&id) {
        decls.flatpaks.push(id);
    }
    save_decls(&decls)?;
    Ok(decls)
}

pub fn remove_flatpak(app_id: &str) -> Result<StoreDecls, String> {
    let mut decls = load_decls();
    decls.flatpaks.retain(|x| x != app_id.trim());
    save_decls(&decls)?;
    Ok(decls)
}

/// Read one declared program option (`programs.firefox.enable`).
pub fn get_program_option(path: &str) -> Option<serde_json::Value> {
    load_decls().program_options.get(path.trim()).cloned()
}

/// Write (`Some`) or prune (`None`) one program option. Values equal to the
/// module default should be pruned by the caller to keep apps.nix clean.
pub fn set_program_option(path: &str, value: Option<serde_json::Value>) -> Result<StoreDecls, String> {
    let mut decls = load_decls();
    let path = path.trim().to_string();
    match value {
        Some(v) => {
            decls.program_options.insert(path, v);
        }
        None => {
            decls.program_options.remove(&path);
        }
    }
    save_decls(&decls)?;
    Ok(decls)
}

pub fn resolve_home_manager_file() -> Option<String> {
    Command::new("sh")
        .args(["-c", "EDITOR=echo home-manager edit 2>/dev/null"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let first = s.lines().next().unwrap_or("").trim().to_string();
            if first.is_empty() || !first.ends_with(".nix") {
                None
            } else {
                Some(first)
            }
        })
}

pub fn is_imported(hm_file: &str) -> bool {
    let target = generated_file().to_string_lossy().to_string();
    if let Ok(content) = fs::read_to_string(hm_file) {
        return content.contains(&target) || content.contains("finick/store/apps.nix");
    }
    false
}

/// Bare Nix path literals (`/home/user@host/...`) are a syntax error —
/// `@` is not allowed in path literals. Quote any bare absolute path
/// containing `@` so enterprise / symlinked homes keep working.
pub fn quote_at_paths(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut out = String::with_capacity(content.len() + 16);
    let mut i = 0;
    let mut in_double = false;
    let mut in_indented = false;
    let mut in_line_comment = false;
    while i < chars.len() {
        let c = chars[i];
        if in_line_comment {
            out.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_indented {
            if c == '\'' && i + 1 < chars.len() && chars[i + 1] == '\'' {
                out.push('\'');
                out.push('\'');
                i += 2;
                in_indented = false;
            } else {
                out.push(c);
                i += 1;
            }
            continue;
        }
        if in_double {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
            } else if c == '"' {
                in_double = false;
                i += 1;
            } else {
                i += 1;
            }
            continue;
        }
        if c == '#' {
            in_line_comment = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_double = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '\'' && i + 1 < chars.len() && chars[i + 1] == '\'' {
            in_indented = true;
            out.push('\'');
            out.push('\'');
            i += 2;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] != '/' && chars[i + 1] != '*' {
            let mut j = i + 1;
            while j < chars.len() && !matches!(chars[j], '"' | '\'' | '#' | ';' | ',' | ']' | ')' | '}' | '{' | '[' | '(')
                && !chars[j].is_whitespace()
            {
                j += 1;
            }
            let token: String = chars[i..j].iter().collect();
            if token.contains('@') {
                out.push('"');
                out.push_str(&token);
                out.push('"');
            } else {
                out.push_str(&token);
            }
            i = j;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Rewrite bare `/...@...` imports to quoted strings. Returns true when the
/// file was changed.
pub fn repair_bare_imports(hm_file: &str) -> bool {
    let Ok(content) = fs::read_to_string(hm_file) else { return false };
    let fixed = quote_at_paths(&content);
    if fixed == content {
        return false;
    }
    fs::write(hm_file, fixed).is_ok()
}

pub fn ensure_imported(hm_file: &str) -> Result<(), String> {
    let _ = repair_bare_imports(hm_file);
    if is_imported(hm_file) {
        return Ok(());
    }
    let target = generated_file().to_string_lossy().to_string();
    let _ = save_decls(&load_decls());
    let backup = format!("{hm_file}.bak-finick");
    let _ = fs::copy(hm_file, &backup);
    let status = Command::new("nix")
        .args(["run", "github:vlinkz/nix-editor", "--", hm_file, "imports", "--arr", &target, "--inplace", "--format"])
        .output()
        .map_err(|e| format!("failed to run nix-editor: {e}"))?;
    let _ = repair_bare_imports(hm_file);
    if !status.status.success() {
        let err = String::from_utf8_lossy(&status.stderr).trim().to_string();
        if append_import_fallback(hm_file, &target).is_ok() && is_imported(hm_file) {
            return Ok(());
        }
        return Err(format!(
            "nix-editor failed: {err}. Add manually: imports = [ \"{target}\" ];",
            target = target
        ));
    }
    if !is_imported(hm_file)
        && append_import_fallback(hm_file, &target).is_ok()
        && is_imported(hm_file)
    {
        return Ok(());
    }
    let parse = Command::new("nix-instantiate").args(["--parse", hm_file]).output();
    match parse {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(format!("import added but {hm_file} failed to parse — backup at {backup}")),
    }
}

fn append_import_fallback(hm_file: &str, target: &str) -> Result<(), String> {
    let _ = repair_bare_imports(hm_file);
    let content = fs::read_to_string(hm_file).map_err(|e| format!("read {hm_file}: {e}"))?;
    if content.contains(target) || content.contains("finick/store/apps.nix") {
        return Ok(());
    }
    let line = format!("imports = [ \"{target}\" ];");
    let mut out = content;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if let Some(idx) = out.rfind('}') {
        out.insert_str(idx, &format!("  {line}\n"));
    } else {
        out.push_str(&format!("{{ {line} }}\n"));
    }
    fs::write(hm_file, out).map_err(|e| format!("write {hm_file}: {e}"))?;
    let parse = Command::new("nix-instantiate").args(["--parse", hm_file]).output();
    match parse {
        Ok(o) if o.status.success() => Ok(()),
        _ => {
            let backup = format!("{hm_file}.bak-finick");
            let _ = fs::copy(&backup, hm_file);
            Err(format!("fallback edit failed to parse — backup at {backup}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_bare_at_path_in_imports() {
        let input = "{\n  imports = [\n    ./phone-stuff.nix\n    /home/flora.hill@rmhedge.com/.config/finick/store/apps.nix\n  ];\n}\n";
        let fixed = quote_at_paths(input);
        assert!(fixed.contains("\"/home/flora.hill@rmhedge.com/.config/finick/store/apps.nix\""));
        assert!(fixed.contains("./phone-stuff.nix"));
    }

    #[test]
    fn leaves_quoted_and_plain_paths_alone() {
        let input = "{\n  imports = [ \"/home/a@b/apps.nix\" ./local.nix /nix/store/plain ];\n}\n";
        assert_eq!(quote_at_paths(input), input);
    }
}

pub fn apply_home_manager() -> Result<String, String> {
    // Absolute store imports (`"/home/user@host/.config/..."`) need impure
    // evaluation; try `--impure` variants before pure fallbacks.
    let cmds: Vec<Vec<&str>> = vec![
        vec!["nh", "home", "switch", "--", "--impure"],
        vec!["home-manager", "switch", "--impure"],
        vec!["nh", "home", "switch"],
        vec!["home-manager", "switch"],
    ];
    for cmd in cmds {
        if let Ok(out) = Command::new(cmd[0]).args(&cmd[1..]).output() {
            if out.status.success() {
                return Ok(String::from_utf8_lossy(&out.stdout).to_string());
            }
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !err.contains("not found") && !err.is_empty() {
                return Err(err);
            }
        }
    }
    Err("no home-manager switch backend found (nh / home-manager)".to_string())
}
