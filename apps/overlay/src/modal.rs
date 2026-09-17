use freya::prelude::*;
use ipsea::modals::{ModalRequest, ModalResponse};
use ui::*;
use system::{HyprlandBackend, SystemBackend};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct ModalState {
    pub req: Arc<Mutex<Option<(ModalRequest, std::sync::mpsc::Sender<ModalResponse>, winit::window::WindowId)>>>,
}

#[derive(PartialEq, Clone)]
struct ModalApp;

impl Component for ModalApp {
    fn render(&self) -> impl IntoElement {
        let _st = use_init_app_theme(get_theme());
        let t = use_app_theme();
        
        let ctx_opt = GlobalContexts::get().try_get_context::<ModalState>();
        let Some(ctx) = ctx_opt else {
            return rect().into_element();
        };

        let req_lock = ctx.req.lock().unwrap();
        let Some((req, tx, wid)) = req_lock.clone() else {
            return rect().into_element();
        };
        drop(req_lock); // drop lock before rendering

        let password = use_state(String::new);
        let is_working = use_state(|| false);
        
        match req {
            ModalRequest::WifiPassword { ssid, security } => {
                let pwd = password;
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(Color::from_argb(120, 0, 0, 0))
                    .center()
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(400.))
                            .padding(20.)
                            .corner_radius(16.)
                            .background(t.panel)
                            .border(Border::new().width(1.).fill(t.border))
                            .spacing(12.)
                            .child(label().font_size(18.).font_weight(FontWeight::BOLD).color(t.text).text(format!("Join {}", ssid)))
                            .child(label().font_size(12.).color(t.text_dim).text(format!("Security: {}", security)))
                            .child(Input::new(pwd.into_writable()))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(10.)
                                    .child(
                                        Button::new().on_press({
                                            let ctx = ctx.clone();
                                            let tx = tx.clone();
                                            move |_| {
                                                let _ = tx.send(ModalResponse::Canceled);
                                                *ctx.req.lock().unwrap() = None;
                                                Platform::get().close_window(wid);
                                            }
                                        }).child("Cancel")
                                    )
                                    .child(
                                        Button::new().on_press({
                                            let p = pwd.clone();
                                            let s = ssid.clone();
                                            let ctx = ctx.clone();
                                            let mut iw = is_working;
                                            let tx = tx.clone();
                                            move |_| {
                                                iw.set(true);
                                                let p_str = p.read().clone();
                                                let s_str = s.clone();
                                                let tx2 = tx.clone();
                                                let ctx2 = ctx.clone();
                                                std::thread::spawn(move || {
                                                    let _ = HyprlandBackend.connect_wifi_with_password(&s_str, Some(&p_str), false);
                                                    let _ = tx2.send(ModalResponse::Success { data: None });
                                                    *ctx2.req.lock().unwrap() = None;
                                                    Platform::get().close_window(wid);
                                                });
                                            }
                                        }).child(if *is_working.read() { "Connecting..." } else { "Connect" })
                                    )
                            )
                    ).into_element()
            },
            ModalRequest::BluetoothPair { name, mac } => {
                let pwd = password;
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(Color::from_argb(120, 0, 0, 0))
                    .center()
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(400.))
                            .padding(20.)
                            .corner_radius(16.)
                            .background(t.panel)
                            .border(Border::new().width(1.).fill(t.border))
                            .spacing(12.)
                            .child(label().font_size(18.).font_weight(FontWeight::BOLD).color(t.text).text(format!("Pair {}", name)))
                            .child(label().font_size(12.).color(t.text_dim).text(format!("MAC: {}", mac)))
                            .child(Input::new(pwd.into_writable()))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(10.)
                                    .child(
                                        Button::new().on_press({
                                            let ctx = ctx.clone();
                                            let tx = tx.clone();
                                            move |_| {
                                                let _ = tx.send(ModalResponse::Canceled);
                                                *ctx.req.lock().unwrap() = None;
                                                Platform::get().close_window(wid);
                                            }
                                        }).child("Cancel")
                                    )
                                    .child(
                                        Button::new().on_press({
                                            let m = mac.clone();
                                            let p = pwd.clone();
                                            let ctx = ctx.clone();
                                            let mut iw = is_working;
                                            let tx = tx.clone();
                                            move |_| {
                                                iw.set(true);
                                                let m_str = m.clone();
                                                let p_str = p.read().clone();
                                                let tx2 = tx.clone();
                                                let ctx2 = ctx.clone();
                                                std::thread::spawn(move || {
                                                    let _ = HyprlandBackend.pair_bluetooth_device(&m_str, if p_str.is_empty() { None } else { Some(&p_str) });
                                                    let _ = tx2.send(ModalResponse::Success { data: None });
                                                    *ctx2.req.lock().unwrap() = None;
                                                    Platform::get().close_window(wid);
                                                });
                                            }
                                        }).child(if *is_working.read() { "Pairing..." } else { "Pair" })
                                    )
                            )
                    ).into_element()
            },
            ModalRequest::PamAuth { prompt } => {
                let pwd = password;
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(Color::from_argb(120, 0, 0, 0))
                    .center()
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(400.))
                            .padding(20.)
                            .corner_radius(16.)
                            .background(t.panel)
                            .border(Border::new().width(1.).fill(t.border))
                            .spacing(12.)
                            .child(label().font_size(18.).font_weight(FontWeight::BOLD).color(t.text).text(prompt))
                            .child(Input::new(pwd.into_writable()))
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(10.)
                                    .child(
                                        Button::new().on_press({
                                            let ctx = ctx.clone();
                                            let tx = tx.clone();
                                            move |_| {
                                                let _ = tx.send(ModalResponse::Canceled);
                                                *ctx.req.lock().unwrap() = None;
                                                Platform::get().close_window(wid);
                                            }
                                        }).child("Cancel")
                                    )
                                    .child(
                                        Button::new().on_press({
                                            let p = pwd.clone();
                                            let ctx = ctx.clone();
                                            let mut iw = is_working;
                                            let tx = tx.clone();
                                            move |_| {
                                                iw.set(true);
                                                let p_str = p.read().clone();
                                                let tx2 = tx.clone();
                                                let ctx2 = ctx.clone();
                                                std::thread::spawn(move || {
                                                    let _ = tx2.send(ModalResponse::Success { data: Some(p_str) });
                                                    *ctx2.req.lock().unwrap() = None;
                                                    Platform::get().close_window(wid);
                                                });
                                            }
                                        }).child(if *is_working.read() { "Authenticating..." } else { "Authenticate" })
                                    )
                            )
                    ).into_element()
            },
            ModalRequest::Clipboard => {
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(Color::from_argb(120, 0, 0, 0))
                    .center()
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(520.))
                            .height(Size::px(520.))
                            .corner_radius(16.)
                            .background(t.panel)
                            .border(Border::new().width(1.).fill(t.border))
                            .child(crate::clipboard_manager::ClipboardManager)
                    ).into_element()
            },
            ModalRequest::Screenshot => {
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(Color::from_argb(120, 0, 0, 0))
                    .center()
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::px(500.))
                            .corner_radius(16.)
                            .background(t.panel)
                            .border(Border::new().width(1.).fill(t.border))
                            .child(crate::screenshot::ScreenshotUtility)
                    ).into_element()
            },
            _ => rect().into_element()
        }
    }
}

pub fn fullscreen_modal_app() -> impl IntoElement {
    ModalApp
}
