#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::net::TcpStream;

use crate::comms::{GuiResponse, SOCKET_ADDR, sync_socket::WriteObj};
use app::Gui;

mod app;
mod timer;

pub(crate) fn launch_gui() {
    let mut connection = TcpStream::connect(SOCKET_ADDR)
        .expect(&format!("Unable to connect to tray on {SOCKET_ADDR}"));
    connection
        .set_nonblocking(true)
        .expect("Unable to set tcp connection to non-blocking");

    connection
        .write_obj(GuiResponse::Opened)
        .expect("Unable to inform tray of GUI open");

    eframe::run_native(
        "Gui Timer",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(Gui::new(cc, connection)))),
    )
    .expect("Unable to start GUI");
}
