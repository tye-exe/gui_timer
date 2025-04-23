use std::{
    ops::Add,
    time::{Duration, Instant},
};

use egui::{Align2, Color32, Pos2, Shape, Stroke, Ui, Widget, WidgetInfo, WidgetType, emath};

/// Persistent data for a [`Timer`] to use between frames.
/// This needs to be passed into the [`Timer`] on each frame.
///
/// This data can also be seralised and deserialised.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TimerData {
    /// When the timer was last updated.
    /// If this is None, then this is the first update.
    #[serde(skip)]
    last_ticked: Option<Instant>,
    /// How much time has passed.
    duration: Duration,
    /// After how long will the timer end.
    end_after: Duration,
    /// Whether the timer is running.
    paused: bool,
}

impl TimerData {
    /// Create new [`TimerData`] with the given timer duration.
    pub fn new(end_after: Duration) -> Self {
        Self {
            last_ticked: None,
            duration: Duration::ZERO,
            end_after,
            paused: false,
        }
    }

    /// Whether a [`Timer`] is puased.
    pub fn pause(&mut self, pause: bool) {
        self.paused = pause;
    }

    /// Sets the amount of time that has passed to 0.
    pub fn reset(&mut self) {
        self.duration = Duration::ZERO;
    }
}

/// A circular progress bar to indicate an percentage of time remaining.
pub struct Timer<'data> {
    radius: Option<f32>,
    data: &'data mut TimerData,
}

impl<'data> Timer<'data> {
    /// Creates a [`Timer`] with the persistent [`TimerData`].
    pub fn new(data: &'data mut TimerData) -> Self {
        Self { radius: None, data }
    }

    /// Sets the radius of the timer that will be shown.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

impl<'data> Widget for Timer<'data> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(TimerWidget {
            radius: self.radius.unwrap_or(50.0),
            data: self.data,
        })
    }
}

/// Responsible for drawing the widget specified via a [`Timer`].
struct TimerWidget<'data> {
    radius: f32,
    data: &'data mut TimerData,
}

impl<'data> TimerWidget<'data> {
    const START_ANGLE: f64 = 140f64.to_radians();
    const END_ANGLE: f64 = 400f64.to_radians();

    /// Draws the timer widget centered at the given position.
    /// The timer widget extends out by its [`radius`](Self::radius) in a circle.
    fn paint_at(self, ui: &Ui, position: Pos2) {
        // Increment timer
        if !self.data.paused {
            if let Some(last_tick) = self.data.last_ticked {
                let elapsed = Instant::now() - last_tick;
                self.data.duration = self.data.duration.add(elapsed).min(self.data.end_after);
            }

            self.data.last_ticked = Some(Instant::now());
        }

        let progress = self.data.duration.div_duration_f32(self.data.end_after);
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

        let angle_offset: f64 = (Self::END_ANGLE - Self::START_ANGLE) * progress as f64;
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

        // Remaining time.
        let remaining = {
            let secs = self.data.end_after.as_secs() - self.data.duration.as_secs();
            let minuets = secs / 60;
            let hours = minuets / 60;
            format!("{hours:0>2}:{minuets:0>2}:{secs:0>2}")
        };

        let total = {
            let secs = self.data.end_after.as_secs();
            let minuets = secs / 60;
            let hours = minuets / 60;
            format!("{hours:0>2}:{minuets:0>2}:{secs:0>2}")
        };

        ui.painter().text(
            position,
            Align2::CENTER_CENTER,
            format!("{remaining}\n{total}"),
            egui::FontId::default(),
            ui.visuals().text_color(),
        );
    }
}

impl<'data> Widget for TimerWidget<'data> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let size = self.radius * 2.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::empty());

        self.paint_at(ui, rect.center());
        response.widget_info(|| WidgetInfo::new(WidgetType::ProgressIndicator));

        response
    }
}
