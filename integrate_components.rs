use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/settings/src/main.rs").unwrap();

    let storage_mock = "#[derive(PartialEq)]\nstruct Storage;\nimpl Component for Storage { fn render(&self) -> impl IntoElement { rect() } }";
    let bluetooth_mock = "#[derive(PartialEq)]\nstruct Bluetooth;\nimpl Component for Bluetooth { fn render(&self) -> impl IntoElement { rect() } }";
    
    let acc_start = main_rs.find("#[derive(PartialEq)]\nstruct Accounts;").unwrap();
    let snd_start = main_rs.find("#[derive(PartialEq)]\nstruct Sound;").unwrap();
    let old_accounts = main_rs[acc_start..snd_start].to_string();

    let app_start = main_rs.find("#[derive(PartialEq)]\nstruct Appearance;").unwrap();
    let net_start = main_rs.find("#[derive(PartialEq)]\nstruct Network;").unwrap();
    let old_appearance = main_rs[app_start..net_start].to_string();

    let accounts_new = fs::read_to_string("accounts.rs").unwrap();
    let accounts_new = accounts_new.replace("use freya::prelude::*;\nuse ui::*;\n", "");
    
    let appearance_new = fs::read_to_string("appearance.rs").unwrap();
    let appearance_new = appearance_new.replace("use std::process::Command;\n", "");
    
    let storage_new = fs::read_to_string("storage.rs").unwrap();
    let bluetooth_new = fs::read_to_string("bluetooth.rs").unwrap();

    main_rs = main_rs.replace(storage_mock, &storage_new);
    main_rs = main_rs.replace(bluetooth_mock, &bluetooth_new);
    main_rs = main_rs.replace(&old_accounts, &format!("{}\n", accounts_new));
    main_rs = main_rs.replace(&old_appearance, &format!("{}\n", appearance_new));

    if !main_rs.contains("use std::process::Command;") {
        main_rs = main_rs.replace("use freya::prelude::*;", "use freya::prelude::*;\nuse std::process::Command;");
    }

    fs::write("apps/settings/src/main.rs", main_rs).unwrap();
}
