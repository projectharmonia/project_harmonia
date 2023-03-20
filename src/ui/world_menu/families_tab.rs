use bevy::prelude::*;
use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};

use crate::core::{
    doll::ActiveDoll,
    family::{Dolls, FamilyDespawn},
    game_state::GameState,
};

pub(super) struct FamiliesTab<'a, 'w, 's, 'we, 'wq, 'sq> {
    commands: &'a mut Commands<'w, 's>,
    game_state: &'a mut NextState<GameState>,
    despawn_events: &'a mut EventWriter<'we, FamilyDespawn>,
    families: &'a Query<'wq, 'sq, (Entity, &'static Name, &'static Dolls)>,
}

impl<'a, 'w, 's, 'we, 'wq, 'sq> FamiliesTab<'a, 'w, 's, 'we, 'wq, 'sq> {
    #[must_use]
    pub(super) fn new(
        commands: &'a mut Commands<'w, 's>,
        game_state: &'a mut NextState<GameState>,
        despawn_events: &'a mut EventWriter<'we, FamilyDespawn>,
        families: &'a Query<'wq, 'sq, (Entity, &'static Name, &'static Dolls)>,
    ) -> Self {
        Self {
            commands,
            game_state,
            despawn_events,
            families,
        }
    }
}

impl FamiliesTab<'_, '_, '_, '_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        for (family_entity, name, dolls) in self.families {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        Image::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]),
                    );
                    ui.label(name.as_str());
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        if ui.button("⏵ Play").clicked() {
                            let doll_entity = *dolls
                                .first()
                                .expect("family always have at least one member");
                            self.commands.entity(doll_entity).insert(ActiveDoll);
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
