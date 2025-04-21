#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::LazyLock;

use comms::GuiAction;
use comms::async_socket::AsyncWriteObj;
use interprocess::local_socket::traits::tokio::Listener;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, NameType, ToFsName, ToNsName,
};

use ksni::TrayMethods;
use tokio::sync::mpsc::{self, Receiver};
use tokio::task;
use tokio_util::sync::CancellationToken;
use tray_icon::TimerTray;

mod tray_icon;

const SEND: &str = "timer_tray_to_gui.sock";
const RECEIVE: &str = "timer_gui_to_tray.sock";

/// The [`CancellationToken`] that is responsible for shutting down the entire application when it is cancelled.
static GLOBAL_CANCEL: LazyLock<CancellationToken> = LazyLock::new(|| CancellationToken::new());

#[tokio::main]
async fn main() {
    let (sender, receiver) = mpsc::channel(4);

    gui_comms(receiver);
    spawn_gui();

    let handle = TimerTray::new(sender)
        .spawn()
        .await
        .expect("Unable to start taskbar tray.");

    GLOBAL_CANCEL.cancelled().await;
    handle.shutdown().await;
}

/// Spawns a new task that forwards data to the current GUI.
fn gui_comms(mut receiver: Receiver<GuiAction>) {
    task::spawn(async move {
        let listener = create_listener(SEND);
        loop {
            let mut stream = until_global_cancel!(listener.accept())
                .expect("Unable to listen for gui communication.");

            while let Some(action) = until_global_cancel!(receiver.recv()) {
                if action == GuiAction::Close {
                    stream.write_obj::<GuiAction>(action).await.unwrap();
                    break;
                }
                stream.write_obj::<GuiAction>(action).await.unwrap();
            }
        }
    });
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
    Closed,
    Open,
}

impl GuiState {
    fn opposite(self) -> Self {
        match self {
            GuiState::Closed => Self::Open,
            GuiState::Open => Self::Closed,
        }
    }
}

/// Creates a new gui.
fn spawn_gui() {
    let Ok(mut exe_path) = std::env::current_exe() else {
        return;
    };
    exe_path.pop();
    exe_path.push("gui_timer");
    std::process::Command::new(exe_path).spawn().unwrap();
}

/// Runs the given future until [`GLOBAL_CANCEL`] is cancelled.
#[macro_export]
macro_rules! until_global_cancel {
    ($future:expr) => {
        (match crate::GLOBAL_CANCEL.run_until_cancelled($future).await {
            Some(var) => var,
            None => return,
        })
    };
}
