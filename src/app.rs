use egui::Widget;
use serde::{Deserialize, Serialize};

use crate::timer::Timer;

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct Gui {
    progress: f32,
}

impl Gui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, "gui_timer").unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Timer::default().radius(50.0).progress(self.progress).ui(ui);
        });

        self.progress += 0.01;
    }
}
