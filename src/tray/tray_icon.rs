use crate::{
    comms::{GuiAction, GuiResponse},
    until_global_cancel,
};
use image::GenericImageView;
use ksni::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::{GLOBAL_CANCEL, GuiState, spawn_gui};

pub(crate) struct TimerTray {
    sender: UnboundedSender<GuiAction>,

    state: GuiState,
}

impl TimerTray {
    pub(crate) fn new(sender: UnboundedSender<GuiAction>) -> Self {
        Self {
            sender,
            state: GuiState::OpenRequested,
        }
    }

    /// Opens or closes the GUI depending on the current state.
    fn toggle_gui(&mut self) {
        match self.state {
            GuiState::Opened => {
                if let Err(err) = self.sender.send(GuiAction::Close) {
                    log::error!("Internal tray communication was closed unexpectedly: {err}");
                    GLOBAL_CANCEL.cancel();
                }

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
        if let Err(err) = self.sender.send(GuiAction::Quit) {
            log::error!("Internal tray communication was closed unexpectedly: {err}")
        }
        GLOBAL_CANCEL.cancel();
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

pub(crate) async fn update_tray(
    handle: Handle<TimerTray>,
    mut rx_from_gui: UnboundedReceiver<GuiResponse>,
) {
    loop {
        let response = match until_global_cancel!(rx_from_gui.recv()) {
            Some(response) => response,
            None => {
                log::error!("Internal tray communication was closed unexpectedly");
                GLOBAL_CANCEL.cancel();
                break;
            }
        };

        until_global_cancel!(handle.update(|tray| {
            tray.state = match response {
                GuiResponse::Closed => GuiState::Closed,
                GuiResponse::Opened => GuiState::Opened,
            };
        }));

        log::debug!("Tray tick loop.");
    }
}
