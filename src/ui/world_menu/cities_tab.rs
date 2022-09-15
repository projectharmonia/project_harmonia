use bevy::prelude::*;
use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};
use iyes_loopless::prelude::*;

use super::CreateCityDialog;
use crate::core::{city::City, game_state::GameState};

pub(super) struct CitiesTab<'a, 'w, 's, 'wq, 'sq> {
    commands: &'a mut Commands<'w, 's>,
    cities: &'a Query<'wq, 'sq, (Entity, &'static Name), With<City>>,
}

impl<'a, 'w, 's, 'wq, 'sq> CitiesTab<'a, 'w, 's, 'wq, 'sq> {
    #[must_use]
    pub(super) fn new(
        commands: &'a mut Commands<'w, 's>,
        cities: &'a Query<'wq, 'sq, (Entity, &'static Name), With<City>>,
    ) -> Self {
        Self { cities, commands }
    }
}

impl CitiesTab<'_, '_, '_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        for (entity, name) in self.cities {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        Image::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]),
                    );
                    ui.label(name.as_str());
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        if ui.button("✏ Edit").clicked() {
                            self.commands
                                .entity(entity)
                                .insert_bundle(VisibilityBundle::default());
                            self.commands.insert_resource(NextState(GameState::City));
                        }
                        if ui.button("🗑 Delete").clicked() {
                            self.commands.entity(entity).despawn();
                        }
                    })
                });
            });
        }
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            if ui.button("➕ Create new").clicked() {
                self.commands.init_resource::<CreateCityDialog>();
            }
        });
    }
}
