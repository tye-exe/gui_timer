use std::{io::ErrorKind, time::Duration};

use comms::{
    GuiAction,
    sync_socket::{ReadError, ReadObj as _},
};
use egui::Widget;
use interprocess::local_socket::traits::RecvHalf;
use serde::{Deserialize, Serialize};

use crate::timer::Timer;

const APP_KEY: &str = "GUI_TIMER";

pub(crate) struct Gui<Receiver: RecvHalf> {
    progress: f32,
    receiver: Option<Receiver>,

    persistent: Persistent,
}

impl<Receiver: RecvHalf> Gui<Receiver> {
    pub fn new(cc: &eframe::CreationContext<'_>, receiver: Option<Receiver>) -> Self {
        let persistent = cc
            .storage
            .map(|storage| eframe::get_value(storage, APP_KEY))
            .flatten()
            .unwrap_or_default();

        Self {
            progress: 0.0,
            receiver,
            persistent,
        }
    }

    /// Reads the action from the tray if there is one.
    fn read_action(&mut self) -> Option<GuiAction> {
        let Some(ref mut receiver) = self.receiver else {
            return None;
        };

        receiver
            .read_obj::<GuiAction>()
            .inspect_err(|err| match err {
                ReadError::Read(error) if error.kind() == ErrorKind::WouldBlock => {}
                _ => {
                    eprintln!("Failed to parse message from tray : {err}")
                }
            })
            .ok()
    }
}

impl<Receiver: RecvHalf> eframe::App for Gui<Receiver> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Timer::default().radius(50.0).progress(self.progress).ui(ui);
        });

        // Dummy code to simulate an active timer.
        self.progress += 0.1;
        if self.progress > 1.0 {
            self.progress -= 1.0;
        }

        // Ensure the GUI still updates when the user is not interacting with it.
        ctx.request_repaint_after(Duration::from_millis(250));

        // Execute on any sent actions.
        if let Some(action) = self.read_action() {
            match action {
                GuiAction::Close => {
                    self.receiver = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                }
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_KEY, &self.persistent);
    }
}

// For persistent data
// TODO(tye): finish later
#[derive(Deserialize, Serialize, Default)]
struct Persistent {}
