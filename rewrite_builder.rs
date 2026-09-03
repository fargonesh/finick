use std::fs;

fn main() {
    let mut main_rs = fs::read_to_string("apps/settings/src/main.rs").unwrap();

    // I want to replace the `match *current_route.read() {` block with:
    // match *current_route.read() {
    //     Route::General => view_General(&t, &state),
    //     ...
    // }
    
    // I also need to define `SettingsState` at the top of the file.
    let settings_state = r#"
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
"#;
    
    // Replace the top of `app()` to include the state initialization
    let app_start = r#"fn app() -> impl IntoElement {
    use_init_app_theme(DARK_THEME);
    let t = use_app_theme();
    let current_route = use_state(|| Route::General);
"#;
    
    let new_app_start = format!(r#"{}
    let state = SettingsState {{
        audio: use_state(|| AudioInfo {{ volume: 50.0, is_muted: false, default_sink_name: "Loading...".to_string() }}),
        displays: use_state(|| Vec::<DisplayInfo>::new()),
        power: use_state(|| PowerInfo {{ capacity: "Unknown".to_string(), status: "Unknown".to_string() }}),
        input: use_state(|| InputDevices {{ mice: vec![], keyboards: vec![] }}),
        users: use_state(|| Vec::<UserAccount>::new()),
        gaps_in: use_state(|| 5.0f64),
        gaps_out: use_state(|| 20.0f64),
        border_size: use_state(|| 1.0f64),
        storage: use_state(|| None::<StorageInfo>),
        bt_devices: use_state(|| Vec::<BluetoothDevice>::new()),
        bt_is_enabled: use_state(|| false),
        bt_is_loading: use_state(|| true),
    }};
    
    let mut loaded = use_state(|| false);
    if !*loaded.read() {{
        loaded.set(true);
        // We spawn background threads to load everything once!
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
        
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || {{
            let backend = HyprlandBackend;
            let _ = tx.send(("audio", Box::new(backend.get_audio_info()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("displays", Box::new(backend.get_displays()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("power", Box::new(backend.get_power_info()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("input", Box::new(backend.get_input_devices()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("storage", Box::new(backend.get_storage_info()) as Box<dyn std::any::Any + Send>));
            let _ = tx.send(("bt", Box::new((backend.get_bluetooth_status(), backend.get_paired_bluetooth_devices())) as Box<dyn std::any::Any + Send>));
            
            let gi = get_hypr_option("general:gaps_in", 5.0);
            let go = get_hypr_option("general:gaps_out", 20.0);
            let bs = get_hypr_option("general:border_size", 1.0);
            let _ = tx.send(("appearance", Box::new((gi, go, bs)) as Box<dyn std::any::Any + Send>));
        }});
        
        tokio::spawn(async move {{
            let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
            if let Ok(content) = tokio::fs::read_to_string("/etc/passwd").await {{
                let _ = tx2.send(parse_passwd_users(&content));
            }}
            freya::prelude::spawn(async move {{
                if let Some(user_list) = rx2.recv().await {{
                    users_state.set(user_list);
                }}
            }});
        }});
        
        freya::prelude::spawn(async move {{
            while let Some((k, v)) = rx.recv().await {{
                match k {{
                    "audio" => if let Ok(info) = v.downcast::<AudioInfo>() {{ audio_state.set(*info); }},
                    "displays" => if let Ok(info) = v.downcast::<Vec<DisplayInfo>>() {{ d_state.set(*info); }},
                    "power" => if let Ok(info) = v.downcast::<PowerInfo>() {{ p_state.set(*info); }},
                    "input" => if let Ok(info) = v.downcast::<InputDevices>() {{ i_state.set(*info); }},
                    "storage" => if let Ok(info) = v.downcast::<StorageInfo>() {{ storage_setter.set(Some(*info)); }},
                    "bt" => if let Ok(info) = v.downcast::<(bool, Vec<BluetoothDevice>)>() {{
                        bt_state.set(info.0);
                        dev_state.set(info.1);
                        loading_state.set(false);
                    }},
                    "appearance" => if let Ok(info) = v.downcast::<(f64, f64, f64)>() {{
                        gi_state.set(info.0);
                        go_state.set(info.1);
                        bs_state.set(info.2);
                    }},
                    _ => {{}}
                }}
            }}
        }});
    }}
"#, app_start);

    // I will write this into a new script `final_build.rs` that reads the original generated files 
    // and assembles the perfect `apps/settings/src/main.rs`.
    fs::write("rewrite.sh", "echo ok").unwrap();
}
