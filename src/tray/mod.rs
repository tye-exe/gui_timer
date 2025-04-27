#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crate::until_global_cancel;
use std::sync::LazyLock;

use crate::comms::async_socket::{AsyncReadObj, AsyncWriteObj};
use crate::comms::{GuiAction, GuiResponse, TO_GUI_SOCK, TO_TRAY_SOCK};
use interprocess::local_socket::traits::tokio::Listener;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, NameType, ToFsName, ToNsName,
};

use ksni::TrayMethods;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_util::sync::CancellationToken;
use tray_icon::{TimerTray, update_tray};

mod tray_icon;

/// The [`CancellationToken`] that is responsible for shutting down the entire application when it is cancelled.
static GLOBAL_CANCEL: LazyLock<CancellationToken> = LazyLock::new(|| CancellationToken::new());

fn launch_tray() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (tx_to_gui, rx_to_gui) = mpsc::channel(4);
            let (tx_from_gui, rx_from_gui) = mpsc::channel(4);

            tokio::spawn(gui_send(rx_to_gui));
            tokio::spawn(gui_receive(tx_from_gui));
            spawn_gui();

            let handle = TimerTray::new(tx_to_gui)
                .spawn()
                .await
                .expect("Unable to start taskbar tray.");

            tokio::spawn(update_tray(handle.clone(), rx_from_gui));

            GLOBAL_CANCEL.cancelled().await;
            handle.shutdown().await;
        });
}

/// Forwards actions to the GUI.
async fn gui_send(mut receiver: Receiver<GuiAction>) {
    let listener = create_listener(TO_GUI_SOCK);
    loop {
        let mut stream = until_global_cancel!(listener.accept())
            .expect("Unable to listen for GUI communication.");

        while let Some(action) = until_global_cancel!(receiver.recv()) {
            let stop = matches!(action, GuiAction::Close);

            log::debug!("Sent Action: {action:?}");

            let _ = stream
                .write_obj::<GuiAction>(action)
                .await
                .inspect_err(|e| log::error!("The GUI was closed unexpectedly? {e}"));

            if stop {
                break;
            }
        }
    }
}

/// Forwards responses from the GUI.
async fn gui_receive(sender: Sender<GuiResponse>) {
    let listener = create_listener(TO_TRAY_SOCK);

    loop {
        let mut stream = until_global_cancel!(listener.accept())
            .expect("Unable to listen for gui communication.");

        loop {
            let response = match until_global_cancel!(stream.read_obj()) {
                Ok(response) => response,
                Err(err) => {
                    log::error!("Error reading GUI response: {err}");
                    break;
                }
            };

            let stop = matches!(response, GuiResponse::Closed);

            log::debug!("Received Response: {response:?}");

            until_global_cancel!(sender.send(response)).expect("Unable to listen for GUI response");

            if stop {
                break;
            }
        }
    }
}

/// Creates a listener that listens for connections from the GUI.
fn create_listener(name: &'static str) -> interprocess::local_socket::tokio::Listener {
    let name = match GenericNamespaced::is_supported() {
        true => name.to_ns_name::<GenericNamespaced>(),
        false => format!("/tmp/{}", name).to_fs_name::<GenericFilePath>(),
    }
    .expect("Unable to start IPC");

    ListenerOptions::new()
        .name(name.clone())
        .create_tokio()
        .expect("Unable to listen for gui communication.")
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
    let Ok(mut exe_path) = std::env::current_exe() else {
        return;
    };
    exe_path.pop();
    exe_path.push("gui");
    std::process::Command::new(exe_path).spawn().unwrap();
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
