use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::settings::{Settings, SettingsApply};

pub(super) struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputMap<Action>>()
            .init_resource::<ActionState<Action>>()
            .add_systems(Startup, Self::set_input_map)
            .add_systems(
                PostUpdate,
                Self::set_input_map.run_if(on_event::<SettingsApply>()),
            );
    }
}

impl ActionPlugin {
    fn set_input_map(mut input_map: ResMut<InputMap<Action>>, settings: Res<Settings>) {
        input_map.clear();
        for (&action, inputs) in &settings.controls.mappings {
            input_map.insert_one_to_many(action, inputs.iter().cloned());
        }
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
