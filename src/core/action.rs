use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::settings::{Settings, SettingsApply};

pub(super) struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionState<Action>>()
            .add_systems(Startup, Self::load_mappings_system)
            .add_systems(
                PostUpdate,
                Self::load_mappings_system.run_if(on_event::<SettingsApply>()),
            );
    }
}

impl ActionPlugin {
    fn load_mappings_system(mut commands: Commands, settings: Res<Settings>) {
        let mut input_map = InputMap::default();
        for (&action, inputs) in &settings.controls.mappings {
            input_map.insert_many_to_one(inputs.iter().cloned(), action);
        }
        commands.insert_resource(input_map);
    }
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
    Reflect,
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
