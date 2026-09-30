use {
    freya::prelude::*,
    overlay::launcher::{check_single_instance_toggle, launcher_window_config},
};

fn main() {
    if check_single_instance_toggle() {
        return;
    }
    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());
    launch(LaunchConfig::new().with_window(launcher_window_config()));
}
