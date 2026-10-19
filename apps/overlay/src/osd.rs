use {
    freya::prelude::*,
    ipsea::settings::{SETTINGS_SOCKET_NAME, SettingKey, SubscriptionFilter, subscribe_channel},
    ui::*,
};

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
    pub reduced: State<bool>,
}

impl Component for OsdOverlay {
    fn render(&self) -> impl IntoElement {
        let visible = self.visible;
        let kind = self.kind;
        let value = self.value;
        let muted = self.muted;
        let reduced = self.reduced;
        let t = use_app_theme();
        if !*visible.read() {
            return rect().width(Size::fill()).height(Size::fill()).background(Color::TRANSPARENT).into_element();
        }
        let k = *kind.read();
        let v = *value.read();
        let is_muted = *muted.read();
        let is_reduced = *reduced.read();
        let icon_svg = match k {
            OsdKind::Volume => SOUND,
            OsdKind::Brightness => SUN,
        };
        let label_text = match k {
            OsdKind::Volume => {
                if is_muted {
                    "Muted".to_string()
                } else {
                    format!("Volume {}%", v as i32)
                }
            }
            OsdKind::Brightness => format!("Brightness {}%", v as i32),
        };
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
                FadeSlideIn::new().reduced(is_reduced).child(
                    dialog_card(360.)
                        .spacing(10.)
                        .child(
                            rect()
                                .width(Size::fill())
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(10.)
                                .content(Content::Flex)
                                .child(icon(icon_svg, 18., t.text))
                                .child(
                                    label().font_size(13.).font_weight(FontWeight::SEMI_BOLD).color(t.text).text(label_text),
                                ),
                        )
                        .child(input_level_pill(v)),
                ),
            )
            .into_element()
    }
}

pub fn use_osd_state() -> (State<bool>, State<OsdKind>, State<f64>, State<bool>, State<bool>) {
    let visible = use_state(|| false);
    let kind = use_state(|| OsdKind::Volume);
    let value = use_state(|| 50.0);
    let muted = use_state(|| false);
    let reduced = use_state(|| false);
    let r#gen = use_state(|| 0u64);
    use_hook(move || {
        let mut vis = visible;
        let mut kd = kind;
        let mut val = value;
        let mut mu = muted;
        let mut red = reduced;
        let mut g = r#gen;
        let mut show_for = move |new_kind: OsdKind, new_val: f64, cur: u64| {
            kd.set(new_kind);
            val.set(new_val);
            vis.set(true);
            g.set(cur);
            let mut vis2 = vis;
            let g2 = g;
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                if *g2.read() == cur {
                    vis2.set(false);
                }
            });
        };
        spawn(async move {
            loop {
                let init = tokio::task::spawn_blocking(|| ipsea::settings::get_all_settings(SETTINGS_SOCKET_NAME))
                    .await
                    .unwrap_or(Ok(vec![]))
                    .unwrap_or_default();
                for e in init {
                    if e.key == SettingKey::AudioVolume {
                        if let Some(v) = e.value.as_f64() {
                            val.set(v);
                        }
                        if let Some(v) = e.value.as_i64() {
                            val.set(v as f64);
                        }
                    }
                    if e.key == SettingKey::DisplayBrightness {
                        if let Some(v) = e.value.as_f64() {
                            val.set(v);
                        }
                    }
                    if e.key == SettingKey::AudioMuted {
                        if let Some(b) = e.value.as_bool() {
                            mu.set(b);
                        }
                    }
                    if e.key == SettingKey::AccessibilityReduceMotion {
                        if let Some(b) = e.value.as_bool() {
                            red.set(b);
                        }
                    }
                }
                // Prefer live hardware values when the tools exist so the OSD
                // reflects reality even if the daemon state is stale.
                let live_vol = tokio::task::spawn_blocking(crate::state::query_sink_volume_pct).await.unwrap_or(None);
                if let Some(v) = live_vol {
                    val.set(v);
                }
                let live_mute = tokio::task::spawn_blocking(crate::state::query_sink_muted).await.unwrap_or(None);
                if let Some(m) = live_mute {
                    mu.set(m);
                }
                let live_bright = tokio::task::spawn_blocking(crate::state::query_brightness_pct).await.unwrap_or(None);
                if let Some(b) = live_bright {
                    // Keep the settings value until we know which kind changed.
                    let _ = b;
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
                                    if let Some(b) = v.as_bool() {
                                        mu.set(b);
                                    }
                                    new_kind = OsdKind::Volume;
                                    new_val = tokio::task::spawn_blocking(crate::state::query_sink_volume_pct)
                                        .await
                                        .unwrap_or(None)
                                        .unwrap_or(*val.read());
                                    show = true;
                                }
                                SettingKey::DisplayBrightness => {
                                    new_kind = OsdKind::Brightness;
                                    new_val = v.as_f64().unwrap_or(0.0);
                                    show = true;
                                }
                                SettingKey::AccessibilityReduceMotion => {
                                    if let Some(b) = v.as_bool() {
                                        red.set_if_modified(b);
                                    }
                                }
                                _ => {}
                            }
                            if show {
                                let cur = g.read().wrapping_add(1);
                                show_for(new_kind, new_val, cur);
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        });
        // Live poll: volume/brightness keys often change hardware directly
        // (wpctl/brightnessctl), so re-query and surface the OSD when the
        // live value moves even without a settings event.
        let mut p_vis = visible;
        let mut p_kd = kind;
        let mut p_val = value;
        let mut p_mu = muted;
        let mut p_gen = r#gen;
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                let live = tokio::task::spawn_blocking(|| {
                    (
                        crate::state::query_sink_volume_pct(),
                        crate::state::query_sink_muted(),
                        crate::state::query_brightness_pct(),
                    )
                })
                .await
                .unwrap_or((None, None, None));
                let (lv, lm, lb) = live;
                if let Some(m) = lm {
                    if m != *p_mu.read() {
                        p_mu.set(m);
                        let cur = p_gen.read().wrapping_add(1);
                        p_kd.set(OsdKind::Volume);
                        p_val.set(lv.unwrap_or(*p_val.read()));
                        p_vis.set(true);
                        p_gen.set(cur);
                        let mut v2 = p_vis;
                        let g2 = p_gen;
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                            if *g2.read() == cur {
                                v2.set(false);
                            }
                        });
                        continue;
                    }
                }
                if let Some(v) = lv {
                    if (v - *p_val.read()).abs() > 0.5 && !*p_vis.read() {
                        // Volume keys move hardware directly; pop the OSD
                        // even when the last shown kind was brightness.
                        p_val.set(v);
                        let cur = p_gen.read().wrapping_add(1);
                        p_kd.set(OsdKind::Volume);
                        p_vis.set(true);
                        p_gen.set(cur);
                        let mut v2 = p_vis;
                        let g2 = p_gen;
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                            if *g2.read() == cur {
                                v2.set(false);
                            }
                        });
                    } else if *p_vis.read() && *p_kd.read() == OsdKind::Volume {
                        p_val.set_if_modified(v);
                    }
                }
                if let Some(b) = lb {
                    if (b - *p_val.read()).abs() > 0.5 && (*p_kd.read() != OsdKind::Brightness || !*p_vis.read()) {
                        // Brightness keys move hardware directly; pop the OSD
                        // the same way volume does.
                        p_val.set(b);
                        let cur = p_gen.read().wrapping_add(1);
                        p_kd.set(OsdKind::Brightness);
                        p_vis.set(true);
                        p_gen.set(cur);
                        let mut v2 = p_vis;
                        let g2 = p_gen;
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                            if *g2.read() == cur {
                                v2.set(false);
                            }
                        });
                    } else if *p_vis.read() && *p_kd.read() == OsdKind::Brightness {
                        p_val.set_if_modified(b);
                    }
                }
            }
        });
    });
    (visible, kind, value, muted, reduced)
}

pub fn osd_app() -> Element {
    let t = use_init_app_theme(get_theme());
    let (vis, kd, val, mu, red) = use_osd_state();
    let _theme = t;
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(Color::TRANSPARENT)
        .content(Content::Flex)
        .child(OsdOverlay { visible: vis, kind: kd, value: val, muted: mu, reduced: red })
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
