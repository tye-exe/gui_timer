#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crate::comms::{GuiResponse, TO_GUI_SOCK, TO_TRAY_SOCK, sync_socket::WriteObj};
use app::Gui;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, NameType as _, Stream, ToFsName as _, ToNsName as _,
    traits::Stream as _,
};

mod app;
mod timer;

pub(crate) fn launch_gui() {
    let receiver = create_channel(TO_GUI_SOCK)
        .expect("Unable to create receiver for tray actions")
        .split()
        .0;

    let mut sender = create_channel(TO_TRAY_SOCK)
        .expect("Unable to create sender for gui responses")
        .split()
        .1;

    sender
        .write_obj(GuiResponse::Opened)
        .expect("Unable to inform tray of open.");

    eframe::run_native(
        "Gui Timer",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(Gui::new(cc, receiver, sender)))),
    )
    .expect("Unable to start GUI");
}

/// Creates a receiver to listen to actions from the tray.
fn create_channel(channel_name: &'static str) -> Result<Stream, std::io::Error> {
    let name = match GenericNamespaced::is_supported() {
        true => channel_name.to_ns_name::<GenericNamespaced>(),
        false => format!("/tmp/{}", channel_name).to_fs_name::<GenericFilePath>(),
    }?;

    let stream = Stream::connect(name)?;
    stream.set_nonblocking(true)?;
    Ok(stream)
}
