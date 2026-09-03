use freya::prelude::*;

#[derive(PartialEq, Clone)]
struct CoolComp1;
impl Component for CoolComp1 {
    fn render(&self) -> impl IntoElement {
        let _s = use_state(|| 1);
        rect()
    }
}

#[derive(PartialEq, Clone)]
struct CoolComp2;
impl Component for CoolComp2 {
    fn render(&self) -> impl IntoElement {
        let _s = use_state(|| 1);
        let _s2 = use_state(|| 2);
        rect()
    }
}

fn app() -> impl IntoElement {
    let route = use_state(|| 1);
    
    rect().child({
        match *route.read() {
            1 => CoolComp1.into_element(),
            _ => CoolComp2.into_element(),
        }
    })
}

fn main() {}
