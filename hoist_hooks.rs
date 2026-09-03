use std::fs;

fn main() {
    let mut code = fs::read_to_string("apps/settings/src/main.rs").unwrap();
    
    // Replace structs with functions
    let names = ["General", "Network", "Sound", "Displays", "Power", "Input", "About", "QuickActions", "Storage", "Bluetooth", "Accounts", "Appearance"];
    
    for name in names.iter() {
        let pattern = format!("#[derive(PartialEq)]\nstruct {};\nimpl Component for {} {{\n    fn render(&self) -> impl IntoElement {{", name, name);
        let replacement = format!("fn view_{}(t: &AppTheme, state: &SettingsState) -> Element {{", name);
        code = code.replace(&pattern, &replacement);
        
        let pattern2 = format!("impl Component for {} {{\n    fn render(&self) -> impl IntoElement {{", name);
        let replacement2 = format!("fn view_{}(t: &AppTheme, state: &SettingsState) -> Element {{", name);
        code = code.replace(&pattern2, &replacement2);
    }
    
    // Now remove `let t = use_app_theme();` inside these functions
    // We'll just leave it and let it shadow, but it has hooks! So we MUST remove it.
    code = code.replace("let t = use_app_theme();", "");
    code = code.replace("let mut theme_state = use_app_theme_state();", "");
    
    // Replace state references with state.x
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
    
    // Remove the `let mut loaded = use_state(|| false);` and the `if !*loaded.read() { ... }` block
    // We will do this manually by replacing the `loaded` blocks with empty strings for each view.
    // Instead of doing regex, we can just replace the specific loaded blocks.
    // Actually, I can just build `SettingsState` at the top of `app()` and let the tokio threads spawn inside `app()`.
    
    fs::write("apps/settings/src/main.rs.new", code).unwrap();
}
