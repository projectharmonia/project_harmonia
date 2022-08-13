use bevy_egui::egui::{epaint::WHITE_UV, ImageButton, TextureId, Ui};

pub(super) struct ObjectsTab;

impl ObjectsTab {
    pub(super) fn show(self, ui: &mut Ui) {
        ui.group(|ui| {
            if ui
                .add(ImageButton::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]))
                .clicked()
            {}
            if ui
                .add(ImageButton::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]))
                .clicked()
            {}
            if ui
                .add(ImageButton::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]))
                .clicked()
            {}
            if ui
                .add(ImageButton::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]))
                .clicked()
            {}
        });
    }
}
