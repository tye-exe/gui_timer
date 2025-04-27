use crate::{
    comms::{GuiAction, GuiResponse},
    until_global_cancel,
};
use image::GenericImageView;
use ksni::Handle;
use tokio::sync::mpsc::{Receiver, Sender};

use super::{GLOBAL_CANCEL, GuiState, spawn_gui};

pub(crate) struct TimerTray {
    sender: Sender<GuiAction>,

    state: GuiState,
}

impl TimerTray {
    pub(crate) fn new(sender: Sender<GuiAction>) -> Self {
        Self {
            sender,
            state: GuiState::OpenRequested,
        }
    }

    /// Opens or closes the GUI depending on the current state.
    fn toggle_gui(&mut self) {
        match self.state {
            GuiState::Opened => {
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    until_global_cancel!(sender.send(GuiAction::Close))
                        .expect("Cannot send action to GUI.");
                });
                self.state = GuiState::CloseRequested;
            }
            GuiState::Closed => {
                spawn_gui();
                self.state = GuiState::OpenRequested;
            }
            GuiState::OpenRequested | GuiState::CloseRequested => {}
        }
    }

    /// Quits the Gui and the tray.
    fn quit(&mut self) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _ = until_global_cancel!(sender.send(GuiAction::Close)).inspect_err(|e| {
                log::error!("Internal tray communication was closed unexpectedly: {e}")
            });
            GLOBAL_CANCEL.cancel();
        });
    }
}

impl ksni::Tray for TimerTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            CheckmarkItem {
                label: "Gui".into(),
                enabled: match self.state {
                    GuiState::OpenRequested => false,
                    GuiState::CloseRequested => false,
                    GuiState::Opened => true,
                    GuiState::Closed => true,
                },
                checked: match self.state {
                    GuiState::Closed => false,
                    GuiState::Opened => true,
                    GuiState::OpenRequested => true,
                    GuiState::CloseRequested => false,
                },
                activate: Box::new(Self::toggle_gui),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(Self::quit),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let img = image::load_from_memory_with_format(
            include_bytes!("icon.png"),
            image::ImageFormat::Png,
        )
        .expect("'icon.png' is not a valid file");

        let (width, height) = img.dimensions();
        let mut data = img.into_rgba8().into_vec();

        assert_eq!(data.len() % 4, 0);

        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1) // rgba to argb
        }

        vec![ksni::Icon {
            width: width as i32,
            height: height as i32,
            data,
        }]
    }
}

pub(crate) async fn update_tray(handle: Handle<TimerTray>, mut rx_from_gui: Receiver<GuiResponse>) {
    loop {
        let response =
            until_global_cancel!(rx_from_gui.recv()).expect("Unable to listen for GUI response.");

        until_global_cancel!(handle.update(|tray| {
            tray.state = match response {
                GuiResponse::Closed => GuiState::Closed,
                GuiResponse::Opened => GuiState::Opened,
            };
        }));

        log::debug!("Tray tick loop.");
    }
}
