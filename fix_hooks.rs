use std::fs;

fn main() {
    let mut code = fs::read_to_string("apps/settings/src/main.rs").unwrap();
    
    let old_match = r#"                .child({
                    match *current_route.read() {
                        Route::General => General.render().into_element(),
                        Route::Appearance => Appearance.render().into_element(),
                        Route::Network => Network.render().into_element(),
                        Route::Sound => Sound.render().into_element(),
                        Route::Displays => Displays.render().into_element(),
                        Route::Power => Power.render().into_element(),
                        Route::Input => Input.render().into_element(),
                        Route::Storage => Storage.render().into_element(),
                        Route::Bluetooth => Bluetooth.render().into_element(),
                        Route::Accounts => Accounts.render().into_element(),
                        Route::About => About.render().into_element(),
                        Route::QuickActions => QuickActions.render().into_element(),
                    }
                })"#;

    let new_match = r#"                .child({
                    let view_general = General.render().into_element();
                    let view_appearance = Appearance.render().into_element();
                    let view_network = Network.render().into_element();
                    let view_sound = Sound.render().into_element();
                    let view_displays = Displays.render().into_element();
                    let view_power = Power.render().into_element();
                    let view_input = Input.render().into_element();
                    let view_storage = Storage.render().into_element();
                    let view_bluetooth = Bluetooth.render().into_element();
                    let view_accounts = Accounts.render().into_element();
                    let view_about = About.render().into_element();
                    let view_quickactions = QuickActions.render().into_element();

                    match *current_route.read() {
                        Route::General => view_general,
                        Route::Appearance => view_appearance,
                        Route::Network => view_network,
                        Route::Sound => view_sound,
                        Route::Displays => view_displays,
                        Route::Power => view_power,
                        Route::Input => view_input,
                        Route::Storage => view_storage,
                        Route::Bluetooth => view_bluetooth,
                        Route::Accounts => view_accounts,
                        Route::About => view_about,
                        Route::QuickActions => view_quickactions,
                    }
                })"#;

    code = code.replace(old_match, new_match);
    fs::write("apps/settings/src/main.rs", code).unwrap();
}
