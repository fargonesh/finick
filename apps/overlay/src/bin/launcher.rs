use {
    freya::prelude::*,
    overlay::launcher::{
        check_single_instance_toggle, ensure_launcher_window_rules, initial_query_from_args,
        launcher_window_config_with_query, LAUNCHER_PAGE_SLUGS,
    },
};

fn main() {
    let args = config::LaunchArgs::parse();
    if args.wants_help() {
        args.print_help("launcher", LAUNCHER_PAGE_SLUGS);
        return;
    }
    if check_single_instance_toggle() {
        return;
    }
    ensure_launcher_window_rules();
    let initial_query = initial_query_from_args(&args);
    args.maybe_arm_screenshot("launcher");
    let _rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().ok();
    let _guard = _rt.as_ref().map(|rt| rt.enter());
    launch(LaunchConfig::new().with_window(launcher_window_config_with_query(initial_query)));
}
