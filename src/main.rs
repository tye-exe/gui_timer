#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use app::Gui;
use ksni::TrayMethods;
use tokio_util::sync::CancellationToken;
use tray_icon::TimerTray;

mod app;
mod timer;
mod tray_icon;

fn main() {
    let cancellation_token = CancellationToken::new();

    let spawn = {
        let token = cancellation_token.clone();
        std::thread::spawn(move || {
            let tokio = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Unable to start background tasks");

            tokio.block_on(async {
                TimerTray::default()
                    .spawn()
                    .await
                    .expect("Unable to start taskbar tray.");

                tokio::select! {
                    _ = token.cancelled() => {}
                }
            });
        })
    };

    eframe::run_native(
        "Gui Timer",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(Gui::new(cc)))),
    )
    .expect("Unable to start GUI");

    cancellation_token.cancel();

    spawn.join().expect("Unable to join thread");
}
