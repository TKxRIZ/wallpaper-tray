mod config;
mod service;
mod tray;
mod updater;
mod wallpaper;

use ksni::blocking::TrayMethods;

fn main() {
    let tray   = tray::WallpaperTray::new();
    let handle = tray.spawn().expect("Tray konnte nicht gestartet werden");

    let update_handle = handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let cfg = config::load();

        // Write full state (tray + picker) for the GUI to read
        updater::check_and_write_state(&cfg);

        // Update tray icon/menu if tray itself has an update
        if let Some(version) = updater::check_update(&cfg) {
            let _ = update_handle.update(|tray| {
                tray.update_available = Some(version);
            });
        }
    });

    std::thread::park();
}
