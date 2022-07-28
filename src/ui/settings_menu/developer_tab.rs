use bevy_egui::egui::Ui;

use crate::core::settings::DeveloperSettings;

pub(super) struct DeveloperTab<'a> {
    developer_settings: &'a mut DeveloperSettings,
}

impl<'a> DeveloperTab<'a> {
    #[must_use]
    pub(super) fn new(developer_settings: &'a mut DeveloperSettings) -> Self {
        Self { developer_settings }
    }
}

impl DeveloperTab<'_> {
    pub(super) fn show(self, ui: &mut Ui) {
        ui.checkbox(
            &mut self.developer_settings.world_inspector,
            "Enable world inspector",
        );
        ui.checkbox(
            &mut self.developer_settings.debug_collisions,
            "Debug collisions",
        );
    }
}
