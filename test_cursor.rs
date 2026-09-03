use freya::prelude::*;
fn test() {
    let _ = rect()
        .on_pointer_enter(move |_| Cursor::set(CursorIcon::Pointer))
        .on_pointer_leave(move |_| Cursor::set(CursorIcon::default()));
}
