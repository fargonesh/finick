use freya::prelude::*;
use ipsea::settings::{SettingKey, SubscriptionFilter, SETTINGS_SOCKET_NAME, subscribe_channel};
use ui::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OsdKind {
    Volume,
    Brightness,
}

#[derive(PartialEq)]
pub struct OsdOverlay {
    pub visible: State<bool>,
    pub kind: State<OsdKind>,
    pub value: State<f64>,
    pub muted: State<bool>,
}

impl Component for OsdOverlay {
    fn render(&self) -> impl IntoElement {
        let visible = self.visible;
        let kind = self.kind;
        let value = self.value;
        let muted = self.muted;
        let t = use_app_theme();
        if !*visible.read() {
            return rect().width(Size::fill()).height(Size::fill()).background(Color::TRANSPARENT).into_element();
        }
        let k = *kind.read();
        let v = *value.read();
        let is_muted = *muted.read();
        let icon_svg = match k {
            OsdKind::Volume => SOUND,
            OsdKind::Brightness => SUN,
        };
        let label_text = match k {
            OsdKind::Volume => if is_muted { "Muted".to_string() } else { format!("Volume {}%", v as i32) },
            OsdKind::Brightness => format!("Brightness {}%", v as i32),
        };
        let pct = v.clamp(0.0, 100.0) as f32;
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(Color::from_argb(90, 0, 0, 0))
            .center()
            .content(Content::Flex)
            .on_press({
                let mut vis = visible;
                move |_| vis.set(false)
            })
            .child(
                rect()
                    .width(Size::px(360.))
                    .padding(16.)
                    .corner_radius(16.)
                    .background(t.panel)
                    .border(Border::new().width(1.).fill(t.border))
                    .vertical()
                    .spacing(10.)
                    .content(Content::Flex)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(10.)
                            .content(Content::Flex)
                            .child(icon(icon_svg, 18., t.text))
                            .child(label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(label_text)),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::px(8.))
                            .corner_radius(999.)
                            .background(t.track)
                            .child(rect().width(Size::percent(pct)).height(Size::fill()).corner_radius(999.).background(t.accent)),
                    ),
            )
            .into_element()
    }
}

pub fn use_osd_state() -> (State<bool>, State<OsdKind>, State<f64>, State<bool>) {
    let visible = use_state(|| false);
    let kind = use_state(|| OsdKind::Volume);
    let value = use_state(|| 50.0);
    let muted = use_state(|| false);
    let r#gen = use_state(|| 0u64);
    use_hook(move || {
        let mut vis = visible;
        let mut kd = kind;
        let mut val = value;
        let mut mu = muted;
        let mut g = r#gen;
        spawn(async move {
            let init = tokio::task::spawn_blocking(|| ipsea::settings::get_all_settings(SETTINGS_SOCKET_NAME)).await.unwrap_or(Ok(vec![])).unwrap_or_default();
            for e in init {
                if e.key == SettingKey::AudioVolume {
                    if let Some(v) = e.value.as_f64() { val.set(v); }
                    if let Some(v) = e.value.as_i64() { val.set(v as f64); }
                }
                if e.key == SettingKey::DisplayBrightness {
                    if let Some(v) = e.value.as_f64() { val.set(v); }
                }
                if e.key == SettingKey::AudioMuted {
                    if let Some(b) = e.value.as_bool() { mu.set(b); }
                }
            }
            if let Ok(mut rx) = subscribe_channel(SETTINGS_SOCKET_NAME, SubscriptionFilter::all()) {
                while let Some(evt) = rx.recv().await {
                    if let ipsea::settings::SettingsEvent::Changed { key, value: v, .. } = evt {
                        let mut show = false;
                        let mut new_kind = OsdKind::Volume;
                        let mut new_val = 0.0;
                        match key {
                            SettingKey::AudioVolume => {
                                new_kind = OsdKind::Volume;
                                new_val = v.as_f64().unwrap_or(v.as_i64().map(|x| x as f64).unwrap_or(0.0));
                                show = true;
                            }
                            SettingKey::AudioMuted => {
                                if let Some(b) = v.as_bool() { mu.set(b); }
                                new_kind = OsdKind::Volume;
                                new_val = *val.read();
                                show = true;
                            }
                            SettingKey::DisplayBrightness => {
                                new_kind = OsdKind::Brightness;
                                new_val = v.as_f64().unwrap_or(0.0);
                                show = true;
                            }
                            _ => {}
                        }
                        if show {
                            kd.set(new_kind);
                            val.set(new_val);
                            vis.set(true);
                            let cur = g.read().wrapping_add(1);
                            g.set(cur);
                            let mut vis2 = vis;
                            let g2 = g;
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                                if *g2.read() == cur { vis2.set(false); }
                            });
                        }
                    }
                }
            }
        });
    });
    (visible, kind, value, muted)
}

pub fn osd_app() -> Element {
    let t = use_init_app_theme(get_theme());
    let (vis, kd, val, mu) = use_osd_state();
    let _theme = t;
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(OsdOverlay { visible: vis, kind: kd, value: val, muted: mu })
        .into_element()
}

pub fn osd_window_config() -> WindowConfig {
    WindowConfig::new(osd_app)
        .with_title("osd")
        .with_app_id("osd")
        .with_size(420., 160.)
        .with_decorations(false)
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
}
