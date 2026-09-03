#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use freya::prelude::*;
use ui::*;
use system::*;
use std::process::Command;
mod pages;
use pages::*;

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
    DateTime,
    Privacy,
    Language,
    Printers,
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
                        .child(nav_item(Route::DateTime, "Date & Time"))
                        .child(nav_item(Route::Privacy, "Privacy & Security"))
                        .child(nav_item(Route::Language, "Language & Region"))
                        .child(nav_item(Route::Printers, "Printers & Scanners"))
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
                        Route::General => General.into_element(),
                        Route::Appearance => Appearance.into_element(),
                        Route::Network => Network.into_element(),
                        Route::Bluetooth => Bluetooth.into_element(),
                        Route::Accounts => Accounts.into_element(),
                        Route::Sound => Sound.into_element(),
                        Route::Displays => Displays.into_element(),
                        Route::Power => Power.into_element(),
                        Route::Input => Input.into_element(),
                        Route::Storage => Storage.into_element(),
                        Route::About => About.into_element(),
                        Route::DateTime => DateTime.into_element(),
                        Route::Privacy => Privacy.into_element(),
                        Route::Language => Language.into_element(),
                        Route::Printers => Printers.into_element(),
                        Route::QuickActions => QuickActions.into_element(),
                    }
                })
        )
}

#[derive(PartialEq)]
struct General;
impl Component for General { fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text("General Settings")) } }


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
                            move |val: f64| {
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
struct QuickActions;
impl Component for QuickActions { fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text("Quick Actions")) } }

#[derive(Clone, Debug, PartialEq)]
pub struct UserAccount {
    pub username: String,
    pub uid: u32,
    pub shell: String,
}

pub fn parse_passwd_users(content: &str) -> Vec<UserAccount> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 7 {
                let username = fields[0].to_string();
                let uid = fields[2].parse::<u32>().ok()?;
                let shell = fields[6].to_string();

                if (1000..60000).contains(&uid) {
                    return Some(UserAccount { username, uid, shell });
                }
            }
            None
        })
        .collect()
}

#[derive(PartialEq)]
struct Accounts;

impl Component for Accounts {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let mut users = use_state(|| Vec::<UserAccount>::new());
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut users_state = users.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<UserAccount>>();

            tokio::spawn(async move {
                if let Ok(content) = tokio::fs::read_to_string("/etc/passwd").await {
                    let parsed = parse_passwd_users(&content);
                    let _ = tx.send(parsed);
                }
            });

            freya::prelude::spawn(async move {
                if let Some(user_list) = rx.recv().await {
                    users_state.set(user_list);
                }
            });
        }

        let user_list = users.read().clone();

        rect()
            .child(page_header("Accounts", "Manage user accounts and view system users."))
            .child(
                rect()
                    .margin((0., 0., 16., 0.))
                    .child(
                        label()
                            .font_size(16.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_primary)
                            .margin((0., 0., 12., 0.))
                            .text("Human Users (UID 1000 - 59999)")
                    )
            )
            .child({
                if user_list.is_empty() {
                    rect()
                        .padding(24.)
                        .corner_radius(12.)
                        .background(t.bg_card)
                        .border(Border::new().width(1.).fill(t.border_card))
                        .center()
                        .child(
                            label()
                                .font_size(14.)
                                .color(t.text_secondary)
                                .text(if !*loaded.read() {
                                    "Loading users..."
                                } else {
                                    "No human users found."
                                }),
                        )
                        .into_element()
                } else {
                    rect()
                        .children(user_list.into_iter().map(|user| {
                            let initial = user
                                .username
                                .chars()
                                .next()
                                .map(|c| c.to_uppercase().to_string())
                                .unwrap_or_else(|| "U".to_string());

                            rect()
                                .margin((0., 0., 12., 0.))
                                .padding(16.)
                                .corner_radius(12.)
                                .background(t.bg_card)
                                .border(Border::new().width(1.).fill(t.border_card))
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .child(
                                    rect()
                                        .width(Size::px(48.))
                                        .height(Size::px(48.))
                                        .corner_radius(24.)
                                        .background(t.primary_accent)
                                        .center()
                                        .margin((0., 16., 0., 0.))
                                        .child(
                                            label()
                                                .font_size(20.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(t.bg_base)
                                                .text(initial),
                                        ),
                                )
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .child(
                                            label()
                                                .font_size(16.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(t.text_primary)
                                                .text(user.username.clone()),
                                        )
                                        .child(
                                            rect()
                                                .horizontal()
                                                .spacing(8.)
                                                .margin((4., 0., 0., 0.))
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_secondary)
                                                        .text(format!("UID: {}", user.uid)),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_muted)
                                                        .text("•"),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_secondary)
                                                        .text(format!("Shell: {}", user.shell)),
                                                ),
                                        ),
                                )
                                .into_element()
                        }))
                        .into_element()
                }
            })
    }
}

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
                                    move |val: f64| {
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
                                    move |val: f64| {
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
                                    move |val: f64| {
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

