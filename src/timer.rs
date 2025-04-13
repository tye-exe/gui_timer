use egui::{Color32, Pos2, Shape, Stroke, Ui, Widget, WidgetInfo, WidgetType, emath};

#[derive(Default)]
pub struct Timer {
    radius: Option<f32>,
    progress: Option<f32>,
}

impl Timer {
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }
}

impl Widget for Timer {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(TimerWidget {
            radius: self.radius.unwrap_or(50.0),
            progress: self
                .progress
                .map(|progress| progress.clamp(0.0, 1.0))
                .unwrap_or(0.0),
        })
    }
}

struct TimerWidget {
    radius: f32,
    progress: f32,
}

impl TimerWidget {
    const START_ANGLE: f64 = 140f64.to_radians();
    const END_ANGLE: f64 = 400f64.to_radians();

    fn paint_at(self, ui: &Ui, position: Pos2) {
        let points = 20;

        let outline_points: Vec<Pos2> = (0..=points)
            .map(|i| {
                let angle = emath::lerp(
                    Self::START_ANGLE..=Self::END_ANGLE,
                    i as f64 / points as f64,
                );
                let (sin, cos) = angle.sin_cos();
                position + self.radius * egui::vec2(cos as f32, sin as f32)
            })
            .collect();

        // Outline
        ui.painter().add(Shape::line(
            outline_points,
            Stroke::new(5.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        ));

        let angle_offset: f64 = (Self::END_ANGLE - Self::START_ANGLE) * self.progress as f64;
        let current_angle = Self::START_ANGLE + angle_offset;

        let progress_points: Vec<Pos2> = (0..=points)
            .map(|i| {
                let angle = emath::lerp(current_angle..=Self::END_ANGLE, i as f64 / points as f64);
                let (sin, cos) = angle.sin_cos();
                position + self.radius * egui::vec2(cos as f32, sin as f32)
            })
            .collect();

        // Progress
        ui.painter().add(Shape::line(
            progress_points,
            Stroke::new(3.0, Color32::LIGHT_BLUE),
        ));
    }
}

impl Widget for TimerWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let size = self.radius * 2.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::empty());

        self.paint_at(ui, rect.center());
        response.widget_info(|| WidgetInfo::new(WidgetType::ProgressIndicator));

        response
    }
}
