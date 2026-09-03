use std::fs;

fn main() {
    let mut main_rs = String::new();

    main_rs.push_str(r#"#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use freya::{
    prelude::*,
    router::*,
};
use ui::*;
use system::*;
use std::process::Command;

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(AppSideBar)]
    #[route("/")]
    General,
    #[route("/appearance")]
    Appearance,
    #[route("/network")]
    Network,
    #[route("/bluetooth")]
    Bluetooth,
    #[route("/accounts")]
    Accounts,
    #[route("/sound")]
    Sound,
    #[route("/displays")]
    Displays,
    #[route("/power")]
    Power,
    #[route("/input")]
    Input,
    #[route("/storage")]
    Storage,
    #[route("/about")]
    About,
    #[route("/quick-actions")]
    QuickActions,
}

#[allow(non_snake_case)]
#[component]
fn AppSideBar() -> Element {
    let t = use_app_theme();
    rsx!(
        rect {
            width: "100%",
            height: "100%",
            direction: "horizontal",
            background: "{t.bg_base.to_rgb_string()}",
            rect {
                width: "250",
                height: "100%",
                background: "{t.bg_sidebar.to_rgb_string()}",
                padding: "24",
                border: "0 1 0 0 solid {t.border_subtle.to_rgb_string()}",
                rect {
                    margin: "0 0 24 0",
                    label { font_size: "24", font_weight: "bold", color: "{t.text_primary.to_rgb_string()}", "Settings" }
                }
                ScrollView {
                    width: "100%",
                    height: "fill",
                    SidebarItem { to: Route::General, text: "General".to_string() }
                    SidebarItem { to: Route::Appearance, text: "Appearance".to_string() }
                    SidebarItem { to: Route::Network, text: "Network".to_string() }
                    SidebarItem { to: Route::Bluetooth, text: "Bluetooth".to_string() }
                    SidebarItem { to: Route::Accounts, text: "Accounts".to_string() }
                    SidebarItem { to: Route::Sound, text: "Sound".to_string() }
                    SidebarItem { to: Route::Displays, text: "Displays".to_string() }
                    SidebarItem { to: Route::Power, text: "Power".to_string() }
                    SidebarItem { to: Route::Input, text: "Mouse & Keyboard".to_string() }
                    SidebarItem { to: Route::Storage, text: "Storage".to_string() }
                    SidebarItem { to: Route::About, text: "About".to_string() }
                    rect { margin: "24 0 0 0", SidebarItem { to: Route::QuickActions, text: "⚡ Quick Actions".to_string() } }
                }
            }
            rect {
                width: "fill",
                height: "100%",
                padding: "48",
                Outlet::<Route> {}
            }
        }
    )
}

#[allow(non_snake_case)]
#[component]
fn SidebarItem(to: Route, text: String) -> Element {
    let t = use_app_theme();
    let nav = use_navigator();
    let current = use_route::<Route>();
    let is_active = current.map(|c| c == to).unwrap_or(false);
    
    let bg = if is_active { t.bg_selected } else { Color::TRANSPARENT };
    
    rsx!(
        rect {
            width: "100%",
            padding: "12",
            corner_radius: "8",
            background: "{bg.to_rgb_string()}",
            margin: "0 0 4 0",
            cursor: "pointer",
            onclick: move |_| { nav.push(to.clone()); },
            label { color: "{t.text_primary.to_rgb_string()}", font_size: "14", "{text}" }
        }
    )
}

#[derive(PartialEq)]
struct General;
impl Component for General { fn render(&self) -> impl IntoElement { rect().child(label().text("General Settings")) } }

#[derive(PartialEq)]
struct Network;
impl Component for Network { fn render(&self) -> impl IntoElement { rect().child(label().text("Network Settings")) } }

#[derive(PartialEq)]
struct Sound;
impl Component for Sound { fn render(&self) -> impl IntoElement { rect().child(label().text("Sound Settings")) } }

#[derive(PartialEq)]
struct Displays;
impl Component for Displays { fn render(&self) -> impl IntoElement { rect().child(label().text("Display Settings")) } }

#[derive(PartialEq)]
struct Power;
impl Component for Power { fn render(&self) -> impl IntoElement { rect().child(label().text("Power Settings")) } }

#[derive(PartialEq)]
struct Input;
impl Component for Input { fn render(&self) -> impl IntoElement { rect().child(label().text("Input Settings")) } }

#[derive(PartialEq)]
struct About;
impl Component for About { fn render(&self) -> impl IntoElement { rect().child(label().text("About System")) } }

#[derive(PartialEq)]
struct QuickActions;
impl Component for QuickActions { fn render(&self) -> impl IntoElement { rect().child(label().text("Quick Actions")) } }
"#);

    let accounts = fs::read_to_string("accounts.rs").unwrap().replace("use freya::prelude::*;\nuse ui::*;\n", "");
    let appearance = fs::read_to_string("appearance.rs").unwrap().replace("use std::process::Command;\n", "");
    let storage = fs::read_to_string("storage.rs").unwrap();
    let bluetooth = fs::read_to_string("bluetooth.rs").unwrap();

    main_rs.push_str(&accounts);
    main_rs.push_str(&appearance);
    main_rs.push_str(&storage);
    main_rs.push_str(&bluetooth);

    main_rs.push_str(r#"
pub fn main() {
    launch(LaunchConfig::new().with_window(WindowConfig::new(app).with_title("Settings").with_size(1000., 700.)))
}

fn app() -> impl IntoElement {
    use_init_app_theme(DARK_THEME);
    Router::<Route>::new(RouterConfig::default)
}
"#);

    fs::write("apps/settings/src/main.rs", main_rs).unwrap();
}
