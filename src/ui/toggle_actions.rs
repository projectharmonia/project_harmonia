use bevy::prelude::*;
use bevy_egui::{egui::Id, EguiContext};
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;

use super::modal_window::ModalIds;
use crate::core::{action::Action, game_state::GameState};

pub(super) struct ToggleActionsPlugin;

impl Plugin for ToggleActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::toggle_actions_system.run_in_state(GameState::FamilyEditor))
            .add_system(Self::toggle_actions_system.run_in_state(GameState::City));
    }
}

impl ToggleActionsPlugin {
    fn toggle_actions_system(
        mut egui: ResMut<EguiContext>,
        mut toggle_actions: ResMut<ToggleActions<Action>>,
    ) {
        let ctx = egui.ctx_mut();
        if ctx.wants_pointer_input()
            || ctx.wants_keyboard_input()
            || !ctx
                .data()
                .get_temp_mut_or_default::<ModalIds>(Id::null())
                .is_empty()
        {
            toggle_actions.enabled = false;
        } else {
            toggle_actions.enabled = true;
        }
    }
}
