use freya::prelude::*;
use std::process::Command;
use ui::*;

#[derive(Clone, Debug, PartialEq)]
pub struct DiskInfo {
    pub filesystem: String,
    pub mount_point: String,
    pub total: String,
    pub used: String,
    pub available: String,
    pub use_percent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageInfo {
    pub total: String,
    pub used: String,
    pub available: String,
    pub use_percent: f64,
    pub disks: Vec<DiskInfo>,
}

fn fetch_storage_info() -> StorageInfo {
    let mut disks = Vec::new();
    let mut root_disk = DiskInfo {
        filesystem: "/dev/root".to_string(),
        mount_point: "/".to_string(),
        total: "512 GB".to_string(),
        used: "128 GB".to_string(),
        available: "384 GB".to_string(),
        use_percent: 25.0,
    };

    if let Ok(output) = Command::new("df").args(["-hP"]).output() {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parsed_any = false;

        for line in text.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let (fs, size, used, avail, pct_str, mount) =
                    (parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
                let pct = pct_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);

                if fs.starts_with("tmpfs")
                    || fs.starts_with("devtmpfs")
                    || fs.starts_with("efivarfs")
                    || fs.starts_with("none")
                    || fs.starts_with("overlay")
                    || fs.starts_with("udev")
                    || mount.starts_with("/run")
                    || mount.starts_with("/sys")
                    || mount.starts_with("/dev")
                {
                    continue;
                }

                let disk = DiskInfo {
                    filesystem: fs.to_string(),
                    mount_point: mount.to_string(),
                    total: size.to_string(),
                    used: used.to_string(),
                    available: avail.to_string(),
                    use_percent: pct,
                };

                if mount == "/" {
                    root_disk = disk.clone();
                }

                disks.push(disk);
                parsed_any = true;
            }
        }

        if !parsed_any {
            disks.push(root_disk.clone());
        }
    } else {
        disks.push(root_disk.clone());
    }

    StorageInfo {
        total: root_disk.total,
        used: root_disk.used,
        available: root_disk.available,
        use_percent: root_disk.use_percent,
        disks,
    }
}

#[derive(PartialEq)]
pub struct Storage;

impl Component for Storage {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut storage_data = use_state(|| Some(fetch_storage_info()));

        rect()
            .child(page_header(
                "Storage",
                "Disk space, mount points, and drive capacity.",
            ))
            .child({
                if let Some(storage) = storage_data.read().clone() {
                    let pct = storage.use_percent.clamp(0.0, 100.0);
                    let bar_color = if pct > 90.0 {
                        t.accent_red
                    } else if pct > 75.0 {
                        t.accent_orange
                    } else {
                        t.primary_accent
                    };

                    rect()
                        .child(
                            rect()
                                .margin((0., 0., 24., 0.))
                                .padding(24.)
                                .corner_radius(12.)
                                .background(t.bg_card)
                                .border(Border::new().width(1.).fill(t.border_card))
                                .overflow(Overflow::Clip)
                                .child(
                                    rect()
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .margin((0., 0., 16., 0.))
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .child(
                                                    label()
                                                        .font_size(14.)
                                                        .font_weight(FontWeight::BOLD)
                                                        .color(t.text_secondary)
                                                        .text("SYSTEM STORAGE (/)"),
                                                ),
                                        )
                                        .child(secondary_button("Refresh", {
                                            let mut setter = storage_data.clone();
                                            move || setter.set(Some(fetch_storage_info()))
                                        })),
                                )
                                .child(
                                    rect()
                                        .horizontal()
                                        .cross_align(Alignment::End)
                                        .margin((0., 0., 12., 0.))
                                        .child(
                                            rect().width(Size::fill()).child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::End)
                                                    .child(
                                                        label()
                                                            .font_size(26.)
                                                            .font_weight(FontWeight::BOLD)
                                                            .color(t.text_primary)
                                                            .text(storage.used.clone()),
                                                    )
                                                    .child(
                                                        label()
                                                            .font_size(16.)
                                                            .color(t.text_secondary)
                                                            .margin((0., 0., 2., 4.))
                                                            .text(format!(
                                                                " used of {}",
                                                                storage.total
                                                            )),
                                                    ),
                                            ),
                                        )
                                        .child(
                                            label()
                                                .font_size(20.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(bar_color)
                                                .text(format!("{:.0}%", pct)),
                                        ),
                                )
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .height(Size::px(10.))
                                        .corner_radius(5.)
                                        .background(t.bg_base)
                                        .overflow(Overflow::Clip)
                                        .child(
                                            rect()
                                                .width(Size::percent(pct as f32))
                                                .height(Size::fill())
                                                .corner_radius(5.)
                                                .background(bar_color),
                                        ),
                                )
                                .child(
                                    rect()
                                        .horizontal()
                                        .margin((16., 0., 0., 0.))
                                        .spacing(24.)
                                        .child(
                                            rect()
                                                .child(
                                                    label()
                                                        .font_size(12.)
                                                        .color(t.text_secondary)
                                                        .text("Used Space"),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(16.)
                                                        .font_weight(FontWeight::BOLD)
                                                        .color(t.text_primary)
                                                        .text(storage.used.clone()),
                                                ),
                                        )
                                        .child(
                                            rect()
                                                .child(
                                                    label()
                                                        .font_size(12.)
                                                        .color(t.text_secondary)
                                                        .text("Available Space"),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(16.)
                                                        .font_weight(FontWeight::BOLD)
                                                        .color(t.text_primary)
                                                        .text(storage.available.clone()),
                                                ),
                                        )
                                        .child(
                                            rect()
                                                .child(
                                                    label()
                                                        .font_size(12.)
                                                        .color(t.text_secondary)
                                                        .text("Total Capacity"),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(16.)
                                                        .font_weight(FontWeight::BOLD)
                                                        .color(t.text_primary)
                                                        .text(storage.total.clone()),
                                                ),
                                        ),
                                ),
                        )
                        .child(
                            rect()
                                .padding(24.)
                                .corner_radius(12.)
                                .background(t.bg_card)
                                .border(Border::new().width(1.).fill(t.border_card))
                                .overflow(Overflow::Clip)
                                .child(
                                    label()
                                        .font_size(14.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(t.text_secondary)
                                        .margin((0., 0., 16., 0.))
                                        .text("MOUNTED DRIVES & PARTITIONS"),
                                )
                                .children(storage.disks.into_iter().map(|disk| {
                                    let disk_pct = disk.use_percent.clamp(0.0, 100.0);
                                    let d_color = if disk_pct > 90.0 {
                                        t.accent_red
                                    } else if disk_pct > 75.0 {
                                        t.accent_orange
                                    } else {
                                        t.primary_accent
                                    };

                                    rect()
                                        .margin((0., 0., 16., 0.))
                                        .padding(14.)
                                        .corner_radius(8.)
                                        .background(t.bg_base)
                                        .border(Border::new().width(1.).fill(t.border_subtle))
                                        .child(
                                            rect()
                                                .horizontal()
                                                .cross_align(Alignment::Center)
                                                .margin((0., 0., 8., 0.))
                                                .child(
                                                    rect().width(Size::fill()).child(
                                                        rect()
                                                            .horizontal()
                                                            .spacing(8.)
                                                            .cross_align(Alignment::Center)
                                                            .child(
                                                                label()
                                                                    .font_size(15.)
                                                                    .font_weight(FontWeight::BOLD)
                                                                    .color(t.text_primary)
                                                                    .text(disk.mount_point.clone()),
                                                            )
                                                            .child(
                                                                label()
                                                                    .font_size(12.)
                                                                    .color(t.text_secondary)
                                                                    .text(format!(
                                                                        "({})",
                                                                        disk.filesystem
                                                                    )),
                                                            ),
                                                    ),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_secondary)
                                                        .text(format!(
                                                            "{} / {} ({:.0}%)",
                                                            disk.used, disk.total, disk_pct
                                                        )),
                                                ),
                                        )
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .height(Size::px(6.))
                                                .corner_radius(3.)
                                                .background(t.bg_card)
                                                .overflow(Overflow::Clip)
                                                .child(
                                                    rect()
                                                        .width(Size::percent(disk_pct as f32))
                                                        .height(Size::fill())
                                                        .corner_radius(3.)
                                                        .background(d_color),
                                                ),
                                        )
                                        .into_element()
                                })),
                        )
                        .into_element()
                } else {
                    rect()
                        .padding(24.)
                        .corner_radius(12.)
                        .background(t.bg_card)
                        .border(Border::new().width(1.).fill(t.border_card))
                        .child(
                            label()
                                .color(t.text_muted)
                                .text("Reading disk and storage information..."),
                        )
                        .into_element()
                }
            })
    }
}
