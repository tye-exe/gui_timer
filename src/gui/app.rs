use std::{
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    time::Duration,
};

use egui::Widget;
use serde::{Deserialize, Serialize};

use crate::{
    comms::{
        GuiAction, GuiResponse,
        sync_socket::{ReadError, ReadObj as _, WriteObj as _},
    },
    gui::timer::{Timer, TimerData},
};

/// The key that persistent data is saved at.
const APP_KEY: &str = "GUI_TIMER";

pub(crate) struct Gui {
    /// The connection to the tray.
    connection: TcpStream,
    /// Whether the GUI is in the process of closing.
    is_closing: bool,

    /// Persistent GUI data.
    persistent: Persistent,
}

impl Gui {
    pub fn new(cc: &eframe::CreationContext<'_>, connection: TcpStream) -> Self {
        let mut persistent: Persistent = cc
            .storage
            .map(|storage| eframe::get_value(storage, APP_KEY))
            .flatten()
            .unwrap_or_default();

        // TODO(tye): remove after debugging.
        persistent.timer_data = vec![TimerData::new(Duration::from_secs(3))];

        Self {
            connection,
            is_closing: false,
            persistent,
        }
    }

    /// Reads the action from the tray if there is one.
    fn read_action(&mut self) -> Option<GuiAction> {
        // Otherwise there is an error trying to read from the connection.
        if self.is_closing {
            return None;
        }

        self.connection
            .read_obj::<GuiAction>()
            .inspect_err(|err| match err {
                ReadError::Read(error) if error.kind() != ErrorKind::WouldBlock => {
                    log::error!("Failed to parse message from tray :{err}")
                }
                _ => {}
            })
            .ok()
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            for timer_data in self.persistent.timer_data.iter_mut() {
                Timer::new(timer_data).radius(50.0).ui(ui);
            }

            if ui.button("Reset").clicked() {
                let _ = self.persistent.timer_data.pop();
                self.persistent
                    .timer_data
                    .push(TimerData::new(Duration::from_secs(5)));
            }
        });

        // Ensure the GUI still updates when the user is not interacting with it.
        ctx.request_repaint_after(Duration::from_millis(250));

        // Execute on any sent actions.
        if let Some(action) = self.read_action() {
            log::debug!("Gui Received : {action:?}");

            match action {
                GuiAction::Close => {
                    self.is_closing = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                }
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_KEY, &self.persistent);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self
            .connection
            .write_obj(GuiResponse::Closed)
            .inspect_err(|err| log::error!("Unable to inform tray of GUI close: {err}"));

        log::debug!("Gui Sent : {:?}", GuiResponse::Closed);
    }
}

#[derive(Deserialize, Serialize, Default)]
struct Persistent {
    timer_data: Vec<TimerData>,
}
