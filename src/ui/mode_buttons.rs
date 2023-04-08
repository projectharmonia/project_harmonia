use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContexts,
};
use strum::IntoEnumIterator;

use crate::core::{family::FamilyMode, game_state::GameState};

pub(super) struct ModeButtonsPlugin;

impl Plugin for ModeButtonsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::mode_buttons_system.in_set(OnUpdate(GameState::Family)));
    }
}

impl ModeButtonsPlugin {
    fn mode_buttons_system(
        mut egui: EguiContexts,
        family_mode: Res<State<FamilyMode>>,
        mut next_family_mode: ResMut<NextState<FamilyMode>>,
    ) {
        Window::new("Mode buttons")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::RIGHT_TOP, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    let mut current_mode = family_mode.0;
                    for mode in FamilyMode::iter() {
                        ui.selectable_value(
                            &mut current_mode,
                            mode,
                            RichText::new(mode.glyph()).size(22.0),
                        )
                        .on_hover_text(mode.to_string());
                    }
                    if current_mode != family_mode.0 {
                        next_family_mode.set(current_mode);
                    }
                });
            });
    }
}
