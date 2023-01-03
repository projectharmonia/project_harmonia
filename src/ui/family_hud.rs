mod life_hud;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use strum::IntoEnumIterator;

use crate::core::{family::FamilyMode, game_state::GameState};
use life_hud::LifeHudPlugin;

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(LifeHudPlugin)
            .add_system(Self::mode_buttons_system.run_in_state(GameState::Family));
    }
}

impl FamilyHudPlugin {
    fn mode_buttons_system(
        mut commands: Commands,
        family_mode: ResMut<CurrentState<FamilyMode>>,
        mut egui: ResMut<EguiContext>,
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
                        commands.insert_resource(NextState(current_mode))
                    }
                });
            });
    }
}
