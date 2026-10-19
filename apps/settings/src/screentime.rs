use std::collections::HashMap;

/// Real Screen Time tracking backed by Hyprland's active window.
///
/// Usage accumulates into `~/.config/finick/screentime_usage.json` keyed by
/// day (`YYYY-MM-DD`) so totals survive restarts. The daemon only owns the
/// `ScreenTimeEnabled` flag; per-app seconds live here because finickd has
/// no usage-tracking backend yet. Limits persist in
/// `screentime_limits.json` next to it.
#[derive(Clone, Debug, Default)]
pub struct DayUsage {
    pub per_app: HashMap<String, u64>,
}

impl DayUsage {
    pub fn total_secs(&self) -> u64 {
        self.per_app.values().sum()
    }

    pub fn top_apps(&self, n: usize) -> Vec<(String, u64)> {
        let mut v: Vec<(String, u64)> = self.per_app.iter().map(|(k, v)| (k.clone(), *v)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }
}

fn data_file(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|c| c.join("finick").join(name))
}

fn today_key() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn load_day_usage() -> DayUsage {
    let mut out = DayUsage::default();
    let Some(f) = data_file("screentime_usage.json") else { return out };
    let Ok(raw) = std::fs::read_to_string(f) else { return out };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { return out };
    let today = today_key();
    if let Some(day) = v.get(&today).and_then(|d| d.as_object()) {
        for (app, secs) in day {
            if let Some(s) = secs.as_u64() {
                out.per_app.insert(app.clone(), s);
            }
        }
    }
    out
}

pub fn record_tick(app: &str, secs: u64) {
    if app.trim().is_empty() || secs == 0 {
        return;
    }
    let Some(f) = data_file("screentime_usage.json") else { return };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut root: serde_json::Value =
        std::fs::read_to_string(&f).ok().and_then(|r| serde_json::from_str(&r).ok()).unwrap_or(serde_json::json!({}));
    let today = today_key();
    let cur = root.get(&today).and_then(|d| d.get(app)).and_then(|v| v.as_u64()).unwrap_or(0);
    if let Some(day) = root.as_object_mut() {
        let entry = day.entry(today).or_insert_with(|| serde_json::json!({}));
        if let Some(map) = entry.as_object_mut() {
            map.insert(app.to_string(), serde_json::Value::from(cur + secs));
        }
    }
    let _ = std::fs::write(f, serde_json::to_string_pretty(&root).unwrap_or_default());
}

pub fn load_limits() -> HashMap<String, u64> {
    let Some(f) = data_file("screentime_limits.json") else { return HashMap::new() };
    let Ok(raw) = std::fs::read_to_string(f) else { return HashMap::new() };
    serde_json::from_str::<HashMap<String, u64>>(&raw).unwrap_or_default()
}

pub fn save_limit(app: &str, secs: u64) {
    let Some(f) = data_file("screentime_limits.json") else { return };
    if let Some(parent) = f.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut map = load_limits();
    if secs == 0 {
        map.remove(app);
    } else {
        map.insert(app.to_string(), secs);
    }
    let _ = std::fs::write(f, serde_json::to_string_pretty(&map).unwrap_or_default());
}

pub fn parse_active_app(output: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(output).ok()?;
    if v.get("address").and_then(|a| a.as_str()).map(|s| s.is_empty()).unwrap_or(true)
        && v.get("class").is_none()
    {
        return None;
    }
    let class = v.get("class").and_then(|c| c.as_str()).unwrap_or("").trim();
    let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("").trim();
    if class.is_empty() && title.is_empty() {
        return None;
    }
    Some(if class.is_empty() { title.to_string() } else { class.to_string() })
}

pub fn active_app_blocking() -> Option<String> {
    let out = std::process::Command::new("hyprctl").args(["activewindow", "-j"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_active_app(&String::from_utf8_lossy(&out.stdout))
}

pub fn format_duration(total_secs: u64) -> String {
    if total_secs < 60 {
        return format!("{total_secs}s");
    }
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

pub fn format_clock(total_secs: u64) -> String {
    format!("{:02}:{:02}", total_secs / 3600, (total_secs % 3600) / 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_active_app_class_preferred() {
        let raw = r#"{"address":"0x1","class":"firefox","title":"Inbox"}"#;
        assert_eq!(parse_active_app(raw), Some("firefox".to_string()));
    }

    #[test]
    fn test_parse_active_app_empty_is_none() {
        assert_eq!(parse_active_app(r#"{"address":"","title":""}"#), None);
        assert_eq!(parse_active_app("not json"), None);
    }

    #[test]
    fn test_day_usage_totals_and_top() {
        let mut d = DayUsage::default();
        d.per_app.insert("a".to_string(), 60);
        d.per_app.insert("b".to_string(), 120);
        assert_eq!(d.total_secs(), 180);
        assert_eq!(d.top_apps(1)[0].0, "b");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(90), "1m");
        assert_eq!(format_duration(3700), "1h 1m");
    }
}
