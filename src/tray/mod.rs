#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use comms::gui_communication;
use ksni::TrayMethods;
use std::sync::LazyLock;
use tokio::sync::mpsc::{self};
use tokio_util::sync::CancellationToken;
use tray_icon::{TimerTray, update_tray};

mod comms;
mod tray_icon;

/// The [`CancellationToken`] that is responsible for shutting down the entire application when it is cancelled.
static GLOBAL_CANCEL: LazyLock<CancellationToken> = LazyLock::new(|| CancellationToken::new());

pub(crate) fn launch_tray() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(start());
}

async fn start() {
    let (tx_to_gui, rx_to_gui) = mpsc::channel(4);
    let (tx_from_gui, rx_from_gui) = mpsc::channel(4);

    gui_communication(tx_from_gui, rx_to_gui);
    spawn_gui();

    let handle = TimerTray::new(tx_to_gui)
        .spawn()
        .await
        .expect("Unable to start taskbar tray.");

    tokio::spawn(update_tray(handle.clone(), rx_from_gui));

    GLOBAL_CANCEL.cancelled().await;
    handle.shutdown().await;
}

/// The state of the GUI.
#[derive(Clone, Copy, PartialEq)]
enum GuiState {
    OpenRequested,
    CloseRequested,
    Opened,
    Closed,
}

/// Creates a new gui.
fn spawn_gui() {
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    std::process::Command::new(exe_path)
        .arg("--gui")
        .spawn()
        .unwrap();
}

/// Runs the given future until [`GLOBAL_CANCEL`] is cancelled.
#[macro_export]
macro_rules! until_global_cancel {
    ($future:expr) => {
        (match crate::tray::GLOBAL_CANCEL
            .run_until_cancelled($future)
            .await
        {
            Some(var) => var,
            None => return,
        })
    };
}
