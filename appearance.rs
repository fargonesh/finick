use std::process::Command;

fn get_hypr_option(option: &str, default: f64) -> f64 {
    if let Ok(output) = Command::new("hyprctl").args(["getoption", option]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("int:") {
                if let Ok(n) = rest.trim().parse::<f64>() { return n; }
            } else if let Some(rest) = line.strip_prefix("custom type:") {
                if let Some(n) = rest.split_whitespace().next().and_then(|p| p.parse::<f64>().ok()) { return n; }
            }
        }
    }
    if let Ok(output) = Command::new("hyprctl").args(["getoption", option, "-j"]).output() {
        // Fallback omitting json parse for brevity
    }
    default
}

#[derive(PartialEq)]
struct Appearance;

impl Component for Appearance {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut theme_state = use_app_theme_state();
        let is_dark = theme_state.read().mode == ThemeMode::Dark;

        let mut gaps_in = use_state(|| 5.0f64);
        let mut gaps_out = use_state(|| 20.0f64);
        let mut border_size = use_state(|| 1.0f64);
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut gi_state = gaps_in.clone();
            let mut go_state = gaps_out.clone();
            let mut bs_state = border_size.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let gi = get_hypr_option("general:gaps_in", 5.0);
                let go = get_hypr_option("general:gaps_out", 20.0);
                let bs = get_hypr_option("general:border_size", 1.0);
                let _ = tx.send((gi, go, bs));
            });
            freya::prelude::spawn(async move {
                if let Some((gi, go, bs)) = rx.recv().await {
                    gi_state.set(gi);
                    go_state.set(go);
                    bs_state.set(bs);
                }
            });
        }

        rect()
            .child(page_header("Appearance", "Customize how your system and windows look."))
            .child(
                rect()
                    .margin((0., 0., 24., 0.))
                    .padding(24.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_secondary).margin((0., 0., 16., 0.)).text("THEME"))
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .margin((0., 0., 16., 0.))
                            .child(label().font_size(16.).color(t.text_primary).width(Size::fill()).text("Dark Mode"))
                            .child(
                                Switch::new()
                                    .toggled(is_dark)
                                    .on_toggle(move |_| {
                                        let new_mode = if is_dark { ThemeMode::Light } else { ThemeMode::Dark };
                                        let mut new_theme = match new_mode { ThemeMode::Light => LIGHT_THEME, ThemeMode::Dark => DARK_THEME };
                                        new_theme.primary_accent = theme_state.read().primary_accent;
                                        theme_state.set(new_theme);
                                    })
                            )
                    )
                    .child(
                        rect()
                            .margin((0., 0., 8., 0.))
                            .child(label().font_size(16.).color(t.text_primary).margin((0., 0., 8., 0.)).text("Accent Color"))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .child(
                                        rect()
                                            .width(Size::px(32.)).height(Size::px(32.)).corner_radius(16.).background(t.accent_blue)
                                            .border(Border::new().width(if t.primary_accent == t.accent_blue { 2. } else { 0. }).fill(t.text_primary))
                                            .on_pointer_enter(|_| { Cursor::set(CursorIcon::Pointer); })
                                            .on_pointer_leave(|_| { Cursor::set(CursorIcon::default()); })
                                            .on_press({ let mut ts = theme_state.clone(); let blue = t.accent_blue; move |_| { let mut theme = *ts.read(); theme.primary_accent = blue; ts.set(theme); } })
                                    )
                                    .child(
                                        rect()
                                            .width(Size::px(32.)).height(Size::px(32.)).corner_radius(16.).background(t.accent_red)
                                            .border(Border::new().width(if t.primary_accent == t.accent_red { 2. } else { 0. }).fill(t.text_primary))
                                            .on_pointer_enter(|_| { Cursor::set(CursorIcon::Pointer); })
                                            .on_pointer_leave(|_| { Cursor::set(CursorIcon::default()); })
                                            .on_press({ let mut ts = theme_state.clone(); let red = t.accent_red; move |_| { let mut theme = *ts.read(); theme.primary_accent = red; ts.set(theme); } })
                                    )
                                    .child(
                                        rect()
                                            .width(Size::px(32.)).height(Size::px(32.)).corner_radius(16.).background(t.accent_green)
                                            .border(Border::new().width(if t.primary_accent == t.accent_green { 2. } else { 0. }).fill(t.text_primary))
                                            .on_pointer_enter(|_| { Cursor::set(CursorIcon::Pointer); })
                                            .on_pointer_leave(|_| { Cursor::set(CursorIcon::default()); })
                                            .on_press({ let mut ts = theme_state.clone(); let green = t.accent_green; move |_| { let mut theme = *ts.read(); theme.primary_accent = green; ts.set(theme); } })
                                    )
                                    .child(
                                        rect()
                                            .width(Size::px(32.)).height(Size::px(32.)).corner_radius(16.).background(t.accent_orange)
                                            .border(Border::new().width(if t.primary_accent == t.accent_orange { 2. } else { 0. }).fill(t.text_primary))
                                            .on_pointer_enter(|_| { Cursor::set(CursorIcon::Pointer); })
                                            .on_pointer_leave(|_| { Cursor::set(CursorIcon::default()); })
                                            .on_press({ let mut ts = theme_state.clone(); let orange = t.accent_orange; move |_| { let mut theme = *ts.read(); theme.primary_accent = orange; ts.set(theme); } })
                                    )
                                    .child(
                                        rect()
                                            .width(Size::px(32.)).height(Size::px(32.)).corner_radius(16.).background(t.accent_purple)
                                            .border(Border::new().width(if t.primary_accent == t.accent_purple { 2. } else { 0. }).fill(t.text_primary))
                                            .on_pointer_enter(|_| { Cursor::set(CursorIcon::Pointer); })
                                            .on_pointer_leave(|_| { Cursor::set(CursorIcon::default()); })
                                            .on_press({ let mut ts = theme_state.clone(); let purple = t.accent_purple; move |_| { let mut theme = *ts.read(); theme.primary_accent = purple; ts.set(theme); } })
                                    )
                            )
                    )
            )
            .child(
                rect()
                    .margin((0., 0., 24., 0.))
                    .padding(24.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_secondary).margin((0., 0., 16., 0.)).text("WINDOW & GAPS (HYPRLAND)"))
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(16.)
                            .margin((0., 0., 16., 0.))
                            .child(label().font_size(14.).color(t.text_secondary).width(Size::px(120.)).text("Inner Gaps"))
                            .child(
                                Slider::new({
                                    let mut gi = gaps_in.clone();
                                    move |val| {
                                        let px = (val / 100.0 * 50.0).round();
                                        gi.set(px);
                                        std::thread::spawn(move || {
                                            let _ = Command::new("hyprctl")
                                                .args(["keyword", "general:gaps_in", &format!("{}", px as i32)])
                                                .output();
                                        });
                                    }
                                }).value((*gaps_in.read() / 50.0 * 100.0).clamp(0.0, 100.0))
                            )
                            .child(label().font_size(14.).color(t.text_secondary).width(Size::px(48.)).text(format!("{}px", *gaps_in.read() as i32)))
                    )
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(16.)
                            .margin((0., 0., 16., 0.))
                            .child(label().font_size(14.).color(t.text_secondary).width(Size::px(120.)).text("Outer Gaps"))
                            .child(
                                Slider::new({
                                    let mut go = gaps_out.clone();
                                    move |val| {
                                        let px = (val / 100.0 * 100.0).round();
                                        go.set(px);
                                        std::thread::spawn(move || {
                                            let _ = Command::new("hyprctl")
                                                .args(["keyword", "general:gaps_out", &format!("{}", px as i32)])
                                                .output();
                                        });
                                    }
                                }).value((*gaps_out.read() / 100.0 * 100.0).clamp(0.0, 100.0))
                            )
                            .child(label().font_size(14.).color(t.text_secondary).width(Size::px(48.)).text(format!("{}px", *gaps_out.read() as i32)))
                    )
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(16.)
                            .child(label().font_size(14.).color(t.text_secondary).width(Size::px(120.)).text("Border Size"))
                            .child(
                                Slider::new({
                                    let mut bs = border_size.clone();
                                    move |val| {
                                        let px = (val / 100.0 * 20.0).round();
                                        bs.set(px);
                                        std::thread::spawn(move || {
                                            let _ = Command::new("hyprctl")
                                                .args(["keyword", "general:border_size", &format!("{}", px as i32)])
                                                .output();
                                        });
                                    }
                                }).value((*border_size.read() / 20.0 * 100.0).clamp(0.0, 100.0))
                            )
                            .child(label().font_size(14.).color(t.text_secondary).width(Size::px(48.)).text(format!("{}px", *border_size.read() as i32)))
                    )
            )
    }
}
