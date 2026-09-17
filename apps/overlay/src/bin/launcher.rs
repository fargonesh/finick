use {freya::prelude::*, overlay::launcher::launcher_window_config};

fn main() { launch(LaunchConfig::new().with_window(launcher_window_config())); }
