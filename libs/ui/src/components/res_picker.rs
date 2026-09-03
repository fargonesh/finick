use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_MD, RADIUS_PILL};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResolutionChoice {
    LargerText,
    Default,
    MoreSpace,
}

/// A display resolution picker matching .res-options in ui_demo.html
pub fn resolution_picker(
    selected: ResolutionChoice,
    on_select: impl Into<EventHandler<ResolutionChoice>>,
) -> impl IntoElement {
    let t = use_app_theme();
    let on_select: EventHandler<ResolutionChoice> = on_select.into();

    let options = [
        ("Larger Text", ResolutionChoice::LargerText, 3),
        ("Default", ResolutionChoice::Default, 4),
        ("More Space", ResolutionChoice::MoreSpace, 5),
    ];

    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(10.)
            .content(Content::Flex)
        .children(options.into_iter().map(move |(lbl, choice, dot_count)| {
            let is_selected = choice == selected;
            let border_color = if is_selected { t.accent } else { t.border };
            let fg_color = if is_selected { t.accent } else { t.text_dim };
            let on_select = on_select.clone();

            rect()
                .width(Size::flex(1.))
                .padding((12., 10.))
                .corner_radius(RADIUS_MD)
                .background(t.panel_raised)
                .border(Border::new().width(1.).fill(border_color))
                .center()
                .cursor(CursorIcon::Pointer)
                .a11y_role(AccessibilityRole::RadioButton)
                .a11y_focusable(true)
                .on_press(move |_| on_select.call(choice))
                .child(
                    rect()
                        .horizontal()
                        .spacing(3.)
                        .margin((0., 0., 8., 0.))
                        .children((0..dot_count).map(move |_| {
                            rect()
                                .width(Size::px(4.))
                                .height(Size::px(4.))
                                .corner_radius(RADIUS_PILL)
                                .background(fg_color)
                        })),
                )
                .child(
                    label()
                        .font_size(11.5)
                        .color(fg_color)
                        .text(lbl),
                )
        }))
}


