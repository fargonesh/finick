use freya::prelude::*;
use crate::theme::{use_app_theme, RADIUS_SM};

#[derive(Clone, Debug)]
pub struct DisplayMockItem {
    pub name: String,
    pub height_px: f32,
}

/// Display monitor mock arrangement matching .displays-mock in ui_demo.html
pub fn displays_mock(displays: Vec<DisplayMockItem>) -> impl IntoElement {
    let t = use_app_theme();

    rect()
        .width(Size::fill())
        .horizontal()
        .cross_align(Alignment::End)
        .spacing(14.)
        .margin((4., 0., 0., 0.))
            .content(Content::Flex)
        .children(displays.into_iter().map(move |disp| {
            rect()
                .width(Size::flex(1.))
                .vertical()
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(disp.height_px))
                        .corner_radius(RADIUS_SM)
                        .background(t.panel_raised)
                        .border(Border::new().width(1.).fill(t.border)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .center()
                        .margin((6., 0., 0., 0.))
                        .child(
                            label()
                                .font_size(11.)
                                .color(t.text_dim)
                                .text(disp.name),
                        ),
                )
        }))
}
