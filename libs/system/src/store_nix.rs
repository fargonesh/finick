use {
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, process::Command},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NixPackage {
    pub attr: String,
    pub pname: String,
    pub version: String,
    pub description: String,
    pub long_description: String,
    pub homepage: String,
    pub license: String,
    pub installed: bool,
    pub impure_fallback: bool,
}

pub fn nix_available() -> bool {
    Command::new("nix").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn nqx_available() -> bool {
    Command::new("nqx").arg("--help").output().map(|o| o.status.success()).unwrap_or(false)
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd).args(args).output().ok().filter(|o| o.status.success()).map(|o| {
        String::from_utf8_lossy(&o.stdout).to_string()
    })
}

pub fn profile_list() -> Vec<String> {
    let Some(out) = run("nix", &["profile", "list", "--json"]) else {
        return Vec::new();
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
        if let Some(elements) = v.get("elements").and_then(|e| e.as_object()) {
            return elements
                .values()
                .filter_map(|el| {
                    el.get("attrPath")
                        .and_then(|a| a.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| el.get("originalUrl").and_then(|u| u.as_str()).map(|s| s.to_string()))
                })
                .collect();
        }
        if let Some(arr) = v.as_array() {
            return arr
                .iter()
                .filter_map(|el| el.get("attrPath").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .collect();
        }
    }
    out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

pub fn search(query: &str) -> Vec<NixPackage> {
    let q = query.trim();
    if q.is_empty() || !nix_available() {
        return Vec::new();
    }
    if nqx_available() {
        if let Some(hits) = nqx_search(q) {
            return hits;
        }
    }
    nix_search(q)
}

fn installed_attrs() -> std::collections::HashSet<String> {
    profile_list().into_iter().collect()
}

fn nqx_search(query: &str) -> Option<Vec<NixPackage>> {
    let out = run("nqx", &["search", query])?;
    let installed = installed_attrs();
    let mut hits = Vec::new();
    for line in out.lines().take(50) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let attr = line.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '*' || c == ' ').to_string();
        if attr.is_empty() {
            continue;
        }
        hits.push(NixPackage {
            attr: attr.clone(),
            pname: attr.split('.').last().unwrap_or(&attr).to_string(),
            installed: installed.contains(&attr),
            ..Default::default()
        });
    }
    if hits.is_empty() { None } else { Some(hits) }
}

fn nix_search(query: &str) -> Vec<NixPackage> {
    let installed = installed_attrs();
    let Some(out) = run("nix", &["search", "nixpkgs", query, "--json"]) else {
        return Vec::new();
    };
    let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&out) else {
        return Vec::new();
    };
    map.into_iter()
        .take(50)
        .map(|(full_attr, v)| {
            let attr = full_attr.strip_prefix("nixpkgs#").unwrap_or(&full_attr).to_string();
            let pname = v
                .get("pname")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| attr.split('.').last().unwrap_or(&attr).to_string());
            NixPackage {
                attr: attr.clone(),
                pname,
                version: v.get("version").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                description: v.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                installed: installed.contains(&attr),
                ..Default::default()
            }
        })
        .collect()
}

pub fn eval_meta(attr: &str) -> Option<NixPackage> {
    let a = attr.trim().trim_start_matches("nixpkgs#");
    let expr = format!("nixpkgs#{a}.meta");
    let out = run("nix", &["eval", &expr, "--json"])?;
    let v: serde_json::Value = serde_json::from_str(&out).ok()?;
    let license = v
        .get("license")
        .and_then(|l| l.as_str().map(|s| s.to_string()).or_else(|| l.get("spdxId").and_then(|s| s.as_str()).map(|s| s.to_string())))
        .unwrap_or_default();
    Some(NixPackage {
        attr: a.to_string(),
        pname: a.split('.').last().unwrap_or(a).to_string(),
        description: v.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        long_description: v.get("longDescription").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        homepage: v
            .get("homepage")
            .and_then(|x| x.as_str().or_else(|| x.get("url").and_then(|u| u.as_str())))
            .unwrap_or_default()
            .to_string(),
        license,
        ..Default::default()
    })
}

pub fn package_details(attr: &str) -> NixPackage {
    let mut base = eval_meta(attr).unwrap_or_else(|| NixPackage {
        attr: attr.trim().trim_start_matches("nixpkgs#").to_string(),
        ..Default::default()
    });
    base.pname = base.attr.split('.').last().unwrap_or(&base.attr).to_string();
    base.installed = installed_attrs().contains(&base.attr);
    if base.version.is_empty() {
        let expr = format!("nixpkgs#{}", base.attr);
        if let Some(out) = run("nix", &["eval", &format!("{expr}.version"), "--raw"]) {
            base.version = out.trim().trim_matches('"').to_string();
        }
    }
    base
}

pub fn profile_install(attr: &str) -> Result<String, String> {
    let a = attr.trim().trim_start_matches("nixpkgs#");
    let out = Command::new("nix")
        .args(["profile", "install", &format!("nixpkgs#{a}")])
        .output()
        .map_err(|e| format!("failed to run nix profile: {e}"))?;
    if out.status.success() {
        Ok(format!("installed {a} via nix profile (impure fallback)"))
    } else {
        Err(format!(
            "{} {}",
            String::from_utf8_lossy(&out.stderr).trim(),
            String::from_utf8_lossy(&out.stdout).trim()
        )
        .trim()
        .to_string())
    }
}

pub fn profile_remove(attr: &str) -> Result<String, String> {
    let a = attr.trim();
    let target = if a.contains("nixpkgs#") || a.contains('.') { a.to_string() } else { format!("nixpkgs#{a}") };
    let out = Command::new("nix").args(["profile", "remove", &target]).output().map_err(|e| format!("failed: {e}"))?;
    if out.status.success() {
        return Ok(format!("removed {a}"));
    }
    let fallback = Command::new("nix").args(["profile", "remove", a]).output().map_err(|e| format!("failed: {e}"))?;
    if fallback.status.success() {
        Ok(format!("removed {a}"))
    } else {
        Err(String::from_utf8_lossy(&fallback.stderr).trim().to_string())
    }
}
