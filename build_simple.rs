use std::fs;

fn main() {
    let mut main_rs = String::new();

    main_rs.push_str(r#"#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use freya::prelude::*;
use ui::*;
use system::*;
use std::process::Command;

#[derive(Clone, PartialEq)]
pub enum Route {
    General,
    Appearance,
    Network,
    Bluetooth,
    Accounts,
    Sound,
    Displays,
    Power,
    Input,
    Storage,
    About,
    QuickActions,
}

pub fn main() {
    launch(LaunchConfig::new().with_window(WindowConfig::new(app).with_title("Settings").with_size(1000., 700.)))
}

fn app() -> impl IntoElement {
    use_init_app_theme(DARK_THEME);
    let t = use_app_theme();
    let current_route = use_state(|| Route::General);

    let nav_item = |route: Route, text: &str| {
        let is_active = *current_route.read() == route;
        let bg = if is_active { t.bg_selected } else { Color::TRANSPARENT };
        let mut cr = current_route.clone();
        let r = route.clone();
        
        rect()
            .width(Size::fill())
            .padding(12.)
            .corner_radius(8.)
            .background(bg)
            .margin((0., 0., 4., 0.))
            .on_press(move |_| cr.set(r.clone()))
            .child(label().font_size(14.).color(t.text_primary).text(text.to_string()))
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .horizontal()
        .background(t.bg_base)
        .child(
            rect()
                .width(Size::px(250.))
                .height(Size::fill())
                .background(t.bg_sidebar)
                .padding(24.)
                .border(Border::new().width(1.).fill(t.border_subtle))
                .child(
                    rect()
                        .margin((0., 0., 24., 0.))
                        .child(label().font_size(24.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Settings"))
                )
                .child(
                    ScrollView::new()
                        .width(Size::fill())
                        .height(Size::fill())
                        .child(nav_item(Route::General, "General"))
                        .child(nav_item(Route::Appearance, "Appearance"))
                        .child(nav_item(Route::Network, "Network"))
                        .child(nav_item(Route::Bluetooth, "Bluetooth"))
                        .child(nav_item(Route::Accounts, "Accounts"))
                        .child(nav_item(Route::Sound, "Sound"))
                        .child(nav_item(Route::Displays, "Displays"))
                        .child(nav_item(Route::Power, "Power"))
                        .child(nav_item(Route::Input, "Mouse & Keyboard"))
                        .child(nav_item(Route::Storage, "Storage"))
                        .child(nav_item(Route::About, "About"))
                        .child(rect().margin((24., 0., 0., 0.)).child(nav_item(Route::QuickActions, "⚡ Quick Actions")))
                )
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .padding(48.)
                .child({
                    match *current_route.read() {
                        Route::General => General.render().into_element(),
                        Route::Appearance => Appearance.render().into_element(),
                        Route::Network => Network.render().into_element(),
                        Route::Bluetooth => Bluetooth.render().into_element(),
                        Route::Accounts => Accounts.render().into_element(),
                        Route::Sound => Sound.render().into_element(),
                        Route::Displays => Displays.render().into_element(),
                        Route::Power => Power.render().into_element(),
                        Route::Input => Input.render().into_element(),
                        Route::Storage => Storage.render().into_element(),
                        Route::About => About.render().into_element(),
                        Route::QuickActions => QuickActions.render().into_element(),
                    }
                })
        )
}

#[derive(PartialEq)]
struct General;
impl Component for General { fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text("General Settings")) } }

#[derive(PartialEq)]
struct Network;
impl Component for Network { fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text("Network Settings")) } }

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
            std::thread::spawn(move || { let _ = tx.send(HyprlandBackend.get_audio_info()); });
            freya::prelude::spawn(async move { if let Some(info) = rx.recv().await { audio_state.set(info); } });
        }
        let info = audio.read().clone();
        rect()
            .child(page_header("Sound", "Manage audio output and volume."))
            .child(
                rect().margin((0., 0., 24., 0.)).padding(24.).corner_radius(12.).background(t.bg_card).border(Border::new().width(1.).fill(t.border_card))
                .child(label().font_size(14.).font_weight(FontWeight::BOLD).color(t.text_secondary).margin((0., 0., 16., 0.)).text("OUTPUT"))
                .child(
                    rect().horizontal().cross_align(Alignment::Center).margin((0., 0., 16., 0.))
                    .child(rect().width(Size::fill()).child(label().font_size(16.).color(t.text_primary).text(info.default_sink_name)))
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
            std::thread::spawn(move || { let _ = tx.send(HyprlandBackend.get_displays()); });
            freya::prelude::spawn(async move { if let Some(info) = rx.recv().await { d_state.set(info); } });
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
                .child(rect().width(Size::fill()).child(label().font_size(16.).font_weight(FontWeight::BOLD).color(t.text_primary).text("Battery")))
                .child(label().color(t.text_secondary).text(format!("{}% ({})", info.capacity, info.status)))
            )
    }
}

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
                        .child(rect().width(Size::fill()).child(label().color(t.text_primary).text(d.name.clone())))
                        .child(label().color(t.text_secondary).text(d.layout_or_type.clone()))
                        .into_element()
                }))
            )
    }
}

#[derive(PartialEq)]
struct About;
impl Component for About { fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text("About System")) } }

#[derive(PartialEq)]
struct QuickActions;
impl Component for QuickActions { fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text("Quick Actions")) } }
"#);

    let accounts = fs::read_to_string("accounts.rs").unwrap().replace("use freya::prelude::*;\nuse ui::*;\n", "");
    let appearance = fs::read_to_string("appearance.rs").unwrap().replace("use std::process::Command;\n", "");
    let storage = fs::read_to_string("storage.rs").unwrap();
    let bluetooth = fs::read_to_string("bluetooth.rs").unwrap();

    main_rs.push_str(&accounts);
    main_rs.push_str(&appearance);
    main_rs.push_str(&storage);
    main_rs.push_str(&bluetooth);

    fs::write("apps/settings/src/main.rs", main_rs).unwrap();
}
