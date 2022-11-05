use bevy::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::{
    game_state::GameState,
    settings::{Settings, SettingsApply},
};

pub(super) struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionState<Action>>()
            .insert_resource(ToggleActions::<Action>::DISABLED)
            .add_startup_system(Self::load_mappings_system)
            .add_enter_system(GameState::FamilyEditor, Self::enable_actions_system)
            .add_exit_system(GameState::FamilyEditor, Self::disable_actions_system)
            .add_enter_system(GameState::City, Self::enable_actions_system)
            .add_exit_system(GameState::City, Self::disable_actions_system)
            .add_system(Self::load_mappings_system.run_on_event::<SettingsApply>());
    }
}

impl ActionPlugin {
    fn load_mappings_system(mut commands: Commands, settings: Res<Settings>) {
        commands.insert_resource(settings.controls.mappings.clone());
    }

    fn enable_actions_system(mut toggle_actions: ResMut<ToggleActions<Action>>) {
        toggle_actions.enabled = true;
    }

    fn disable_actions_system(mut toggle_actions: ResMut<ToggleActions<Action>>) {
        toggle_actions.enabled = false;
    }
}

#[derive(Actionlike, Clone, Copy, Debug, Deserialize, Display, Hash, PartialEq, Serialize)]
pub(crate) enum Action {
    #[strum(serialize = "Camera Forward")]
    CameraForward,
    #[strum(serialize = "Camera Backward")]
    CameraBackward,
    #[strum(serialize = "Camera Left")]
    CameraLeft,
    #[strum(serialize = "Camera Right")]
    CameraRight,
    #[strum(serialize = "Rotate Camera")]
    RotateCamera,
    #[strum(serialize = "Zoom Camera")]
    ZoomCamera,
    #[strum(serialize = "Rotate Object")]
    RotateObject,
    Confirm,
    Delete,
    Cancel,
}
