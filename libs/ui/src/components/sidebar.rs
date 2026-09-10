use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_PILL, RADIUS_SM};
use crate::icons::{icon, SEARCH};

/// A nav item matching .nav-item in ui_demo.html
pub fn nav_item(
    icon_svg: &'static str,
    name: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let name_str = name.into();

    let bg = if active {
        t.bg_active
    } else {
        Color::TRANSPARENT
    };

    let text_color = if active {
        t.text
    } else {
        t.text_dim
    };

    let icon_color = if active {
        t.accent
    } else {
        t.text_dim
    };

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .padding((8., 10.))
        .corner_radius(RADIUS_SM)
        .background(bg)
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_press())
        .child({
            let icon_sz = match t.icon_size_pref {
                0 => 14.,
                2 => 21.,
                _ => 17.,
            };
            icon(icon_svg, icon_sz, icon_color)
        })
        .child(
            label()
                .font_size(13.)
                .font_weight(FontWeight::MEDIUM)
                .color(text_color)
                .text(name_str),
        )
}

/// A nav group label matching .nav-group-label in ui_demo.html
pub fn nav_group_label(text: impl Into<String>) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .margin((12., 10., 4., 10.))
        .child(
            label()
                .font_size(11.)
                .font_weight(FontWeight::MEDIUM)
                .color(t.text_dim)
                .text(text.into()),
        )
}

/// Brand avatar row matching .brand-row in ui_demo.html
pub fn brand_row(
    initial: impl Into<String>,
    user_name: impl Into<String>,
    subtitle: impl Into<String>,
) -> impl IntoElement {
    let t = use_app_theme();
    let init_str = initial.into();
    let name_str = user_name.into();
    let sub_str = subtitle.into();

    let user_img = std::env::var("HOME").ok().and_then(|home| {
        let p1 = std::path::PathBuf::from(&home).join(".face.icon");
        let p2 = std::path::PathBuf::from(&home).join(".face");
        let p3 = std::path::PathBuf::from(&home).join(".config/finick/user.png");
        let target = if p1.is_file() { Some(p1) } else if p2.is_file() { Some(p2) } else if p3.is_file() { Some(p3) } else { None };
        target.and_then(|p| std::fs::read(p).ok())
    });

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(10.)
        .padding((4., 6.))
        .child(
            rect()
                .width(Size::px(32.))
                .height(Size::px(32.))
                .corner_radius(RADIUS_PILL)
                .background(t.bg_active)
                .center()
                .child({
                    if let Some(bytes) = user_img {
                        ImageViewer::new(Bytes::from(bytes))
                            .width(Size::px(32.))
                            .height(Size::px(32.))
                            .corner_radius(RADIUS_PILL)
                            .into_element()
                    } else {
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.accent)
                            .text(init_str)
                            .into_element()
                    }
                }),
        )
        .child(
            rect()
                .vertical()
                .child(
                    label()
                        .font_size(13.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(t.text)
                        .text(name_str),
                )
                .maybe(!sub_str.is_empty(), |el| {
                    el.child(
                        label()
                            .font_size(11.5)
                            .color(t.text_dim)
                            .text(sub_str),
                    )
                }),
        )
}

/// Search bar in the sidebar matching .sidebar .search in ui_demo.html
pub fn sidebar_search(
    query: impl Into<Writable<String>>,
    placeholder: &'static str,
) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(8.)
        .padding((8., 12.))
        .corner_radius(RADIUS_PILL)
        .background(t.panel)
        .border(Border::new().width(1.).fill(t.border))
        .child(icon(SEARCH, 14., t.text_dim))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(
                    Input::new(query)
                        .background(Color::TRANSPARENT)
                        .border_fill(Color::TRANSPARENT)
                        .focus_background(Color::TRANSPARENT)
                        .focus_border_fill(Color::TRANSPARENT)
                        .placeholder(placeholder),
                ),
        )
            .content(Content::Flex)
}


/// A navigation item for sidebars (legacy compatibility).
pub fn sidebar_item(
    name: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let bg = if active {
        t.bg_selected
    } else {
        Color::TRANSPARENT
    };
    let text_color = if active {
        t.text_primary
    } else {
        t.text_muted
    };

    rect()
        .width(Size::fill())
        .padding(8.)
        .corner_radius(6.)
        .background(bg)
        .margin((0., 0., 4., 0.))
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(14.)
                .color(text_color)
                .text(name.into()),
        )
}

/// A navigation item for sidebars with an icon or emoji prefix (legacy compatibility).
pub fn sidebar_icon_item(
    icon_str: impl Into<String>,
    name: impl Into<String>,
    active: bool,
    mut on_press: impl FnMut() + 'static,
) -> impl IntoElement {
    let t = use_app_theme();
    let bg = if active {
        t.bg_selected
    } else {
        Color::TRANSPARENT
    };
    let text_color = if active {
        t.text_primary
    } else {
        t.text_muted
    };

    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .width(Size::fill())
        .padding(8.)
        .corner_radius(6.)
        .background(bg)
        .margin((0., 0., 4., 0.))
        .on_press(move |_| on_press())
        .child(
            label()
                .font_size(16.)
                .margin((0., 8., 0., 0.))
                .text(icon_str.into()),
        )
        .child(
            label()
                .font_size(14.)
                .color(text_color)
                .text(name.into()),
        )
}

/// A standard sidebar header title (legacy compatibility).
pub fn sidebar_header(title: impl Into<String>) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .margin((0., 0., 16., 0.))
        .child(
            label()
                .font_size(22.)
                .font_weight(FontWeight::BOLD)
                .color(t.text_primary)
                .text(title.into()),
        )
}

/// A styled sidebar container.
pub fn sidebar_container(
    width_px: f32,
    children: impl IntoIterator<Item = impl IntoElement>,
) -> impl IntoElement {
    let t = use_app_theme();
    rect()
        .width(Size::px(width_px))
        .height(Size::fill())
        .background(t.sidebar_bg)
        .border(Border::new().width(1.).fill(t.border))
        .padding(16.)
        .vertical()
        .children(children)
}

