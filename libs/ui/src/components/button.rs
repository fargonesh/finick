use {
    crate::{
        icons::icon,
        theme::{RADIUS_PILL, use_app_theme},
    },
    freya::prelude::*,
};

fn wrap_press(mut on_press: impl FnMut() + 'static) -> EventHandler<()> {
    EventHandler::new(move |_| on_press())
}

#[derive(Clone, PartialEq)]
struct ActionButton {
    text: String,
    full: bool,
    kind: ActionKind,
    on_press: EventHandler<()>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActionKind {
    Primary,
    Secondary,
    Danger,
}

impl Component for ActionButton {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;

        let (normal_bg, hover_bg) = match self.kind {
            ActionKind::Primary => (t.primary_accent, t.accent_blue_hover),
            ActionKind::Secondary => (t.bg_card, t.bg_hover),
            ActionKind::Danger => (
                t.accent_red,
                Color::from_rgb(0xFF, 0x7A, 0x66),
            ),
        };
        let bg = if is_hovered { hover_bg } else { normal_bg };
        let border_fill = if is_kb { t.accent } else { t.border_card };
        let border_w = if is_kb { 2. } else { 1. };

        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();
        let text_str = self.text.clone();
        let full = self.full;

        rect()
            .maybe(full, |el| el.width(Size::fill()))
            .padding((10., 16.))
            .corner_radius(8.)
            .background(bg)
            .border(Border::new().width(border_w).fill(border_fill))
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(
                label()
                    .font_size(14.)
                    .font_weight(FontWeight::SEMI_BOLD)
                    .color(t.text_primary)
                    .text(text_str),
            )
    }
}

/// A primary accent-styled action button.
pub fn primary_button(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ActionButton {
        text: text.into(),
        full: false,
        kind: ActionKind::Primary,
        on_press: wrap_press(on_press),
    }
}

/// A secondary outlined action button with surface background.
pub fn secondary_button(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ActionButton {
        text: text.into(),
        full: false,
        kind: ActionKind::Secondary,
        on_press: wrap_press(on_press),
    }
}

pub fn primary_button_full(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ActionButton {
        text: text.into(),
        full: true,
        kind: ActionKind::Primary,
        on_press: wrap_press(on_press),
    }
}

pub fn secondary_button_full(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ActionButton {
        text: text.into(),
        full: true,
        kind: ActionKind::Secondary,
        on_press: wrap_press(on_press),
    }
}

/// A destructive/danger action button with red background.
pub fn danger_button(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ActionButton {
        text: text.into(),
        full: false,
        kind: ActionKind::Danger,
        on_press: wrap_press(on_press),
    }
}

pub fn danger_button_full(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ActionButton {
        text: text.into(),
        full: true,
        kind: ActionKind::Danger,
        on_press: wrap_press(on_press),
    }
}

#[derive(Clone, PartialEq)]
struct IconTextButton {
    icon_text: String,
    on_press: EventHandler<()>,
}

impl Component for IconTextButton {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();

        rect()
            .width(Size::px(36.))
            .height(Size::px(36.))
            .corner_radius(8.)
            .background(if is_hovered { t.bg_hover } else { t.bg_card })
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 1. })
                    .fill(if is_kb { t.accent } else { t.border_card }),
            )
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(
                label()
                    .font_size(16.)
                    .color(t.text_primary)
                    .text(self.icon_text.clone()),
            )
    }
}

/// An icon button with subtle rounded background.
pub fn icon_button(icon: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    IconTextButton {
        icon_text: icon.into(),
        on_press: wrap_press(on_press),
    }
}

#[derive(Clone, PartialEq)]
struct GhostButtonInner {
    text: String,
    full: bool,
    on_press: EventHandler<()>,
}

impl Component for GhostButtonInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();

        rect()
            .maybe(self.full, |el| el.width(Size::fill()).content(Content::Flex))
            .padding((6., 12.))
            .corner_radius(999.)
            .background(if is_hovered { t.track } else { t.panel_raised })
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 1. })
                    .fill(if is_kb { t.accent } else { t.border }),
            )
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(
                label()
                    .font_size(12.)
                    .font_weight(FontWeight::MEDIUM)
                    .color(t.text)
                    .max_lines(Some(1))
                    .text(self.text.clone()),
            )
    }
}

/// A ghost button matching .ghost-btn in ui_demo.html (pill shape, panel_raised, border)
pub fn ghost_button(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    GhostButtonInner {
        text: text.into(),
        full: false,
        on_press: wrap_press(on_press),
    }
}

pub fn ghost_button_full(text: impl Into<String>, on_press: impl FnMut() + 'static) -> impl IntoElement {
    GhostButtonInner {
        text: text.into(),
        full: true,
        on_press: wrap_press(on_press),
    }
}

#[derive(Clone, PartialEq)]
struct GhostIconButtonInner {
    svg: &'static str,
    full: bool,
    on_press: EventHandler<()>,
}

impl Component for GhostIconButtonInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();

        rect()
            .maybe(self.full, |el| el.width(Size::fill()).content(Content::Flex))
            .padding((7., 12.))
            .corner_radius(999.)
            .background(if is_hovered { t.track } else { t.panel_raised })
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 1. })
                    .fill(if is_kb { t.accent } else { t.border }),
            )
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(icon(self.svg, 14., t.text))
    }
}

/// Ghost pill button with an SVG icon instead of text. Use for media controls and other icon-only actions.
pub fn ghost_icon_button(svg: &'static str, on_press: impl FnMut() + 'static) -> impl IntoElement {
    GhostIconButtonInner {
        svg,
        full: false,
        on_press: wrap_press(on_press),
    }
}

/// Full-width ghost pill button with an SVG icon. The button itself fills, do not wrap it in a flex rect.
pub fn ghost_icon_button_full(svg: &'static str, on_press: impl FnMut() + 'static) -> impl IntoElement {
    GhostIconButtonInner {
        svg,
        full: true,
        on_press: wrap_press(on_press),
    }
}

#[derive(Clone, PartialEq)]
struct ToolButtonInner {
    svg: &'static str,
    icon_color: Color,
    on_press: EventHandler<()>,
}

impl Component for ToolButtonInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();

        rect()
            .width(Size::px(30.))
            .height(Size::px(30.))
            .corner_radius(8.)
            .background(if is_hovered { t.track } else { t.panel_raised })
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 1. })
                    .fill(if is_kb { t.accent } else { t.border }),
            )
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(icon(self.svg, 14., self.icon_color))
    }
}

/// A uniform square icon button for row actions (pin, copy, delete).
pub fn tool_button(icon_svg: &'static str, icon_color: Color, on_press: impl FnMut() + 'static) -> impl IntoElement {
    ToolButtonInner {
        svg: icon_svg,
        icon_color,
        on_press: wrap_press(on_press),
    }
}

#[derive(Clone, PartialEq)]
struct CloseButtonInner {
    on_press: EventHandler<()>,
}

impl Component for CloseButtonInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();

        rect()
            .width(Size::px(28.))
            .height(Size::px(28.))
            .corner_radius(RADIUS_PILL)
            .background(if is_hovered { t.track } else { t.panel_raised })
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 1. })
                    .fill(if is_kb { t.accent } else { t.border }),
            )
            .center()
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(
                label()
                    .font_size(14.)
                    .color(if is_hovered { t.text } else { t.text_dim })
                    .text("×"),
            )
    }
}

/// A focus pill button with indicator dot matching .focus-pill in ui_demo.html
pub fn small_close_button(on_press: impl FnMut() + 'static) -> impl IntoElement {
    CloseButtonInner {
        on_press: wrap_press(on_press),
    }
}

#[derive(Clone, PartialEq)]
struct FocusPillInner {
    label_text: String,
    is_active: bool,
    on_press: EventHandler<()>,
}

impl Component for FocusPillInner {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let hovered = use_state(|| false);
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let is_hovered = *hovered.read();
        let is_kb = focus() == Focus::Keyboard;
        let mut h1 = hovered;
        let mut h2 = hovered;
        let on_press = self.on_press.clone();
        let border_color = if is_kb {
            t.accent
        } else if self.is_active {
            t.accent
        } else {
            t.border
        };
        let text_color = if self.is_active { t.accent } else { t.text_dim };
        let dot_color = if self.is_active { t.accent } else { t.text_dim };

        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .padding((7., 12.))
            .corner_radius(999.)
            .background(if is_hovered { t.track } else { t.panel_raised })
            .border(
                Border::new()
                    .width(if is_kb { 2. } else { 1. })
                    .fill(border_color),
            )
            .cursor(CursorIcon::Pointer)
            .a11y_id(a11y_id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::RadioButton)
            .on_pointer_enter(move |_| h1.set(true))
            .on_pointer_leave(move |_| h2.set(false))
            .on_press(move |_| on_press.call(()))
            .child(
                rect()
                    .width(Size::px(6.))
                    .height(Size::px(6.))
                    .corner_radius(999.)
                    .background(dot_color)
                    .margin((0., 6., 0., 0.)),
            )
            .child(
                label()
                    .font_size(12.5)
                    .font_weight(FontWeight::MEDIUM)
                    .color(text_color)
                    .text(self.label_text.clone()),
            )
    }
}

pub fn focus_pill(label_text: impl Into<String>, is_active: bool, on_press: impl FnMut() + 'static) -> impl IntoElement {
    FocusPillInner {
        label_text: label_text.into(),
        is_active,
        on_press: wrap_press(on_press),
    }
}
