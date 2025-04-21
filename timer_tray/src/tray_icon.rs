use comms::GuiAction;
use image::GenericImageView;
use tokio::sync::mpsc::Sender;

use crate::{GLOBAL_CANCEL, GuiState, spawn_gui, until_global_cancel};

pub(crate) struct TimerTray {
    sender: Sender<GuiAction>,

    state: GuiState,
}

impl TimerTray {
    pub(crate) fn new(sender: Sender<GuiAction>) -> Self {
        Self {
            sender,
            state: GuiState::Open,
        }
    }

    /// Opens or closes the GUI depending on the current state.
    fn toggle_gui(&mut self) {
        match self.state {
            GuiState::Open => {
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    until_global_cancel!(sender.send(GuiAction::Close)).unwrap();
                });
            }
            GuiState::Closed => {
                spawn_gui();
            }
        }

        self.state = self.state.opposite();
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
                checked: match self.state {
                    GuiState::Closed => false,
                    GuiState::Open => true,
                },
                activate: Box::new(Self::toggle_gui),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let sender = this.sender.clone();
                    tokio::spawn(async move {
                        until_global_cancel!(sender.send(GuiAction::Close)).unwrap();
                        GLOBAL_CANCEL.cancel();
                    });
                }),
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
