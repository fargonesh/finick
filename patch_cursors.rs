use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/settings/src/main.rs").unwrap();

    let old_nav_item = r#".margin((0., 0., 4., 0.))
            .on_press(move |_| cr.set(r.clone()))"#;
    let new_nav_item = r#".margin((0., 0., 4., 0.))
            .on_pointer_enter(|_| { Cursor::set(CursorIcon::Pointer); })
            .on_pointer_leave(|_| { Cursor::set(CursorIcon::default()); })
            .on_press(move |_| cr.set(r.clone()))"#;
    
    main_rs = main_rs.replace(old_nav_item, new_nav_item);
    
    fs::write("apps/settings/src/main.rs", main_rs).unwrap();
}
