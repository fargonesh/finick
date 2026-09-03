use freya::prelude::*;

#[derive(PartialEq)]
struct CoolComp;
impl Component for CoolComp {
    fn render(&self) -> impl IntoElement {
        rect()
    }
}

fn app() -> impl IntoElement {
    rect().child(CoolComp)
}

fn main() {}
