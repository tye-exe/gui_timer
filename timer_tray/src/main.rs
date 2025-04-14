#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::LazyLock;

use ksni::TrayMethods;
use tokio_util::sync::CancellationToken;
use tray_icon::TimerTray;

mod tray_icon;

/// The [`CancellationToken`] that is responsible for shutting down the entire application when it is cancelled.
static GLOBAL_CANCEL: LazyLock<CancellationToken> = LazyLock::new(|| CancellationToken::new());

#[tokio::main]
async fn main() {
    let handle = TimerTray
        .spawn()
        .await
        .expect("Unable to start taskbar tray.");

    GLOBAL_CANCEL.cancelled().await;
    handle.shutdown().await;
}
