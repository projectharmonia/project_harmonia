use bevy::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::{
    game_state::GameState,
    settings::{Settings, SettingsApply},
};

pub(super) struct ControlActionsPlugin;

impl Plugin for ControlActionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionState<ControlAction>>()
            .insert_resource(ToggleActions::<ControlAction>::DISABLED)
            .add_startup_system(Self::load_mappings_system)
            .add_enter_system(GameState::City, Self::enable_actions)
            .add_exit_system(GameState::City, Self::disable_actions)
            .add_enter_system(GameState::FamilyEditor, Self::enable_actions)
            .add_exit_system(GameState::FamilyEditor, Self::disable_actions)
            .add_system(Self::load_mappings_system.run_on_event::<SettingsApply>());
    }
}

impl ControlActionsPlugin {
    fn load_mappings_system(mut commands: Commands, settings: Res<Settings>) {
        commands.insert_resource(settings.controls.mappings.clone());
    }

    fn enable_actions(mut toggle_actions: ResMut<ToggleActions<ControlAction>>) {
        toggle_actions.enabled = true;
    }

    fn disable_actions(mut toggle_actions: ResMut<ToggleActions<ControlAction>>) {
        toggle_actions.enabled = false;
    }
}

#[derive(Actionlike, Clone, Copy, Debug, Deserialize, Display, Hash, PartialEq, Serialize)]
pub(crate) enum ControlAction {
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
