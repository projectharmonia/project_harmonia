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
            &mut self.developer_settings.game_inspector,
            "Enable game inspector",
        );
        ui.checkbox(
            &mut self.developer_settings.debug_collisions,
            "Debug collisions",
        );
        ui.checkbox(
            &mut self.developer_settings.debug_paths,
            "Debug navigation paths",
        );
        ui.checkbox(&mut self.developer_settings.wireframe, "Enable wireframe");
    }
}
