use bevy::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::settings::{Settings, SettingsApply};

pub(super) struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionState<Action>>()
            .add_startup_system(Self::load_mappings_system)
            .add_system(Self::load_mappings_system.run_on_event::<SettingsApply>());
    }
}

impl ActionPlugin {
    fn load_mappings_system(mut commands: Commands, settings: Res<Settings>) {
        commands.insert_resource(settings.controls.mappings.clone());
    }
}

/// A condition for systems to check if an action was just pressed.
pub(crate) const fn just_pressed(action: Action) -> impl Fn(Res<ActionState<Action>>) -> bool {
    move |action_state: Res<ActionState<Action>>| -> bool { action_state.just_pressed(action) }
}

/// A condition for systems to check if an action was pressed.
pub(crate) const fn pressed(action: Action) -> impl Fn(Res<ActionState<Action>>) -> bool {
    move |action_state: Res<ActionState<Action>>| -> bool { action_state.pressed(action) }
}

#[derive(
    Actionlike,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Display,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
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
