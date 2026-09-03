
#[derive(PartialEq)]
struct Storage;

impl Component for Storage {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let mut storage_data = use_state(|| None::<StorageInfo>);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut storage_setter = storage_data.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let backend = HyprlandBackend;
                let info = backend.get_storage_info();
                let _ = tx.send(info);
            });

            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    storage_setter.set(Some(info));
                }
            });
        }

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
                                            let mut l = loaded.clone();
                                            move || l.set(false)
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
