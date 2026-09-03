use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/settings/src/main.rs.recovered").unwrap();
    main_rs = main_rs.replace("NativeRouter::new().child(\n            rect()", "rect()");
    main_rs = main_rs.replace("        )\n    }\n}\n\nfn nav_btn", "    }\n}\n\nfn nav_btn");

    fs::write("apps/settings/src/main.rs.recovered", main_rs).unwrap();
}
