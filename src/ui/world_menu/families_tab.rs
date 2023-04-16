use bevy::prelude::*;
use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};
use derive_more::Constructor;

use crate::core::{
    actor::ActiveActor,
    family::{FamilyActors, FamilyDespawn},
    game_state::GameState,
};

#[derive(Constructor)]
pub(super) struct FamiliesTab<'a, 'w, 's, 'we, 'wq, 'sq> {
    commands: &'a mut Commands<'w, 's>,
    game_state: &'a mut NextState<GameState>,
    despawn_events: &'a mut EventWriter<'we, FamilyDespawn>,
    families: &'a Query<'wq, 'sq, (Entity, &'static Name, &'static FamilyActors)>,
}

impl FamiliesTab<'_, '_, '_, '_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        for (family_entity, name, actors) in self.families {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        Image::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]),
                    );
                    ui.label(name.as_str());
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        if ui.button("⏵ Play").clicked() {
                            let actor_entity = *actors
                                .first()
                                .expect("family always have at least one member");
                            self.commands.entity(actor_entity).insert(ActiveActor);
                            self.game_state.set(GameState::Family);
                        }
                        if ui.button("🗑 Delete").clicked() {
                            self.despawn_events.send(FamilyDespawn(family_entity));
                        }
                    })
                });
            });
        }
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            if ui.button("➕ Create new").clicked() {
                self.game_state.set(GameState::FamilyEditor);
            }
            ui.allocate_space(ui.available_size_before_wrap());
        });
    }
}
