//! Per-package Home Manager option forms.
//!
//! Schemas come from evaluating HM module declarations (`nix eval` on the
//! matching nixpkgs/HM releases, JSON-cached under `~/.cache`). Only scalar
//! kinds are editable (bool/string/int/float/small enums); everything else
//! renders read-only. Values persist in `StoreDecls::program_options` as
//! dotted paths (`programs.firefox.enable`) and render into `apps.nix`.

use {
    serde::Deserialize,
    serde_json::Value,
    std::{collections::HashMap, process::Command},
};

#[derive(Clone, Debug, PartialEq)]
pub enum OptKind {
    Bool,
    String,
    Int,
    Float,
    Enum(Vec<String>),
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptField {
    /// Dotted path, e.g. `programs.firefox.enable`.
    pub path: String,
    /// Leaf name for display.
    pub name: String,
    pub description: String,
    pub kind: OptKind,
    pub default: Option<Value>,
}

const SCHEMA_RELEASE: &str = "nixos-25.05_hm-25.05";
const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60;

fn cache_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(format!("{h}/.cache/finick/hm-options")))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/.finick/hm-options"))
}

fn eval_nix(expr: &str, timeout_secs: u64) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let expr = expr.to_string();
    std::thread::spawn(move || {
        let out = Command::new("nix").args(["eval", "--impure", "--json", "--expr", &expr]).output();
        let _ = tx.send(out);
    });
    let out = rx.recv_timeout(std::time::Duration::from_secs(timeout_secs)).ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

const PRELUDE: &str = r#"
let
  nixpkgs = builtins.getFlake "github:NixOS/nixpkgs/nixos-25.05";
  pkgs = nixpkgs.legacyPackages.x86_64-linux;
  hm = builtins.getFlake "github:nix-community/home-manager/release-25.05";
  cfg = hm.lib.homeManagerConfiguration {
    inherit pkgs;
    modules = [{
      home.username = "finick";
      home.homeDirectory = "/home/finick";
      home.stateVersion = "25.05";
    }];
  };
in
"#;

fn read_cache(name: &str) -> Option<String> {
    let path = cache_dir().join(name);
    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(mtime) = meta.modified() {
            if mtime.elapsed().map(|e| e.as_secs() > CACHE_TTL_SECS).unwrap_or(false) {
                return None;
            }
        }
    }
    std::fs::read_to_string(path).ok()
}

fn write_cache(name: &str, content: &str) {
    let dir = cache_dir();
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(dir.join(name), content);
    }
}

/// All `programs.*` module names (one slow eval, release-keyed TTL cache).
pub fn program_modules() -> Vec<String> {
    let cache_name = format!("programs-{SCHEMA_RELEASE}.json");
    if let Some(cached) = read_cache(&cache_name)
        && let Ok(list) = serde_json::from_str::<Vec<String>>(&cached)
    {
        return list;
    }
    let expr = format!("{PRELUDE} builtins.attrNames cfg.options.programs");
    let list = eval_nix(&expr, 180)
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default();
    if !list.is_empty() {
        write_cache(&cache_name, &serde_json::to_string(&list).unwrap_or_default());
    }
    list
}

pub fn normalize_module_candidate(attr: &str) -> String {
    let pname = attr.trim().trim_start_matches("nixpkgs#").split('.').next_back().unwrap_or("").to_string();
    let mut lower = pname.to_lowercase();
    loop {
        let mut stripped = false;
        for suffix in ["-esr", "-bin", "-git"] {
            if let Some(s) = lower.strip_suffix(suffix) {
                lower = s.to_string();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }
    lower
}

pub fn module_for_attr(attr: &str) -> Option<String> {
    let pname = attr.trim().trim_start_matches("nixpkgs#").split('.').next_back().unwrap_or("").to_string();
    if pname.is_empty() {
        return None;
    }
    let modules = program_modules();
    if let Some(found) = modules.iter().find(|m| m.as_str() == pname.as_str()) {
        return Some(found.clone());
    }
    let norm = normalize_module_candidate(&pname);
    if norm.is_empty() {
        return None;
    }
    if let Some(found) = modules.iter().find(|m| m.to_lowercase() == norm) {
        return Some(found.clone());
    }
    let mut best: Option<String> = None;
    let mut best_len = 0usize;
    for m in modules {
        let ml = m.to_lowercase();
        if norm.contains(&ml) || ml.contains(&norm) {
            if m.len() > best_len {
                best_len = m.len();
                best = Some(m);
            }
        }
    }
    best
}

#[derive(Deserialize)]
struct RawOpt {
    #[serde(default)]
    description: String,
    #[serde(default)]
    #[serde(rename = "type")]
    type_desc: String,
    #[serde(default)]
    default: RawDefault,
}

#[derive(Deserialize, Default)]
struct RawDefault {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    value: Value,
}

/// Full schema for one module (cached per module after first slow eval).
pub fn module_schema(module: &str) -> Vec<OptField> {
    let cache_name = format!("{module}-{SCHEMA_RELEASE}.json");
    if let Some(cached) = read_cache(&cache_name)
        && let Ok(fields) = fields_from_cache(&cached)
    {
        return fields;
    }
    // Declarations only: defaults that don't serialize (functions, derivations)
    // collapse to null instead of failing the whole eval.
    let expr = format!(
        r#"{PRELUDE} builtins.mapAttrs (n: o: {{
          description = o.description or "";
          type = o.type.description or "";
          default = let r = builtins.tryEval (builtins.toJSON (o.default or null));
            in if r.success then builtins.fromJSON r.value else null;
        }}) cfg.options.programs.{module}"#
    );
    let fields = eval_nix(&expr, 180).map(|s| schema_from_json(&s, module).unwrap_or_default()).unwrap_or_default();
    if !fields.is_empty() {
        write_fields_cache(&cache_name, &fields);
    }
    fields
}

fn write_fields_cache(name: &str, fields: &[OptField]) {
    let minimal: Vec<serde_json::Value> = fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "path": f.path,
                "name": f.name,
                "description": f.description,
                "kind": kind_to_string(&f.kind),
                "default": f.default,
            })
        })
        .collect();
    write_cache(name, &serde_json::to_string(&minimal).unwrap_or_default());
}

fn kind_to_string(kind: &OptKind) -> String {
    match kind {
        OptKind::Bool => "bool".to_string(),
        OptKind::String => "string".to_string(),
        OptKind::Int => "int".to_string(),
        OptKind::Float => "float".to_string(),
        OptKind::Enum(v) => format!("enum:{}", v.join(",")),
        OptKind::Unsupported(t) => format!("unsupported:{t}"),
    }
}

fn kind_from_string(s: &str) -> OptKind {
    if s == "bool" {
        return OptKind::Bool;
    }
    if s == "string" {
        return OptKind::String;
    }
    if s == "int" {
        return OptKind::Int;
    }
    if s == "float" {
        return OptKind::Float;
    }
    if let Some(rest) = s.strip_prefix("enum:") {
        return OptKind::Enum(if rest.is_empty() { Vec::new() } else { rest.split(',').map(|x| x.to_string()).collect() });
    }
    OptKind::Unsupported(s.strip_prefix("unsupported:").unwrap_or(s).to_string())
}

fn schema_from_json(s: &str, module: &str) -> Result<Vec<OptField>, String> {
    let map: HashMap<String, RawOpt> = serde_json::from_str(s).map_err(|e| e.to_string())?;
    let mut fields: Vec<OptField> = map
        .into_iter()
        .map(|(name, raw)| {
            let kind = kind_from_type(&raw.type_desc);
            OptField {
                path: format!("programs.{module}.{name}"),
                name,
                description: raw.description,
                kind,
                default: if raw.default.success { Some(raw.default.value) } else { None },
            }
        })
        .collect();
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fields)
}

/// Cached form stores real paths + kind strings (see write_fields_cache).
fn fields_from_cache(s: &str) -> Result<Vec<OptField>, String> {
    let items: Vec<serde_json::Value> = serde_json::from_str(s).map_err(|e| e.to_string())?;
    let mut fields = Vec::new();
    for item in items {
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if path.is_empty() || name.is_empty() {
            continue;
        }
        fields.push(OptField {
            path,
            name,
            description: item.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            kind: kind_from_string(item.get("kind").and_then(|v| v.as_str()).unwrap_or("")),
            default: item.get("default").cloned().filter(|v| !v.is_null()),
        });
    }
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fields)
}

fn kind_from_type(desc: &str) -> OptKind {
    // `null or X` wrappers: edit the inner scalar, empty clears.
    let inner = desc.strip_prefix("null or ").unwrap_or(desc);
    if inner == "boolean" {
        return OptKind::Bool;
    }
    if inner == "string" || inner == "path" || inner == "absolute path" {
        return OptKind::String;
    }
    if inner.contains("integer") && !inner.contains(" or ") {
        return OptKind::Int;
    }
    if (inner.contains("float") || inner == "number") && !inner.contains(" or ") {
        return OptKind::Float;
    }
    if let Some(rest) = inner.strip_prefix("one of ") {
        let values = parse_enum_values(rest);
        if !values.is_empty() {
            return OptKind::Enum(values);
        }
    }
    OptKind::Unsupported(desc.to_string())
}

/// Parse `one of "a", "b", "c"` value lists from enum type descriptions.
fn parse_enum_values(s: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = s.trim();
    // Strip trailing prose after the value list (e.g. `, where ...`).
    loop {
        rest = rest.trim().trim_start_matches(',').trim();
        if !rest.starts_with('"') {
            break;
        }
        let mut escaped = false;
        let mut end = None;
        for (i, c) in rest[1..].char_indices() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                end = Some(i + 1);
                break;
            }
        }
        match end {
            Some(e) => {
                values.push(rest[1..e].to_string());
                rest = &rest[e + 1..];
            }
            None => break,
        }
        if values.len() > 32 {
            break;
        }
    }
    values
}

/// Nix literal for scalar JSON values (bools, numbers, strings).
pub fn nix_literal(value: &Value) -> Option<String> {
    match value {
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_from_type() {
        assert_eq!(kind_from_type("boolean"), OptKind::Bool);
        assert_eq!(kind_from_type("null or boolean"), OptKind::Bool);
        assert_eq!(kind_from_type("string"), OptKind::String);
        assert_eq!(kind_from_type("signed integer"), OptKind::Int);
        assert_eq!(
            kind_from_type("one of \"a\", \"b\""),
            OptKind::Enum(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(kind_from_type("list of strings"), OptKind::Unsupported("list of strings".to_string()));
    }

    #[test]
    fn test_schema_from_json_fixture() {
        let fixture = r#"{
            "enable": {"description": "Whether to enable.", "type": "boolean", "default": {"success": true, "value": false}},
            "package": {"description": "Pkg.", "type": "package", "default": {"success": false, "value": null}},
            "theme": {"description": "Theme.", "type": "one of \"a\", \"b\"", "default": {"success": true, "value": "a"}}
        }"#;
        // Paths get the real module from the caller.
        let fields = schema_from_json(fixture, "firefox").unwrap();
        assert_eq!(fields.len(), 3);
        assert!(fields.iter().any(|f| f.name == "enable" && f.kind == OptKind::Bool));
        let theme = fields.iter().find(|f| f.name == "theme").unwrap();
        assert_eq!(theme.kind, OptKind::Enum(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(theme.default, Some(Value::String("a".to_string())));
    }

    #[test]
    fn test_nix_literal() {
        assert_eq!(nix_literal(&Value::Bool(true)), Some("true".to_string()));
        assert_eq!(nix_literal(&serde_json::json!(3)), Some("3".to_string()));
        assert_eq!(nix_literal(&Value::String("a\"b".to_string())), Some("\"a\\\"b\"".to_string()));
        assert_eq!(nix_literal(&serde_json::json!({"a": 1})), None);
    }

    #[test]
    fn test_normalize_module_candidate() {
        assert_eq!(normalize_module_candidate("firefox-esr"), "firefox");
        assert_eq!(normalize_module_candidate("nixpkgs#firefox-esr"), "firefox");
        assert_eq!(normalize_module_candidate("Firefox-BIN"), "firefox");
        assert_eq!(normalize_module_candidate("ripgrep-git"), "ripgrep");
        assert_eq!(normalize_module_candidate("firefox"), "firefox");
    }
}
