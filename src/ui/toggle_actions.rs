use bevy::prelude::*;
use bevy_egui::EguiContext;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::core::{control_action::ControlAction, game_state::GameState};

pub(super) struct ToggleActionsPlugin;

impl Plugin for ToggleActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::toggle_actions_system.run_in_state(GameState::FamilyEditor))
            .add_system(Self::toggle_actions_system.run_in_state(GameState::City));
    }
}

impl ToggleActionsPlugin {
    fn toggle_actions_system(
        egui: Res<EguiContext>,
        mut toggle_actions: ResMut<ToggleActions<ControlAction>>,
    ) {
        let ctx = egui.ctx();
        if ctx.wants_pointer_input() || ctx.wants_keyboard_input() {
            toggle_actions.enabled = false;
        } else {
            toggle_actions.enabled = true;
        }
    }
}
