use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/files/src/main.rs").unwrap();
    let old_load_dir = r#"fn load_dir(path: String, mut items_state: State<Vec<Item>>) {
    items_state.set(Vec::new());
    
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    std::thread::spawn(move || {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let tx_clone = inner_tx.clone();
        
        let path_clone = path.clone();
        
        let res = ipsea::send_command(
            App::IndexService,
            &index::ty::Request::ListDir { path: path_clone },
            Some(move |res: index::ty::SearchResult| {
                let _ = tx_clone.send(Item {
                    ty: if res.is_dir { ItemType::Folder } else { ItemType::File },
                    name: res.name,
                    path: res.path,
                    size: res.size.unwrap_or(0),
                });
            })
        );
        
        drop(inner_tx);
        
        for item in inner_rx {
            let _ = tx.send(item);
        }
    });
    
    freya::prelude::spawn(async move {
        let mut all_items = Vec::new();
        while let Some(item) = rx.recv().await {
            all_items.push(item);
            let mut sorted = all_items.clone();
            sorted.sort_by(|a, b| {
                if a.ty == b.ty { a.name.cmp(&b.name) }
                else if a.ty == ItemType::Folder { std::cmp::Ordering::Less }
                else { std::cmp::Ordering::Greater }
            });
            items_state.set(sorted);
        }
    });
}"#;

    let new_load_dir = r#"fn load_dir(path: String, mut items_state: State<Vec<Item>>) {
    items_state.set(Vec::new());
    
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    std::thread::spawn(move || {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let p = entry.path();
                let is_dir = p.is_dir();
                let name = entry.file_name().to_string_lossy().to_string();
                let size = if !is_dir { entry.metadata().map(|m| m.len()).unwrap_or(0) } else { 0 };
                let _ = tx.send(Item {
                    ty: if is_dir { ItemType::Folder } else { ItemType::File },
                    name,
                    path: p.to_string_lossy().to_string(),
                    size,
                });
            }
        }
    });
    
    freya::prelude::spawn(async move {
        let mut all_items = Vec::new();
        while let Some(item) = rx.recv().await {
            all_items.push(item);
            let mut sorted = all_items.clone();
            sorted.sort_by(|a, b| {
                if a.ty == b.ty { a.name.cmp(&b.name) }
                else if a.ty == ItemType::Folder { std::cmp::Ordering::Less }
                else { std::cmp::Ordering::Greater }
            });
            items_state.set(sorted);
        }
    });
}"#;

    main_rs = main_rs.replace(old_load_dir, new_load_dir);
    
    // Also "URI bar at the top"
    // I can add a URI bar toggle in the top_bar
    let old_top_bar = r#"let top_bar = rect()
        .height(Size::px(80.))
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .cross_align(Alignment::Center)
        .padding(24.)
        .child(
            rect().width(Size::fill()).child(label().font_size(18.).font_weight(FontWeight::BOLD).color(t.text_primary).text(path_str))
        )
        .child(
            rect().horizontal().spacing(16.).child(icon(SEARCH, 20., t.text_secondary))
        )
        .into_element();"#;

    let new_top_bar = r#"let mut tb_uri = uri_input.clone();
    let mut cp_state = current_path.clone();
    let i_state = items.clone();
    let top_bar = rect()
        .height(Size::px(80.))
        .width(Size::fill())
        .direction(Direction::Horizontal)
        .cross_align(Alignment::Center)
        .padding(24.)
        .border(Border::new().width(1.).fill(t.border_subtle))
        .child(
            rect().width(Size::fill())
                .child(
                    TextInput::new(
                        TextInputConfig::default()
                            .value(tb_uri.read().clone())
                            .on_change(move |s| { tb_uri.set(s); })
                            .on_submit({
                                let tb = tb_uri.clone();
                                move || {
                                    let p = tb.read().clone();
                                    cp_state.set(p.clone());
                                    load_dir(p, i_state.clone());
                                }
                            })
                    )
                )
        )
        .child(
            rect().horizontal().spacing(16.).child(icon(SEARCH, 20., t.text_secondary))
        )
        .into_element();"#;

    main_rs = main_rs.replace(old_top_bar, new_top_bar);

    fs::write("apps/files/src/main.rs", main_rs).unwrap();
}
