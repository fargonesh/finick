use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/settings/src/main.rs").unwrap();

    let old_app_start = r#"fn app() -> impl IntoElement {
    use_init_app_theme(DARK_THEME);
    let t = use_app_theme();
    let current_route = use_state(|| Route::General);"#;

    let new_app_start = r#"
#[derive(Clone)]
struct SettingsState {
    audio: State<AudioInfo>,
    displays: State<Vec<DisplayInfo>>,
    power: State<PowerInfo>,
    input: State<InputDevices>,
    users: State<Vec<UserAccount>>,
    gaps_in: State<f64>,
    gaps_out: State<f64>,
    border_size: State<f64>,
    storage: State<Option<StorageInfo>>,
    bt_devices: State<Vec<BluetoothDevice>>,
    bt_is_enabled: State<bool>,
    bt_is_loading: State<bool>,
}

fn app() -> impl IntoElement {
    use_init_app_theme(DARK_THEME);
    let t = use_app_theme();
    let current_route = use_state(|| Route::General);
    
    let state = SettingsState {
        audio: use_state(|| AudioInfo { volume: 50.0, is_muted: false, default_sink_name: "Loading...".to_string() }),
        displays: use_state(|| Vec::<DisplayInfo>::new()),
        power: use_state(|| PowerInfo { capacity: "Unknown".to_string(), status: "Unknown".to_string() }),
        input: use_state(|| InputDevices { mice: vec![], keyboards: vec![] }),
        users: use_state(|| Vec::<UserAccount>::new()),
        gaps_in: use_state(|| 5.0f64),
        gaps_out: use_state(|| 20.0f64),
        border_size: use_state(|| 1.0f64),
        storage: use_state(|| None::<StorageInfo>),
        bt_devices: use_state(|| Vec::<BluetoothDevice>::new()),
        bt_is_enabled: use_state(|| false),
        bt_is_loading: use_state(|| true),
    };
    
    let mut loaded = use_state(|| false);
    if !*loaded.read() {
        loaded.set(true);
        let mut audio_state = state.audio.clone();
        let mut d_state = state.displays.clone();
        let mut p_state = state.power.clone();
        let mut i_state = state.input.clone();
        let mut users_state = state.users.clone();
        let mut gi_state = state.gaps_in.clone();
        let mut go_state = state.gaps_out.clone();
        let mut bs_state = state.border_size.clone();
        let mut storage_setter = state.storage.clone();
        let mut dev_state = state.bt_devices.clone();
        let mut bt_state = state.bt_is_enabled.clone();
        let mut loading_state = state.bt_is_loading.clone();
        
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(&'static str, Box<dyn std::any::Any + Send>)>();
        std::thread::spawn(move || {
            let backend = HyprlandBackend;
            let _ = tx.send(("audio", Box::new(backend.get_audio_info()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("displays", Box::new(backend.get_displays()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("power", Box::new(backend.get_power_info()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("input", Box::new(backend.get_input_devices()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("storage", Box::new(backend.get_storage_info()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("bt", Box::new((backend.get_bluetooth_status(), backend.get_paired_bluetooth_devices())) as Box<dyn std::any::Any + Send>));
            
            // let gi = get_hypr_option("general:gaps_in", 5.0);
            // let go = get_hypr_option("general:gaps_out", 20.0);
            // let bs = get_hypr_option("general:border_size", 1.0);
            // let _ = tx.send(("appearance", Box::new((gi, go, bs)) as Box<dyn std::any::Any + Send>));
        });
        
        tokio::spawn(async move {
            let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
            if let Ok(content) = tokio::fs::read_to_string("/etc/passwd").await {
                let _ = tx2.send(parse_passwd_users(&content));
            }
            freya::prelude::spawn(async move {
                if let Some(user_list) = rx2.recv().await {
                    users_state.set(user_list);
                }
            });
        });
        
        freya::prelude::spawn(async move {
            while let Some((k, v)) = rx.recv().await {
                match k {
                    "audio" => if let Ok(info) = v.downcast::<AudioInfo>() { audio_state.set(*info); },
                    "displays" => if let Ok(info) = v.downcast::<Vec<DisplayInfo>>() { d_state.set(*info); },
                    "power" => if let Ok(info) = v.downcast::<PowerInfo>() { p_state.set(*info); },
                    "input" => if let Ok(info) = v.downcast::<InputDevices>() { i_state.set(*info); },
                    "storage" => if let Ok(info) = v.downcast::<StorageInfo>() { storage_setter.set(Some(*info)); },
                    "bt" => if let Ok(info) = v.downcast::<(bool, Vec<BluetoothDevice>)>() {
                        bt_state.set(info.0);
                        dev_state.set(info.1);
                        loading_state.set(false);
                    },
                    "appearance" => if let Ok(info) = v.downcast::<(f64, f64, f64)>() {
                        gi_state.set(info.0);
                        go_state.set(info.1);
                        bs_state.set(info.2);
                    },
                    _ => {}
                }
            }
        });
    }
"#;

    // We will just do the same script approach as hoist_hooks.rs, but robustly!
    let mut code = fs::read_to_string("apps/settings/src/main.rs").unwrap();
    code = code.replace(old_app_start, new_app_start);
    
    // Replace struct implementations
    let names = ["General", "Network", "Sound", "Displays", "Power", "Input", "About", "QuickActions", "Storage", "Bluetooth", "Accounts", "Appearance"];
    
    for name in names.iter() {
        let pattern = format!("#[derive(PartialEq)]\nstruct {};\nimpl Component for {} {{\n    fn render(&self) -> impl IntoElement {{", name, name);
        let replacement = format!("fn view_{}(t: &AppTheme, state: &SettingsState) -> Element {{", name);
        code = code.replace(&pattern, &replacement);
        
        let pattern2 = format!("impl Component for {} {{\n    fn render(&self) -> impl IntoElement {{", name);
        let replacement2 = format!("fn view_{}(t: &AppTheme, state: &SettingsState) -> Element {{", name);
        code = code.replace(&pattern2, &replacement2);
        
        let p3 = format!("Route::{} => {}.render().into_element(),", name, name);
        let r3 = format!("Route::{} => view_{}(&t, &state).into_element(),", name, name);
        code = code.replace(&p3, &r3);
        
        let p4 = format!("Route::{} => {}.render().into_element()", name, name);
        let r4 = format!("Route::{} => view_{}(&t, &state).into_element()", name, name);
        code = code.replace(&p4, &r4);
    }
    
    // Remove individual `let t = use_app_theme();`
    code = code.replace("let t = use_app_theme();", "");
    code = code.replace("let mut theme_state = use_app_theme_state();", "");
    code = code.replace("let is_dark = theme_state.read().mode == ThemeMode::Dark;", "let is_dark = true;");
    
    // Replace state definitions with state.xxx.clone()
    code = code.replace("let mut audio =
            use_state(|| AudioInfo { volume: 50.0, is_muted: false, default_sink_name: \"Loading...\".to_string() });", "let mut audio = state.audio.clone();");
            
    code = code.replace("let mut displays = use_state(|| Vec::<DisplayInfo>::new());", "let mut displays = state.displays.clone();");
    code = code.replace("let mut power = use_state(|| PowerInfo { capacity: \"Unknown\".to_string(), status: \"Unknown\".to_string() });", "let mut power = state.power.clone();");
    code = code.replace("let mut input = use_state(|| InputDevices { mice: vec![], keyboards: vec![] });", "let mut input = state.input.clone();");
    code = code.replace("let mut users = use_state(|| Vec::<UserAccount>::new());", "let mut users = state.users.clone();");
    code = code.replace("let mut gaps_in = use_state(|| 5.0f64);", "let mut gaps_in = state.gaps_in.clone();");
    code = code.replace("let mut gaps_out = use_state(|| 20.0f64);", "let mut gaps_out = state.gaps_out.clone();");
    code = code.replace("let mut border_size = use_state(|| 1.0f64);", "let mut border_size = state.border_size.clone();");
    code = code.replace("let mut storage_data = use_state(|| None::<StorageInfo>);", "let mut storage_data = state.storage.clone();");
    code = code.replace("let mut devices = use_state(|| Vec::<BluetoothDevice>::new());", "let mut devices = state.bt_devices.clone();");
    code = code.replace("let mut is_enabled = use_state(|| false);", "let mut is_enabled = state.bt_is_enabled.clone();");
    code = code.replace("let mut is_loading = use_state(|| true);", "let mut is_loading = state.bt_is_loading.clone();");
    
    // Now we must REMOVE the loading blocks from EACH view since it's hoisted to app()
    // It's easier to just regex it out, but I'll write a manual removal for the specific patterns!
    let load_audio = r#"let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut audio_state = audio.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let _ = tx.send(HyprlandBackend.get_audio_info());
            });
            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    audio_state.set(info);
                }
            });
        }"#;
    code = code.replace(load_audio, "");
    
    let load_displays = r#"let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut d_state = displays.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let _ = tx.send(HyprlandBackend.get_displays());
            });
            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    d_state.set(info);
                }
            });
        }"#;
    code = code.replace(load_displays, "");
    
    let load_power = r#"let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut p_state = power.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let _ = tx.send(HyprlandBackend.get_power_info());
            });
            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    p_state.set(info);
                }
            });
        }"#;
    code = code.replace(load_power, "");
    
    let load_input = r#"let mut loaded = use_state(|| false);
        if !*loaded.read() {
            loaded.set(true);
            let mut i_state = input.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let _ = tx.send(HyprlandBackend.get_input_devices());
            });
            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    i_state.set(info);
                }
            });
        }"#;
    code = code.replace(load_input, "");

    let load_users = r#"let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut users_state = users.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<UserAccount>>();

            tokio::spawn(async move {
                if let Ok(content) = tokio::fs::read_to_string("/etc/passwd").await {
                    let parsed = parse_passwd_users(&content);
                    let _ = tx.send(parsed);
                }
            });

            freya::prelude::spawn(async move {
                if let Some(user_list) = rx.recv().await {
                    users_state.set(user_list);
                }
            });
        }"#;
    code = code.replace(load_users, "");

    let load_appearance = r#"let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut gi_state = gaps_in.clone();
            let mut go_state = gaps_out.clone();
            let mut bs_state = border_size.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            std::thread::spawn(move || {
                let gi = get_hypr_option("general:gaps_in", 5.0);
                let go = get_hypr_option("general:gaps_out", 20.0);
                let bs = get_hypr_option("general:border_size", 1.0);
                let _ = tx.send((gi, go, bs));
            });
            freya::prelude::spawn(async move {
                if let Some((gi, go, bs)) = rx.recv().await {
                    gi_state.set(gi);
                    go_state.set(go);
                    bs_state.set(bs);
                }
            });
        }"#;
    code = code.replace(load_appearance, "");
    
    let load_storage = r#"let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut storage_setter = storage_data.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let backend = HyprlandBackend;
                let info = backend.get_storage_info();
                let _ = tx.send(info);
            });

            freya::prelude::spawn(async move {
                if let Some(info) = rx.recv().await {
                    storage_setter.set(Some(info));
                }
            });
        }"#;
    code = code.replace(load_storage, "");

    let load_bt = r#"let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut dev_state = devices.clone();
            let mut bt_state = is_enabled.clone();
            let mut loading_state = is_loading.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let backend = HyprlandBackend;
                let status = backend.get_bluetooth_status();
                let devs = backend.get_paired_bluetooth_devices();
                let _ = tx.send((status, devs));
            });

            freya::prelude::spawn(async move {
                if let Some((status, devs)) = rx.recv().await {
                    bt_state.set(status);
                    dev_state.set(devs);
                    loading_state.set(false);
                }
            });
        }"#;
    code = code.replace(load_bt, "");

    fs::write("apps/settings/src/main.rs", code).unwrap();
}
