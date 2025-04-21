#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use app::Gui;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, NameType as _, Stream, ToFsName as _, ToNsName as _,
    traits::{RecvHalf, Stream as _},
};

mod app;
mod timer;

const RECEIVE: &str = "timer_tray_to_gui.sock";
const SEND: &str = "timer_gui_to_tray.sock";

fn main() {
    let receiver = create_receiver()
        .inspect_err(|e| eprintln!("Error creating receiver for tray actions: {e}"))
        .ok();

    eframe::run_native(
        "Gui Timer",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(Gui::new(cc, receiver)))),
    )
    .expect("Unable to start GUI");
}

/// Creates a receiver to listen to actions from the tray.
fn create_receiver() -> Result<impl RecvHalf, std::io::Error> {
    let name = match GenericNamespaced::is_supported() {
        true => RECEIVE.to_ns_name::<GenericNamespaced>(),
        false => format!("/tmp/{}", RECEIVE).to_fs_name::<GenericFilePath>(),
    }
    .expect("Unable to start IPC");

    let stream = Stream::connect(name)?;
    stream.set_nonblocking(true)?;
    Ok(stream.split().0)
}
