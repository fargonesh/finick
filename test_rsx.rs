use std::fs;

fn main() {
    let mut content = fs::read_to_string("apps/settings/src/main.rs").unwrap();
    
    let old_general = r#"#[derive(PartialEq)]
struct General;
impl Component for General {
    fn render(&self) -> impl IntoElement {
        rect().child(label().color(use_app_theme().text_primary).text("General Settings"))
    }
}"#;

    let new_general = r#"#[allow(non_snake_case)]
fn General() -> Element {
    rsx!(
        rect {
            label {
                color: "{use_app_theme().text_primary.to_rgb_string()}",
                "General Settings"
            }
        }
    )
}"#;

    content = content.replace(old_general, new_general);
    content = content.replace("Route::General => General.render().into_element(),", "Route::General => rsx!( General {} ),");
    
    fs::write("apps/settings/src/main.rs", content).unwrap();
}
