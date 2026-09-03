use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/settings/src/main.rs").unwrap();

    let sound = r#"
#[derive(PartialEq)]
struct Sound;
impl Component for Sound {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut audio = use_state(|| AudioInfo { volume: 50.0, is_muted: false, default_sink_name: "Loading...".to_string() });
        let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut audio_state = audio.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let backend = HyprlandBackend;
                let _ = tx.send(backend.get_audio_info());
            });
            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await { audio_state.set(info); }
            });
        }
        let info = audio.read().clone();
        rect()
            .child(page_header("Sound", "Manage audio output and volume."))
            .child(
                rect().margin((0., 0., 24., 0.)).padding(24.).corner_radius(12.).background(t.bg_card).border(Border::new().width(1.).fill(t.border_card))
                .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_secondary).margin((0., 0., 16., 0.)).text("OUTPUT"))
                .child(
                    rect().horizontal().cross_align(Alignment::Center).margin((0., 0., 16., 0.))
                    .child(label().font_size(16.).color(t.text_primary).width(Size::fill()).text(info.default_sink_name))
                    .child(
                        rect().horizontal().spacing(16.).cross_align(Alignment::Center)
                        .child(label().color(t.text_secondary).text("Mute"))
                        .child(
                            Switch::new().toggled(info.is_muted).on_toggle({
                                let mut a = audio.clone();
                                move |_| {
                                    let mut n = a.read().clone();
                                    n.is_muted = !n.is_muted;
                                    a.set(n);
                                    std::thread::spawn(|| { HyprlandBackend.toggle_mute(); });
                                }
                            })
                        )
                    )
                )
                .child(
                    rect().horizontal().cross_align(Alignment::Center).spacing(16.)
                    .child(label().color(t.text_secondary).text("Volume"))
                    .child(
                        Slider::new({
                            let mut a = audio.clone();
                            move |val| {
                                let mut n = a.read().clone();
                                n.volume = val;
                                a.set(n);
                                std::thread::spawn(move || { HyprlandBackend.set_volume(val as i32); });
                            }
                        }).value(info.volume)
                    )
                    .child(label().color(t.text_secondary).width(Size::px(48.)).text(format!("{}%", info.volume.round() as i32)))
                )
            )
    }
}
"#;

    let displays = r#"
#[derive(PartialEq)]
struct Displays;
impl Component for Displays {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut displays = use_state(|| Vec::<DisplayInfo>::new());
        let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut d_state = displays.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let _ = tx.send(HyprlandBackend.get_displays());
            });
            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await { d_state.set(info); }
            });
        }
        rect()
            .child(page_header("Displays", "Connected monitors and resolutions."))
            .child(
                rect().margin((0., 0., 24., 0.)).padding(24.).corner_radius(12.).background(t.bg_card).border(Border::new().width(1.).fill(t.border_card))
                .children(displays.read().iter().map(|d| {
                    rect().margin((0., 0., 16., 0.)).padding(16.).background(t.bg_base).corner_radius(8.).border(Border::new().width(1.).fill(t.border_subtle))
                        .horizontal().cross_align(Alignment::Center)
                        .child(rect().width(Size::fill()).child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text(d.name.clone())))
                        .child(label().color(t.text_secondary).text(format!("{} @ {}Hz (Scale: {})", d.resolution, d.refresh_rate, d.scale)))
                        .into_element()
                }))
            )
    }
}
"#;

    let power = r#"
#[derive(PartialEq)]
struct Power;
impl Component for Power {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut power = use_state(|| PowerInfo { capacity: "Unknown".to_string(), status: "Unknown".to_string() });
        let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut p_state = power.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || { let _ = tx.send(HyprlandBackend.get_power_info()); });
            freya::prelude::spawn(async move { if let Some(info) = rx.recv().await { p_state.set(info); } });
        }
        let info = power.read().clone();
        rect()
            .child(page_header("Power", "Battery status and power profiles."))
            .child(
                rect().margin((0., 0., 24., 0.)).padding(24.).corner_radius(12.).background(t.bg_card).border(Border::new().width(1.).fill(t.border_card))
                .horizontal().cross_align(Alignment::Center)
                .child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).width(Size::fill()).text("Battery"))
                .child(label().color(t.text_secondary).text(format!("{}% ({})", info.capacity, info.status)))
            )
    }
}
"#;

    let input = r#"
#[derive(PartialEq)]
struct Input;
impl Component for Input {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut input = use_state(|| InputDevices { mice: vec![], keyboards: vec![] });
        let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut i_state = input.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || { let _ = tx.send(HyprlandBackend.get_input_devices()); });
            freya::prelude::spawn(async move { if let Some(info) = rx.recv().await { i_state.set(info); } });
        }
        let info = input.read().clone();
        rect()
            .child(page_header("Mouse & Keyboard", "Input devices."))
            .child(
                rect().margin((0., 0., 24., 0.)).padding(24.).corner_radius(12.).background(t.bg_card).border(Border::new().width(1.).fill(t.border_card))
                .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_secondary).margin((0., 0., 16., 0.)).text("MICE & TOUCHPADS"))
                .children(info.mice.iter().map(|d| {
                    rect().margin((0., 0., 8., 0.)).padding(12.).background(t.bg_base).corner_radius(8.).border(Border::new().width(1.).fill(t.border_subtle))
                        .child(label().color(t.text_primary).text(d.name.clone())).into_element()
                }))
            )
            .child(
                rect().margin((0., 0., 24., 0.)).padding(24.).corner_radius(12.).background(t.bg_card).border(Border::new().width(1.).fill(t.border_card))
                .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_secondary).margin((0., 0., 16., 0.)).text("KEYBOARDS"))
                .children(info.keyboards.iter().map(|d| {
                    rect().margin((0., 0., 8., 0.)).padding(12.).background(t.bg_base).corner_radius(8.).border(Border::new().width(1.).fill(t.border_subtle))
                        .horizontal().cross_align(Alignment::Center)
                        .child(label().width(Size::fill()).color(t.text_primary).text(d.name.clone()))
                        .child(label().color(t.text_secondary).text(d.layout_or_type.clone()))
                        .into_element()
                }))
            )
    }
}
"#;

    let start_snd = main_rs.find("#[derive(PartialEq)]\nstruct Sound;").unwrap();
    let start_abt = main_rs.find("#[derive(PartialEq)]\nstruct About;").unwrap();
    
    let to_replace = &main_rs[start_snd..start_abt];
    main_rs = main_rs.replace(to_replace, &format!("{}\n{}\n{}\n{}\n", sound, displays, power, input));

    fs::write("apps/settings/src/main.rs", main_rs).unwrap();
}
