use bevy::prelude::*;
use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};
use derive_more::Constructor;

use super::CreateCityDialog;
use crate::core::{
    city::{ActiveCity, City},
    game_state::GameState,
};

#[derive(Constructor)]
pub(super) struct CitiesTab<'a, 'w, 's, 'wq, 'sq> {
    commands: &'a mut Commands<'w, 's>,
    game_state: &'a mut NextState<GameState>,
    cities: &'a Query<'wq, 'sq, (Entity, &'static Name), With<City>>,
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
                            self.commands.entity(entity).insert(ActiveCity);
                            self.game_state.set(GameState::City);
                        }
                        if ui.button("🗑 Delete").clicked() {
                            self.commands.entity(entity).despawn();
                        }
                    })
                });
            });
        }
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            if ui.button("➕ Create new").clicked() {
                self.commands.init_resource::<CreateCityDialog>();
            }
            ui.allocate_space(ui.available_size_before_wrap());
        });
    }
}
