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
    scan,
    index,
    accent,
    border,
    settings,
    notify,
}

fn accent_to_hex(s: &str) -> String {
    let clean = s.trim().trim_start_matches('#');
    match clean.to_lowercase().as_str() {
        "indigo" => "5B5FE9".to_string(),
        "coral" => "FF6952".to_string(),
        "amber" => "E3A23D".to_string(),
        "teal" => "2CA6A0".to_string(),
        "rose" => "E85A88".to_string(),
        "slate" => "7B7F87".to_string(),
        other => {
            if (other.len() == 6 || other.len() == 8) && other.chars().all(|c| c.is_ascii_hexdigit()) {
                other.to_uppercase()
            } else {
                "5B5FE9".to_string()
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    match args.program {
        Program::scan => {
            match ipsea::send_command(App::Scan, &(), Some(|_: ()| {})) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to connect to service. ({e:?})");
                }
            };
        }
        Program::index => {
            let q = args.data.unwrap_or_else(|| {
                eprintln!("No data provided");
                std::process::exit(1);
            });

            println!("Searching for: {}", q);
            ipsea::send_command(
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
            let _ =
                std::process::Command::new("hyprctl").args(["keyword", "general:col.active_border", &hypr_color]).status();
            let _ = ipsea::settings::set_and_apply("settings-daemon", ipsea::settings::SettingKey::AccentColor, &*color);
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
                let _ = std::process::Command::new("hyprctl")
                    .args(["keyword", "general:border_size", &size.to_string()])
                    .status();
                let _ = ipsea::settings::set_and_apply(
                    "settings-daemon",
                    ipsea::settings::SettingKey::WindowBorderSize,
                    size as i64,
                );
                println!("Applied border size: {size}");
            } else {
                let hex = accent_to_hex(&val);
                let hypr_color =
                    if val.starts_with("0x") || val.starts_with("rgb") { val.clone() } else { format!("0xff{hex}") };
                let _ = std::process::Command::new("hyprctl")
                    .args(["keyword", "general:col.active_border", &hypr_color])
                    .status();
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
                let _ = ipsea::settings::set_and_apply("settings-daemon", key.clone(), v.trim());
                if key == ipsea::settings::SettingKey::AccentColor {
                    let hex = accent_to_hex(v.trim());
                    let _ = std::process::Command::new("hyprctl")
                        .args(["keyword", "general:col.active_border", &format!("0xff{hex}")])
                        .status();
                }
                println!("Set {k} = {v}");
            } else {
                let key: ipsea::settings::SettingKey = key_or_kv.trim().parse().unwrap();
                match ipsea::settings::get_setting("settings-daemon", key) {
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
    }
}
