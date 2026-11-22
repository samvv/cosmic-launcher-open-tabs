//! One binary, two roles:
//!
//! - **Launcher plugin** (default): spawned by pop-launcher / COSMIC Launcher.
//! - **Native messaging host**: spawned by Chromium-based browsers, which pass
//!   the calling extension's origin (`chrome-extension://<id>/`) as the first
//!   argument. `--native-host` forces this mode for manual testing.

mod host;
mod plugin;
mod tabs;

fn main() {
    let first = std::env::args().nth(1).unwrap_or_default();
    if first == "--native-host" || first.starts_with("chrome-extension://") {
        if let Err(why) = host::run() {
            eprintln!("browser-tabs host: {why}");
            std::process::exit(1);
        }
    } else {
        plugin::run();
    }
}
