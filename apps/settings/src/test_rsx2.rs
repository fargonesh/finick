use freya::prelude::*;
use dioxus::prelude::*;

#[allow(non_snake_case)]
fn CoolComp() -> Element {
    let _s = use_state(|| 5);
    rsx!(
        rect {
            label { "Cool" }
        }
    )
}

fn app() -> Element {
    let route = use_state(|| 1);
    
    rsx!(
        rect {
            width: "100%",
            height: "100%",
            if *route.read() == 1 {
                CoolComp {}
            }
        }
    )
}

fn main() {}
