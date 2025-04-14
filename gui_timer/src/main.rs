#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use app::Gui;

mod app;
mod timer;

fn main() {
    eframe::run_native(
        "Gui Timer",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(Gui::new(cc)))),
    )
    .expect("Unable to start GUI");
}
