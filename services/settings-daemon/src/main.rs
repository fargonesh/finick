use {std::time::Duration, tokio::time};

#[tokio::main]
async fn main() {
    println!("Starting Finick Settings Daemon...");
    println!("Initializing Screen Time tracking and Activity Indexing...");

    // Mock tracking loop for active window and screen time
    let mut interval = time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        // In a real implementation, we would query Hyprland for the active window
        // using `hyprctl activewindow -j` and log the time spent per application.
        // We'd also manage the system's indexing processes for Spotlight-like search.
        println!("Tracking screen time... (mock tick)");
    }
}
