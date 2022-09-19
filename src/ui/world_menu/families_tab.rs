use bevy::prelude::*;
use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};

use crate::core::family::{Family, FamilyBundle};

pub(super) struct FamiliesTab<'a, 'w, 's, 'wq, 'sq> {
    commands: &'a mut Commands<'w, 's>,
    families: &'a Query<'wq, 'sq, (Entity, &'static Name), With<Family>>,
}

impl<'a, 'w, 's, 'wq, 'sq> FamiliesTab<'a, 'w, 's, 'wq, 'sq> {
    #[must_use]
    pub(super) fn new(
        commands: &'a mut Commands<'w, 's>,
        families: &'a Query<'wq, 'sq, (Entity, &'static Name), With<Family>>,
    ) -> Self {
        Self { families, commands }
    }
}

impl FamiliesTab<'_, '_, '_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        for (entity, name) in self.families {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        Image::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]),
                    );
                    ui.label(name.as_str());
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        if ui.button("⏵ Play").clicked() {}
                        if ui.button("🗑 Delete").clicked() {
                            self.commands.entity(entity).despawn();
                        }
                    })
                });
            });
        }
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            if ui.button("➕ Create new").clicked() {
                self.commands.spawn_bundle(FamilyBundle::default());
            }
            ui.allocate_space(ui.available_size_before_wrap());
        });
    }
}
