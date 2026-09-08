use freya::prelude::*;
use std::process::Command;
use ui::*;

/// Representation of a connected or configured printer.
#[derive(Clone, Debug, PartialEq)]
pub struct PrinterDevice {
    pub name: String,
    pub system_name: String,
    pub description: String,
    pub location: String,
    pub is_default: bool,
    pub is_accepting: bool,
    pub status: String,
    pub uri: String,
    pub driver: String,
    pub jobs_count: usize,
}

/// Representation of a connected scanning device.
#[derive(Clone, Debug, PartialEq)]
pub struct ScannerDevice {
    pub name: String,
    pub model: String,
    pub connection: String,
    pub status: String,
}

/// Representation of an active print queue job.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintJob {
    pub id: String,
    pub document_name: String,
    pub user: String,
    pub size: String,
    pub status: String,
}

/// Aggregated printers, scanners, and print service configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintersInfo {
    pub printers: Vec<PrinterDevice>,
    pub scanners: Vec<ScannerDevice>,
    pub active_jobs: Vec<PrintJob>,
    pub cups_active: bool,
    pub network_discovery: bool,
    pub default_paper_size: String,
    pub share_printers: bool,
}

impl Default for PrintersInfo {
    fn default() -> Self {
        Self {
            printers: Vec::new(),
            scanners: Vec::new(),
            active_jobs: Vec::new(),
            cups_active: true,
            network_discovery: true,
            default_paper_size: "A4 (210 x 297 mm)".to_string(),
            share_printers: false,
        }
    }
}

/// Fetch real printer and scanner information from CUPS / lpstat / scanimage,
/// falling back cleanly to rich mock data if no physical devices are found.
pub fn fetch_printers_info() -> PrintersInfo {
    let mut default_printer_name = String::new();

    // 1. Get default printer name from `lpstat -d`
    if let Ok(output) = Command::new("lpstat").arg("-d").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(dest) = line.strip_prefix("system default destination: ") {
                default_printer_name = dest.trim().to_string();
            }
        }
    }

    // 2. Query printers list & statuses via `lpstat -p`
    let mut found_printers = Vec::new();
    if let Ok(output) = Command::new("lpstat").arg("-p").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("printer ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let sys_name = parts[1].to_string();
                    let is_def = !default_printer_name.is_empty() && sys_name == default_printer_name;
                    let is_paused = trimmed.contains("disabled");
                    let is_printing = trimmed.contains("now printing");
                    let status = if is_printing {
                        "Printing".to_string()
                    } else if is_paused {
                        "Paused".to_string()
                    } else {
                        "Ready".to_string()
                    };
                    let display_name = sys_name.replace('_', " ");
                    found_printers.push(PrinterDevice {
                        name: display_name,
                        system_name: sys_name.clone(),
                        description: "Local / Network Printer".to_string(),
                        location: "Connected via CUPS".to_string(),
                        is_default: is_def,
                        is_accepting: !is_paused,
                        status,
                        uri: format!("ipp://localhost/printers/{}", sys_name),
                        driver: "CUPS Standard Driver (IPP Everywhere)".to_string(),
                        jobs_count: 0,
                    });
                }
            }
        }
    }

    // 3. Query accepting status via `lpstat -a`
    if let Ok(output) = Command::new("lpstat").arg("-a").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let accepting = line.contains("accepting requests");
                if let Some(p) = found_printers.iter_mut().find(|p| p.system_name == name) {
                    p.is_accepting = accepting;
                }
            }
        }
    }

    // 4. Query device URIs via `lpstat -v`
    if let Ok(output) = Command::new("lpstat").arg("-v").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("device for ")
                && let Some((pname, uri)) = rest.split_once(':') {
                    let sys_name = pname.trim();
                    let uri_val = uri.trim();
                    if let Some(p) = found_printers.iter_mut().find(|p| p.system_name == sys_name) {
                        p.uri = uri_val.to_string();
                        if uri_val.starts_with("usb:") {
                            p.location = "Direct USB Connection".to_string();
                        } else if uri_val.starts_with("ipp:") || uri_val.starts_with("dnssd:") || uri_val.starts_with("socket:") {
                            p.location = "Local Network (IPP / AirPrint)".to_string();
                        }
                    }
                }
        }
    }

    // 5. Query active print jobs via `lpstat -o`
    let mut found_jobs = Vec::new();
    if let Ok(output) = Command::new("lpstat").arg("-o").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let job_id = parts[0].to_string();
                let user = parts[1].to_string();
                let size_bytes = parts[2].parse::<usize>().unwrap_or(0);
                let size_str = if size_bytes > 1_000_000 {
                    format!("{:.1} MB", size_bytes as f64 / 1_000_000.0)
                } else if size_bytes > 1000 {
                    format!("{:.0} KB", size_bytes as f64 / 1000.0)
                } else {
                    format!("{} B", size_bytes)
                };

                let p_sys_name = job_id.rsplit_once('-').map(|(p, _)| p).unwrap_or("");
                if let Some(p) = found_printers.iter_mut().find(|p| p.system_name == p_sys_name) {
                    p.jobs_count += 1;
                }

                found_jobs.push(PrintJob {
                    id: job_id,
                    document_name: "Active Print Spool".to_string(),
                    user,
                    size: size_str,
                    status: "In Queue".to_string(),
                });
            }
        }
    }

    // 6. Query scanners via `scanimage -L`
    let mut found_scanners = Vec::new();
    if let Ok(output) = Command::new("scanimage").arg("-L").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("device ") {
                let line_content = line.trim_start_matches("device ");
                if let Some((dev_id, desc)) = line_content.split_once(" is a ") {
                    let name = desc.trim_matches('`').trim_matches('\'').trim().to_string();
                    found_scanners.push(ScannerDevice {
                        name,
                        model: "SANE Compatible Scanner".to_string(),
                        connection: dev_id.trim_matches('`').trim_matches('\'').trim().to_string(),
                        status: "Ready".to_string(),
                    });
                }
            }
        }
    }

    // 7. Check CUPS service status
    let cups_active = Command::new("systemctl")
        .args(["is-active", "cups"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(true);

    PrintersInfo {
        printers: found_printers,
        scanners: found_scanners,
        active_jobs: found_jobs,
        cups_active,
        network_discovery: true,
        default_paper_size: "A4 (210 x 297 mm)".to_string(),
        share_printers: false,
    }
}

/// Set default printer using `lpoptions -d`
pub fn set_default_printer(printer_name: &str) {
    let _ = Command::new("lpoptions").args(["-d", printer_name]).output();
}

/// Pause printer queue via `cupsdisable`
pub fn pause_printer(printer_name: &str) {
    let _ = Command::new("cupsdisable").arg(printer_name).output();
}

/// Resume printer queue via `cupsenable`
pub fn resume_printer(printer_name: &str) {
    let _ = Command::new("cupsenable").arg(printer_name).output();
}

/// Print test page via `lpr`
pub fn print_test_page(printer_name: &str) {
    let _ = Command::new("lpr").args(["-P", printer_name, "/etc/os-release"]).output();
}

/// Cancel a specific print job via `cancel`
pub fn cancel_print_job(job_id: &str) {
    let _ = Command::new("cancel").arg(job_id).output();
}

#[derive(PartialEq)]
pub struct Printers;

impl Component for Printers {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let printers_state = use_state(PrintersInfo::default);
        let is_loading = use_state(|| true);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut state_clone = printers_state;
            let mut loading_clone = is_loading;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PrintersInfo>();

            std::thread::spawn(move || {
                let info = fetch_printers_info();
                let _ = tx.send(info);
            });

            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    state_clone.set(info);
                    loading_clone.set(false);
                }
            });
        }

        let info = printers_state.read().clone();
        let loading = *is_loading.read();
        let cups_on = info.cups_active;
        let discovery_on = info.network_discovery;

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(page_header(
                        "Printers & Scanners",
                        "Manage connected printers, print queues, and scanning devices.",
                    ))

                    // --- Service Status & Quick Overview Card ---
                    .child(
                        rect()
                            .width(Size::fill())
                            .margin((0., 0., 16., 0.))
                            .padding(16.)
                            .corner_radius(12.)
                            .background(t.bg_card)
                            .border(Border::new().width(1.).fill(t.border_card))
                            .overflow(Overflow::Clip)
                            .child(
                                rect()
                                    .horizontal()
                                    .main_align(Alignment::SpaceBetween)
                                    .cross_align(Alignment::Center)
                                    .width(Size::fill())
                                    .margin((0., 0., 12., 0.))
                                    .content(Content::Flex)
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(
                                                label()
                                                    .font_size(16.)
                                                    .font_weight(FontWeight::BOLD)
                                                    .color(t.text_primary)
                                                    .text("CUPS Printing Daemon")
                                            )
                                            .child(
                                                label()
                                                    .font_size(13.)
                                                    .color(t.text_secondary)
                                                    .margin((4., 0., 0., 0.))
                                                    .text(if cups_on {
                                                        "System print spooler service is active and listening for print jobs."
                                                    } else {
                                                        "CUPS service is stopped. Print jobs cannot be processed."
                                                    })
                                            )
                                    )
                                    .child(
                                        Switch::new()
                                            .toggled(cups_on)
                                            .on_toggle({
                                                let mut ps = printers_state;
                                                let mut l = loaded;
                                                let mut ld = is_loading;
                                                move |_| {
                                                    let next_val = !cups_on;
                                                    let mut curr = ps.read().clone();
                                                    curr.cups_active = next_val;
                                                    ps.set(curr);
                                                    ld.set(true);
                                                    std::thread::spawn(move || {
                                                        let arg = if next_val { "start" } else { "stop" };
                                                        let _ = Command::new("systemctl").args([arg, "cups"]).output();
                                                        std::thread::sleep(std::time::Duration::from_millis(400));
                                                    });
                                                    l.set(false);
                                                }
                                            })
                                    )
                            )
                            // Overview Metrics Strip
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .spacing(16.)
                                    .cross_align(Alignment::Center)
                                    .content(Content::Flex)
                                    .padding((10., 14.))
                                    .margin((8., 0., 14., 0.))
                                    .corner_radius(8.)
                                    .background(t.bg_base)
                                    .border(Border::new().width(1.).fill(t.border_subtle))
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("CONFIGURED PRINTERS"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(format!("{}", info.printers.len())))
                                    )
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("SCANNERS DETECTED"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(format!("{}", info.scanners.len())))
                                    )
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(label().font_size(11.).font_weight(FontWeight::BOLD).color(t.text_muted).text("ACTIVE QUEUE JOBS"))
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::Center)
                                                    .spacing(6.)
                                                    .child(
                                                        rect()
                                                            .width(Size::px(8.))
                                                            .height(Size::px(8.))
                                                            .corner_radius(4.)
                                                            .background(if info.active_jobs.is_empty() { t.accent_green } else { t.accent_orange })
                                                    )
                                                    .child(
                                                        label()
                                                            .font_size(14.)
                                                            .font_weight(FontWeight::SEMI_BOLD)
                                                            .color(if info.active_jobs.is_empty() { t.text_primary } else { t.accent_orange })
                                                            .text(if info.active_jobs.is_empty() { "Queue Idle (0)".to_string() } else { format!("{} Pending", info.active_jobs.len()) })
                                                    )
                                            )
                                    )
                            )
                            // Action buttons
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .child(
                                        secondary_button(if loading { "Refreshing..." } else { "Refresh Devices" }, {
                                            let mut l = loaded;
                                            let mut ld = is_loading;
                                            move || {
                                                ld.set(true);
                                                l.set(false);
                                            }
                                        })
                                    )
                            )
                    )

                    // --- Printers Section ---
                    .child(
                        rect()
                            .width(Size::fill())
                            .margin((0., 0., 16., 0.))
                            .padding(16.)
                            .corner_radius(12.)
                            .background(t.bg_card)
                            .border(Border::new().width(1.).fill(t.border_card))
                            .overflow(Overflow::Clip)
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .margin((0., 0., 14., 0.))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_secondary)
                                            .text(format!("INSTALLED PRINTERS ({})", info.printers.len()))
                                    )
                            )
                            .child({
                                if loading && info.printers.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_secondary).text("Discovering printers..."))
                                        .into_element()
                                } else if info.printers.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("No printers found or configured on this system."))
                                        .into_element()
                                } else {
                                    rect()
                                        .children(info.printers.into_iter().map(|printer| {
                                            let p_name = printer.name.clone();
                                            let p_sys = printer.system_name.clone();
                                            let is_def = printer.is_default;
                                            let status_str = printer.status.clone();
                                            let is_paused = status_str == "Paused" || !printer.is_accepting;
                                            let is_ready = status_str == "Ready" || status_str == "Idle";
                                            let loc = printer.location.clone();
                                            let drv = printer.driver.clone();
                                            let l = loaded;
                                            let ld = is_loading;
                                            let ps = printers_state;

                                            rect()
                                                .key(printer.system_name.clone())
                                                .width(Size::fill())
                                                .margin((0., 0., 8., 0.))
                                                .padding((12., 14.))
                                                .corner_radius(8.)
                                                .background(if is_def { t.bg_active } else { t.bg_base })
                                                .border(Border::new().width(1.).fill(if is_def { t.primary_accent } else { t.border_subtle }))
                                                .child(
                                                    rect()
                                                        .width(Size::fill())
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            // Status indicator dot
                                                            rect()
                                                                .width(Size::px(10.))
                                                                .height(Size::px(10.))
                                                                .corner_radius(5.)
                                                                .background(if is_ready { t.accent_green } else if is_paused { t.accent_orange } else { t.accent_red })
                                                                .margin((0., 12., 0., 0.))
                                                        )
                                                        .child(
                                                            rect()
                                                                .width(Size::fill())
                                                                .child(
                                                                    rect()
                                                                        .horizontal()
                                                                        .cross_align(Alignment::Center)
                                                                        .spacing(8.)
                                                                        .child(
                                                                            label()
                                                                                .font_size(15.)
                                                                                .font_weight(FontWeight::BOLD)
                                                                                .color(t.text_primary)
                                                                                .text(p_name)
                                                                        )
                                                                        .child({
                                                                            if is_def {
                                                                                rect()
                                                                                    .padding((2., 6.))
                                                                                    .corner_radius(4.)
                                                                                    .background(t.primary_accent)
                                                                                    .child(
                                                                                        label()
                                                                                            .font_size(10.)
                                                                                            .font_weight(FontWeight::BOLD)
                                                                                            .color(t.bg_base)
                                                                                            .text("DEFAULT")
                                                                                    )
                                                                                    .into_element()
                                                                            } else {
                                                                                rect().into_element()
                                                                            }
                                                                        })
                                                                )
                                                                .child(
                                                                    label()
                                                                        .font_size(13.)
                                                                        .color(t.text_secondary)
                                                                        .margin((2., 0., 0., 0.))
                                                                        .text(printer.description.clone())
                                                                )
                                                                .child(
                                                                    rect()
                                                                        .horizontal()
                                                                        .spacing(8.)
                                                                        .margin((4., 0., 0., 0.))
                                                                        .child(
                                                                            label()
                                                                                .font_size(12.)
                                                                                .color(if is_ready { t.accent_green } else { t.text_secondary })
                                                                                .text(format!("Status: {}", status_str))
                                                                        )
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(loc))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(drv))
                                                                )
                                                        )
                                                )
                                                // Action Toolbar for this printer
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .margin((10., 0., 0., 22.))
                                                        .spacing(8.)
                                                        .child({
                                                            if !is_def {
                                                                let sys_c = p_sys.clone();
                                                                let mut ps_c = ps;
                                                                let mut l_c = l;
                                                                let mut ld_c = ld;
                                                                secondary_button("Set Default", move || {
                                                                    let s = sys_c.clone();
                                                                    let mut curr = ps_c.read().clone();
                                                                    for p in curr.printers.iter_mut() {
                                                                        p.is_default = p.system_name == s;
                                                                    }
                                                                    ps_c.set(curr);
                                                                    ld_c.set(true);
                                                                    std::thread::spawn(move || {
                                                                        set_default_printer(&s);
                                                                        std::thread::sleep(std::time::Duration::from_millis(400));
                                                                    });
                                                                    l_c.set(false);
                                                                }).into_element()
                                                            } else {
                                                                rect().into_element()
                                                            }
                                                        })
                                                        .child({
                                                            let sys_c = p_sys.clone();
                                                            let mut ps_c = ps;
                                                            let mut l_c = l;
                                                            let mut ld_c = ld;
                                                            let action_label = if is_paused { "Resume Queue" } else { "Pause Queue" };
                                                            secondary_button(action_label, move || {
                                                                let s = sys_c.clone();
                                                                let mut curr = ps_c.read().clone();
                                                                if let Some(p) = curr.printers.iter_mut().find(|p| p.system_name == s) {
                                                                    p.status = if is_paused { "Ready".to_string() } else { "Paused".to_string() };
                                                                    p.is_accepting = is_paused;
                                                                }
                                                                ps_c.set(curr);
                                                                ld_c.set(true);
                                                                std::thread::spawn(move || {
                                                                    if is_paused {
                                                                        resume_printer(&s);
                                                                    } else {
                                                                        pause_printer(&s);
                                                                    }
                                                                    std::thread::sleep(std::time::Duration::from_millis(400));
                                                                });
                                                                l_c.set(false);
                                                             })
                                                        })
                                                        .child({
                                                            let sys_c = p_sys.clone();
                                                            secondary_button("Print Test Page", move || {
                                                                let s = sys_c.clone();
                                                                std::thread::spawn(move || {
                                                                    print_test_page(&s);
                                                                });
                                                            })
                                                        })
                                                )
                                                .into_element()
                                        }))
                                        .into_element()
                                }
                            })
                    )

                    // --- Scanners Section ---
                    .child(
                        rect()
                            .width(Size::fill())
                            .margin((0., 0., 16., 0.))
                            .padding(16.)
                            .corner_radius(12.)
                            .background(t.bg_card)
                            .border(Border::new().width(1.).fill(t.border_card))
                            .overflow(Overflow::Clip)
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .margin((0., 0., 14., 0.))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_secondary)
                                            .text(format!("CONNECTED SCANNERS ({})", info.scanners.len()))
                                    )
                            )
                            .child({
                                if info.scanners.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("No document scanners or digital imaging units detected."))
                                        .into_element()
                                } else {
                                    rect()
                                        .children(info.scanners.into_iter().map(|scanner| {
                                            let s_name = scanner.name.clone();
                                            let s_model = scanner.model.clone();
                                            let s_conn = scanner.connection.clone();
                                            let s_status = scanner.status.clone();

                                            rect()
                                                .key(format!("scanner-{}", scanner.name))
                                                .width(Size::fill())
                                                .horizontal()
                                                .main_align(Alignment::SpaceBetween)
                                                .cross_align(Alignment::Center)
                                                .padding((10., 12.))
                                                .margin((0., 0., 6., 0.))
                                                .corner_radius(8.)
                                                .background(t.bg_base)
                                                .border(Border::new().width(1.).fill(t.border_subtle))
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            rect()
                                                                .width(Size::px(10.))
                                                                .height(Size::px(10.))
                                                                .corner_radius(5.)
                                                                .background(t.accent_green)
                                                                .margin((0., 12., 0., 0.))
                                                        )
                                                        .child(
                                                            rect()
                                                                .child(
                                                                    label()
                                                                        .font_size(14.)
                                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                                        .color(t.text_primary)
                                                                        .text(s_name)
                                                                )
                                                                .child(
                                                                    rect()
                                                                        .horizontal()
                                                                        .spacing(8.)
                                                                        .margin((2., 0., 0., 0.))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(s_model))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(s_conn))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.accent_green).text(s_status))
                                                                )
                                                        )
                                                )
                                                .child(
                                                    secondary_button("Scan Document", move || {
                                                        std::thread::spawn(move || {
                                                            let _ = Command::new("scanimage")
                                                                .args(["--format=png", "--output-file=/tmp/scan_output.png"])
                                                                .output();
                                                        });
                                                    })
                                                )
                                                .into_element()
                                        }))
                                        .into_element()
                                }
                            })
                    )

                    // --- Active Print Queue Section ---
                    .child(
                        rect()
                            .width(Size::fill())
                            .margin((0., 0., 16., 0.))
                            .padding(16.)
                            .corner_radius(12.)
                            .background(t.bg_card)
                            .border(Border::new().width(1.).fill(t.border_card))
                            .overflow(Overflow::Clip)
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .margin((0., 0., 14., 0.))
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_secondary)
                                            .text(format!("ACTIVE PRINT QUEUE ({})", info.active_jobs.len()))
                                    )
                            )
                            .child({
                                if info.active_jobs.is_empty() {
                                    rect()
                                        .width(Size::fill())
                                        .padding(16.)
                                        .center()
                                        .child(label().font_size(14.).color(t.text_muted).text("No active print jobs in spool."))
                                        .into_element()
                                } else {
                                    rect()
                                        .children(info.active_jobs.into_iter().map(|job| {
                                            let j_id = job.id.clone();
                                            let doc = job.document_name.clone();
                                            let u = job.user.clone();
                                            let sz = job.size.clone();
                                            let stat = job.status.clone();
                                            let ps = printers_state;
                                            let l = loaded;

                                            rect()
                                                .key(format!("job-{}", job.id))
                                                .width(Size::fill())
                                                .horizontal()
                                                .main_align(Alignment::SpaceBetween)
                                                .cross_align(Alignment::Center)
                                                .padding((10., 12.))
                                                .margin((0., 0., 6., 0.))
                                                .corner_radius(8.)
                                                .background(t.bg_base)
                                                .border(Border::new().width(1.).fill(t.border_subtle))
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            rect()
                                                                .width(Size::px(10.))
                                                                .height(Size::px(10.))
                                                                .corner_radius(5.)
                                                                .background(t.accent_orange)
                                                                .margin((0., 12., 0., 0.))
                                                        )
                                                        .child(
                                                            rect()
                                                                .child(
                                                                    label()
                                                                        .font_size(14.)
                                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                                        .color(t.text_primary)
                                                                        .text(doc)
                                                                )
                                                                .child(
                                                                    rect()
                                                                        .horizontal()
                                                                        .spacing(8.)
                                                                        .margin((2., 0., 0., 0.))
                                                                        .child(label().font_size(12.).color(t.accent_orange).text(stat))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(format!("Job {}", j_id)))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(format!("User: {}", u)))
                                                                        .child(label().font_size(12.).color(t.text_muted).text("•"))
                                                                        .child(label().font_size(12.).color(t.text_secondary).text(sz))
                                                                )
                                                        )
                                                )
                                                .child(
                                                    secondary_button("Cancel Job", {
                                                        let j_c = j_id.clone();
                                                        let mut ps_c = ps;
                                                        let mut l_c = l;
                                                        move || {
                                                            let job = j_c.clone();
                                                            let mut curr = ps_c.read().clone();
                                                            curr.active_jobs.retain(|j| j.id != job);
                                                            ps_c.set(curr);
                                                            std::thread::spawn(move || {
                                                                cancel_print_job(&job);
                                                            });
                                                            l_c.set(false);
                                                        }
                                                    })
                                                )
                                                .into_element()
                                        }))
                                        .into_element()
                                }
                            })
                    )

                    // --- Global Printing Preferences Card ---
                    .child(
                        rect()
                            .width(Size::fill())
                            .margin((0., 0., 16., 0.))
                            .padding(16.)
                            .corner_radius(12.)
                            .background(t.bg_card)
                            .border(Border::new().width(1.).fill(t.border_card))
                            .overflow(Overflow::Clip)
                            .child(
                                label()
                                    .font_size(13.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_secondary)
                                    .margin((0., 0., 14., 0.))
                                    .text("GLOBAL PRINTING PREFERENCES")
                            )
                            // Network Discovery
                            .child(
                                rect()
                                    .horizontal()
                                    .main_align(Alignment::SpaceBetween)
                                    .cross_align(Alignment::Center)
                                    .width(Size::fill())
                                    .margin((0., 0., 14., 0.))
                                    .content(Content::Flex)
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(
                                                label()
                                                    .font_size(15.)
                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                    .color(t.text_primary)
                                                    .text("Automatic Network Discovery")
                                            )
                                            .child(
                                                label()
                                                    .font_size(13.)
                                                    .color(t.text_secondary)
                                                    .margin((4., 0., 0., 0.))
                                                    .text("Automatically detect mDNS / Bonjour and IPP Everywhere printers on the local LAN.")
                                            )
                                    )
                                    .child(
                                        Switch::new()
                                            .toggled(discovery_on)
                                            .on_toggle({
                                                let mut ps = printers_state;
                                                move |_| {
                                                    let mut curr = ps.read().clone();
                                                    curr.network_discovery = !curr.network_discovery;
                                                    ps.set(curr);
                                                }
                                            })
                                    )
                            )
                            // Share Printers on Local Network
                            .child(
                                rect()
                                    .horizontal()
                                    .main_align(Alignment::SpaceBetween)
                                    .cross_align(Alignment::Center)
                                    .width(Size::fill())
                                    .content(Content::Flex)
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .child(
                                                label()
                                                    .font_size(15.)
                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                    .color(t.text_primary)
                                                    .text("Share Printers on Local Network")
                                            )
                                            .child(
                                                label()
                                                    .font_size(13.)
                                                    .color(t.text_secondary)
                                                    .margin((4., 0., 0., 0.))
                                                    .text("Allow other devices on this subnet to send print jobs to this machine's printers.")
                                            )
                                    )
                                    .child(
                                        Switch::new()
                                            .toggled(info.share_printers)
                                            .on_toggle({
                                                let mut ps = printers_state;
                                                move |_| {
                                                    let mut curr = ps.read().clone();
                                                    curr.share_printers = !curr.share_printers;
                                                    ps.set(curr);
                                                }
                                            })
                                    )
                            )
                    )
            )
    }
}
