use {
    super::super::theme::use_app_theme,
    freya::prelude::*,
};

#[derive(PartialEq, Clone)]
pub struct FadeScrollList {
    height: f32,
    fade_height: f32,
    fade_color: Option<Color>,
    show_top_fade: bool,
    children: Vec<Element>,
}

impl ChildrenExt for FadeScrollList {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }
}

impl FadeScrollList {
    pub fn new(height: f32) -> Self {
        Self {
            height,
            fade_height: 20.,
            fade_color: None,
            show_top_fade: true,
            children: Vec::new(),
        }
    }

    pub fn fade_height(mut self, h: f32) -> Self {
        self.fade_height = h;
        self
    }

    pub fn fade_color(mut self, c: Color) -> Self {
        self.fade_color = Some(c);
        self
    }

    pub fn show_top_fade(mut self, show: bool) -> Self {
        self.show_top_fade = show;
        self
    }
}

impl Component for FadeScrollList {
    fn render(self: &FadeScrollList) -> impl IntoElement {
        let t = use_app_theme();
        let base = self.fade_color.unwrap_or(t.panel);
        let transparent = base.with_af32(0.);
        let height = self.height;
        let fade_h = self.fade_height;
        let show_top = self.show_top_fade;

        let mut viewport_h = use_state(|| 0_f32);
        let mut content_h = use_state(|| 0_f32);
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        let (_, scroll_y) = scroll.into();

        let viewport = *viewport_h.read();
        let content = *content_h.read();
        let can_scroll = content > viewport + 1. && viewport > 0.;
        let at_top = scroll_y >= -1;
        let at_bottom = if can_scroll {
            scroll_y <= -((content - viewport) as i32) + 1
        } else {
            true
        };
        let show_top_fade = show_top && can_scroll && !at_top;
        let show_bottom_fade = can_scroll && !at_bottom;

        let bottom_gradient: Fill = LinearGradient::new()
            .stop((transparent, 0.))
            .stop((base, 100.))
            .into();
        let top_gradient: Fill = LinearGradient::new()
            .stop((base, 0.))
            .stop((transparent, 100.))
            .into();

        rect()
            .width(Size::fill())
            .height(Size::px(height))
            .overflow(Overflow::Clip)
            .on_sized(move |e: Event<SizedEventData>| {
                let h = e.area.height();
                if (h - *viewport_h.read()).abs() > 0.5 {
                    viewport_h.set(h);
                }
            })
            .on_wheel(move |e: Event<WheelEventData>| {
                let vp = *viewport_h.peek();
                let ct = *content_h.peek();
                if ct <= vp + 1. || vp <= 0. {
                    return;
                }
                let (_, y) = scroll.into();
                let delta = e.delta_y as f32;
                let mut next = y as f32 + delta;
                if next > 0. {
                    next = 0.;
                }
                let min = -(ct - vp);
                if next < min {
                    next = min;
                }
                if scroll.scroll_to_y(next as i32) {
                    e.stop_propagation();
                }
            })
            .child(
                ScrollView::new_controlled(scroll)
                    .width(Size::fill())
                    .height(Size::fill())
                    .show_scrollbar(false)
                    .child(
                        rect()
                            .width(Size::fill())
                            .vertical()
                            .spacing(2.)
                            .padding((0., 0., 6., 0.))
                            .content(Content::Flex)
                            .on_sized(move |e: Event<SizedEventData>| {
                                let h = e.area.height();
                                if (h - *content_h.read()).abs() > 0.5 {
                                    content_h.set(h);
                                }
                            })
                            .children(self.children.clone()),
                    ),
            )
            .maybe(show_top_fade, |el| {
                el.child(
                    rect()
                        .position(Position::new_absolute().top(0.))
                        .width(Size::fill())
                        .height(Size::px(fade_h))
                        .layer(10)
                        .background(top_gradient),
                )
            })
            .maybe(show_bottom_fade, |el| {
                el.child(
                    rect()
                        .position(Position::new_absolute().bottom(0.))
                        .width(Size::fill())
                        .height(Size::px(fade_h))
                        .layer(10)
                        .background(bottom_gradient),
                )
            })
    }
}
