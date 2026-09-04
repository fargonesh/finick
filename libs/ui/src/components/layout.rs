use freya::prelude::*;

use crate::theme::GAP;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackDirection {
    Horizontal,
    Vertical,
}

pub fn stack(
    direction: StackDirection,
    children: impl IntoIterator<Item = impl IntoElement>,
    overflow: Option<Overflow>,
) -> Rect {
    let mut container = rect()
        .width(Size::fill())
        .spacing(GAP)
        .children(children)
        .content(Content::Flex);

    container = match direction {
        StackDirection::Horizontal => container.horizontal(),
        StackDirection::Vertical => container.vertical(),
    };

    if let Some(overflow) = overflow {
        container = container.overflow(overflow);
    }

    container
}

pub fn horizontal_stack(children: impl IntoIterator<Item = impl IntoElement>) -> Rect {
    stack(StackDirection::Horizontal, children, None)
}

pub fn horizontal_stack_clipped(
    children: impl IntoIterator<Item = impl IntoElement>,
) -> Rect {
    stack(StackDirection::Horizontal, children, Some(Overflow::Clip))
}

pub fn vertical_stack(children: impl IntoIterator<Item = impl IntoElement>) -> Rect {
    stack(StackDirection::Vertical, children, None)
}

pub fn responsive_stack(
    compact: bool,
    children: impl IntoIterator<Item = impl IntoElement>,
) -> Rect {
    if compact {
        vertical_stack(children)
    } else {
        horizontal_stack(children)
    }
}

pub fn grid2(children: impl IntoIterator<Item = impl IntoElement>) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .horizontal()
        .spacing(GAP)
        .children(children)
        .content(Content::Flex)
}

pub fn responsive_view<F, T>(breakpoint: f32, mut render: F) -> impl IntoElement
where
    F: FnMut(bool) -> T,
    T: IntoElement,
{
    let width = use_state(|| 0.0);

    rect()
        .width(Size::fill())
        .on_sized({
            let mut width_state = width;
            move |e: Event<SizedEventData>| {
                let current = e.area.width();
                if (*width_state.read() - current).abs() > 1.0 {
                    width_state.set(current);
                }
            }
        })
        .child(render(*width.read() < breakpoint))
}
