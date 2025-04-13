#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{sync::LazyLock, time::Duration};

use app::Gui;
use ksni::TrayMethods;
use tokio::{runtime::Runtime, sync::mpsc::UnboundedReceiver, time};
use tokio_util::sync::CancellationToken;
use tray_icon::TimerTray;

mod app;
mod timer;
mod tray_icon;

/// The [`CancellationToken`] that is responsible for shutting down the entire application when it is cancelled.
static GLOBAL_CANCEL: LazyLock<CancellationToken> = LazyLock::new(|| CancellationToken::new());

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Unable to start background tasks");

    let (ctx_send, ctx_receiver) = tokio::sync::mpsc::unbounded_channel();

    spawn_tray(&runtime);
    gui_management(&runtime, ctx_receiver);

    eframe::run_native(
        "Gui Timer",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            ctx_send
                .send(cc.egui_ctx.clone())
                .expect("Unable to send GUI context to backend");

            Ok(Box::new(Gui::new(cc)))
        }),
    )
    .expect("Unable to start GUI");
}

/// Ensures that the currently open GUI is updated and remains synchronised with the backend.
fn gui_management(runtime: &Runtime, mut ctx_receiver: UnboundedReceiver<egui::Context>) {
    runtime.spawn(async move {
        let Some(ctx): Option<egui::Context> = GLOBAL_CANCEL
            .run_until_cancelled(ctx_receiver.recv())
            .await
            .flatten()
        else {
            return;
        };

        close_gui_when_cancel(ctx);
    });
}

/// Closes the GUI when [`GLOBAL_CANCEL`] is cancelled.
fn close_gui_when_cancel(ctx: egui::Context) {
    tokio::spawn(async move {
        GLOBAL_CANCEL.cancelled().await;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    });
}

/// Displays the [`TimerTray`] in the desktop environment's tray.
fn spawn_tray(runtime: &Runtime) {
    runtime.spawn(async move {
        let handle = TimerTray
            .spawn()
            .await
            .expect("Unable to start taskbar tray.");

        GLOBAL_CANCEL.cancelled().await;
        handle.shutdown().await;
    });
}
