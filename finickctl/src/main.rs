use {clap::Parser, config::ty::App};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, help = "Program to command")]
    program: Program,
    #[arg(short, help = "Output in JSON format", default_value = "false")]
    json: bool,
    #[arg(name = "DATA", help = "Data to parse")]
    data: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(strum::Display, strum::EnumString, Clone, Debug)]
enum Program {
    index,
    accent,
    border,
    settings,
    notify,
    modal,
    launcher,
    apps,
}

fn accent_to_hex(s: &str) -> String { config::ty::accent_to_hex(s) }

// Must stay in sync with overlay::launcher::launcher_lock_path(), which is
// canonical for the Super+Space path. finickctl mirrors it here so `--toggle`
// kills the same process the overlay binary would.
fn launcher_lock_path() -> std::path::PathBuf { std::path::PathBuf::from("/tmp/finick-launcher.lock") }

fn launcher_pid_alive(pid: u32) -> bool { std::path::PathBuf::from(format!("/proc/{pid}")).exists() }

/// Kill the running launcher for `--toggle`. Returns true when a live
/// launcher was toggled off (caller should exit); false means stale or
/// missing lock (caller should start a fresh launcher).
fn kill_old_launcher(lock: &std::path::Path) -> bool {
    let Ok(pid_str) = std::fs::read_to_string(lock) else { return false };
    match pid_str.trim().parse::<u32>() {
        Ok(pid) if pid != std::process::id() && launcher_pid_alive(pid) => {
            let _ = std::process::Command::new("kill").arg(pid.to_string()).output();
            for _ in 0..30 {
                if !launcher_pid_alive(pid) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let _ = std::fs::remove_file(lock);
            true
        }
        _ => {
            let _ = std::fs::remove_file(lock);
            false
        }
    }
}

/// Wait-and-focus after spawning the launcher. Mirrors the overlay path
/// (`overlay::launcher::focus_launcher_best_effort` plus the mount-time
/// re-focus in `launcher_app`): dispatch immediately, let the window map,
/// then bring to top and focus again. Best effort; failures are ignored so
/// a missing Hyprland socket never fails the spawn itself.
fn focus_launcher_after_spawn() {
    let focus_once = || {
        system::hypr_bring_to_top("class:^(launcher)$");
        system::hypr_focus_window("class:^(launcher)$");
    };
    focus_once();
    std::thread::sleep(std::time::Duration::from_millis(250));
    focus_once();
}

fn apps_run_job(job: system::store::StoreJob) -> Result<String, String> {
    let mut progress = |ev: system::store::StoreProgress| match ev {
        system::store::StoreProgress::Started(d) => eprintln!("{d}…"),
        system::store::StoreProgress::Log(l) => eprintln!("{l}"),
    };
    system::store::run_job_ipc_or_local(job, &mut progress)
}

fn main() {
    let args = Args::parse();
    match args.program {
        Program::index => {
            let q = args.data.unwrap_or_else(|| {
                eprintln!("No data provided");
                std::process::exit(1);
            });

            println!("Searching for: {}", q);
            ipc::send_command(
                App::IndexService,
                &index::ty::Request::Search { query: q },
                Some(move |value: index::ty::SearchResult| match args.json {
                    true => println!("{}", serde_json::to_string(&value).unwrap()),
                    false => {
                        println!(
                            "{ic}{}\t{}",
                            value.name,
                            value.path,
                            ic = {
                                if value.is_dir {
                                    "/"
                                } else if value.is_desktop {
                                    "@"
                                } else if value.is_executable {
                                    "*"
                                } else {
                                    ""
                                }
                            }
                        );
                    }
                }),
            )
            .expect("Failed to connect to index");

            println!("Done")
        }
        Program::accent => {
            let color = args.data.unwrap_or_else(|| {
                eprintln!(
                    "No accent color provided (e.g. 'Indigo', 'Coral', 'Amber', 'Teal', 'Rose', 'Slate', or hex '#5B5FE9')"
                );
                std::process::exit(1);
            });
            let hex = accent_to_hex(&color);
            let hypr_color = format!("0xff{hex}");
            let lua = format!(
                "hl.config({{ general = {{ col = {{ active_border = {{ colors = {{ \"{hypr_color}\" }} }} }} }} }})"
            );
            let _ = system::HyprlandBackend::hyprctl_set_config(&lua, "general:col.active_border", &hypr_color);
            let _ = ipsea::settings::set_and_apply("finickd", ipsea::settings::SettingKey::AccentColor, &*color);
            if args.json {
                println!("{}", serde_json::json!({ "accent": color, "hex": hex, "hypr_border": hypr_color }));
            } else {
                println!("Applied accent '{color}' (border {hypr_color})");
            }
        }
        Program::border => {
            let val = args.data.unwrap_or_else(|| {
                eprintln!("No border value provided (e.g. hex color '0xff5B5FE9' or size '2')");
                std::process::exit(1);
            });
            if let Ok(size) = val.parse::<u32>() {
                let lua = format!("hl.config({{ general = {{ border_size = {size} }} }})");
                let _ = system::HyprlandBackend::hyprctl_set_config(&lua, "general:border_size", &size.to_string());
                let _ =
                    ipsea::settings::set_and_apply("finickd", ipsea::settings::SettingKey::WindowBorderSize, size as i64);
                println!("Applied border size: {size}");
            } else {
                let hex = accent_to_hex(&val);
                let hypr_color =
                    if val.starts_with("0x") || val.starts_with("rgb") { val.clone() } else { format!("0xff{hex}") };
                let lua = format!(
                    "hl.config({{ general = {{ col = {{ active_border = {{ colors = {{ \"{hypr_color}\" }} }} }} }} }})"
                );
                let _ =
                    system::HyprlandBackend::hyprctl_set_config(&lua, "general:col.active_border", &hypr_color);
                println!("Applied active window border: {hypr_color}");
            }
        }
        Program::settings => {
            let key_or_kv = args.data.unwrap_or_else(|| {
                eprintln!("No setting provided (e.g. 'accent_color=Teal' or 'accent_color')");
                std::process::exit(1);
            });
            if let Some((k, v)) = key_or_kv.split_once('=') {
                let key: ipsea::settings::SettingKey = k.trim().parse().unwrap();
                let _ = ipsea::settings::set_and_apply("finickd", key.clone(), v.trim());
                if key == ipsea::settings::SettingKey::AccentColor {
                    let hex = accent_to_hex(v.trim());
                    let hypr_color = format!("0xff{hex}");
                    let lua = format!(
                        "hl.config({{ general = {{ col = {{ active_border = {{ colors = {{ \"{hypr_color}\" }} }} }} }} }})"
                    );
                    let _ = system::HyprlandBackend::hyprctl_set_config(
                        &lua,
                        "general:col.active_border",
                        &hypr_color,
                    );
                }
                println!("Set {k} = {v}");
            } else {
                let key: ipsea::settings::SettingKey = key_or_kv.trim().parse().unwrap();
                match ipsea::settings::get_setting("finickd", key) {
                    Ok(Some(entry)) => {
                        if args.json {
                            println!("{}", serde_json::to_string(&entry).unwrap());
                        } else {
                            println!("{}: {:?}", entry.key.as_str(), entry.value);
                        }
                    }
                    Ok(None) => println!("Setting not found"),
                    Err(e) => eprintln!("Failed to query settings daemon: {e:?}"),
                }
            }
        }
        Program::notify => {
            let msg = args.data.unwrap_or_else(|| "Finick|Test Notification".to_string());
            let mut parts = msg.splitn(2, '|');
            let summary = parts.next().unwrap_or("Finick");
            let body = parts.next().unwrap_or("");
            let status = std::process::Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.freedesktop.Notifications",
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.Notifications",
                    "Notify",
                    "susssasa{sv}i",
                    "finickctl",
                    "0",
                    "",
                    summary,
                    body,
                    "0",
                    "0",
                    "5000",
                ])
                .status();
            match status {
                Ok(s) if s.success() => println!("Sent notification via DBus"),
                _ => eprintln!("Failed to run busctl. Is the daemon running?"),
            }
        }
        Program::modal => {
            let val = args.data.unwrap_or_else(|| {
                eprintln!("No modal request provided. e.g. 'pam:Prompt string'");
                std::process::exit(1);
            });
            let req = if let Some(prompt) = val.strip_prefix("pam:") {
                ipsea::modals::ModalRequest::PamAuth { prompt: prompt.to_string() }
            } else if let Some(wifi) = val.strip_prefix("wifi:") {
                let parts: Vec<&str> = wifi.splitn(2, ',').collect();
                ipsea::modals::ModalRequest::WifiPassword {
                    ssid: parts[0].to_string(),
                    security: parts.get(1).unwrap_or(&"WPA2").to_string(),
                }
            } else if let Some(bt) = val.strip_prefix("bt:") {
                let parts: Vec<&str> = bt.splitn(2, ',').collect();
                ipsea::modals::ModalRequest::BluetoothPair {
                    mac: parts[0].to_string(),
                    name: parts.get(1).unwrap_or(&"Unknown").to_string(),
                }
            } else {
                eprintln!("Unknown modal type: {}", val);
                std::process::exit(1);
            };

            match ipsea::modals::send_modal_request(req) {
                Ok(resp) => {
                    if args.json {
                        println!("{}", serde_json::to_string(&resp).unwrap());
                    } else {
                        println!("{:?}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to send modal request: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Program::launcher => {
            let toggle = args.data.as_deref().is_some_and(|d| d == "--toggle" || d == "toggle" || d == "-t");
            let lock = launcher_lock_path();
            if toggle && kill_old_launcher(&lock) {
                println!("Launcher toggled off");
                return;
            }
            if toggle {
                let _ = std::fs::remove_file(&lock);
            }
            if args.data.as_deref().is_some_and(|d| !d.starts_with('-') && d != "toggle") {
                eprintln!("Unknown launcher arg: {}", args.data.as_deref().unwrap_or_default());
                std::process::exit(1);
            }
            let try_direct = std::process::Command::new("launcher").spawn();
            if try_direct.is_ok() {
                focus_launcher_after_spawn();
                println!("Launcher started");
            } else if system::hypr_exec("launcher") {
                focus_launcher_after_spawn();
                println!("Launcher dispatched via hyprctl");
            } else {
                eprintln!("Failed to start launcher. Ensure `launcher` is in PATH.");
                eprintln!("Hyprland shortcut hint: bind = SUPER, SPACE, exec, launcher  # or finickctl launcher");
                std::process::exit(1);
            }
        }
        Program::apps => {
            let cmd = args.data.unwrap_or_else(|| {
                eprintln!(
                    "Usage: finickctl -p apps '<search QUERY | install-nix ATTR | remove-nix ATTR | install-flatpak ID | \
                     remove-flatpak ID | link | apply | list>'"
                );
                std::process::exit(1);
            });
            let (op, rest) = cmd.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((cmd.as_str(), ""));
            match op {
                "search" => {
                    for e in system::store::search_all(rest) {
                        if args.json {
                            println!("{}", serde_json::to_string(&e).unwrap());
                        } else {
                            let src = match e.source {
                                system::store::StoreSource::Flathub => "flathub",
                                system::store::StoreSource::Nixpkgs => "nix",
                                system::store::StoreSource::System => "sys",
                            };
                            println!("[{src}] {} — {} ({})", e.name, e.summary, e.id);
                        }
                    }
                }
                "list" => {
                    for e in system::store::list_installed() {
                        if args.json {
                            println!("{}", serde_json::to_string(&e).unwrap());
                        } else {
                            println!("{} ({})", e.name, e.id);
                        }
                    }
                }
                "install-nix" => match apps_run_job(system::store::StoreJob::InstallNix(rest.to_string())) {
                    Ok(m) => println!("{m}"),
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                },
                "remove-nix" => match apps_run_job(system::store::StoreJob::RemoveNix(rest.to_string())) {
                    Ok(m) => println!("{m}"),
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                },
                "install-flatpak" => match apps_run_job(system::store::StoreJob::InstallFlatpak(rest.to_string())) {
                    Ok(m) => println!("{m}"),
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                },
                "remove-flatpak" => match apps_run_job(system::store::StoreJob::RemoveFlatpak(rest.to_string())) {
                    Ok(m) => println!("{m}"),
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                },
                "link" => {
                    let Some(f) = system::store_hm::resolve_home_manager_file() else {
                        eprintln!("home-manager file not found");
                        std::process::exit(1);
                    };
                    match system::store_hm::ensure_imported(&f) {
                        Ok(()) => println!("Linked {} in {f}", system::store_hm::generated_file().display()),
                        Err(e) => {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                    }
                }
                "apply" => match apps_run_job(system::store::StoreJob::ApplyHomeManager) {
                    Ok(log) => println!("Applied. {log}"),
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                },
                _ => {
                    eprintln!("Unknown apps op: {op}");
                    std::process::exit(1);
                }
            }
        }
    }
}
