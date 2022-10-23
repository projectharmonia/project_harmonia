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
        let mut toggle_actions = ToggleActions::<ControlAction>::default();
        toggle_actions.enabled = false;

        app.init_resource::<ActionState<ControlAction>>()
            .insert_resource(toggle_actions)
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

#[cfg(test)]
mod tests {
    use bevy::ecs::event::Events;

    use super::*;

    #[test]
    fn loading() {
        let mut app = App::new();
        app.add_plugin(TestControlActionsPlugin);

        app.update();

        let mappings = app.world.resource::<InputMap<ControlAction>>();
        let settings = app.world.resource::<Settings>();
        assert_eq!(settings.controls.mappings, *mappings);
    }

    #[test]
    fn applying() {
        let mut app = App::new();
        app.add_plugin(TestControlActionsPlugin);

        let mut settings = app.world.resource_mut::<Settings>();
        settings
            .controls
            .mappings
            .insert(KeyCode::Q, ControlAction::CameraForward);

        let mut apply_events = app.world.resource_mut::<Events<SettingsApply>>();
        apply_events.send(SettingsApply);

        app.update();

        let settings = app.world.resource::<Settings>();
        let mappings = app.world.resource::<InputMap<ControlAction>>();
        assert_eq!(settings.controls.mappings, *mappings);
    }

    #[test]
    fn toggling() {
        let mut app = App::new();
        app.add_plugin(TestControlActionsPlugin);

        assert!(!app.world.resource::<ToggleActions<ControlAction>>().enabled);

        app.world.insert_resource(NextState(GameState::City));
        app.update();

        assert!(app.world.resource::<ToggleActions<ControlAction>>().enabled);

        app.world.insert_resource(NextState(GameState::MainMenu));
        app.update();

        assert!(!app.world.resource::<ToggleActions<ControlAction>>().enabled);
    }

    struct TestControlActionsPlugin;

    impl Plugin for TestControlActionsPlugin {
        fn build(&self, app: &mut App) {
            app.add_loopless_state(GameState::MainMenu)
                .add_event::<SettingsApply>()
                .init_resource::<Settings>()
                .add_plugin(ControlActionsPlugin);
        }
    }
}
