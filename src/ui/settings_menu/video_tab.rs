use bevy_egui::egui::{ComboBox, Ui};

use crate::core::settings::VideoSettings;

pub(super) struct VideoTab<'a> {
    video_settings: &'a mut VideoSettings,
}

impl<'a> VideoTab<'a> {
    #[must_use]
    pub(super) fn new(video_settings: &'a mut VideoSettings) -> Self {
        Self { video_settings }
    }
}

impl VideoTab<'_> {
    pub(super) fn show(self, ui: &mut Ui) {
        ComboBox::from_label("MSAA samples")
            .selected_text(self.video_settings.msaa.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.video_settings.msaa, 1, 1.to_string());
                ui.selectable_value(&mut self.video_settings.msaa, 4, 4.to_string());
            });
        ui.checkbox(
            &mut self.video_settings.perf_stats,
            "Display performance stats",
        );
    }
}
