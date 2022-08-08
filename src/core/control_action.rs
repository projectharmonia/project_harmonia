use bevy::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::{
    game_state::GameState,
    settings::{Settings, SettingsApplied},
};

pub(super) struct ControlActionsPlugin;

impl Plugin for ControlActionsPlugin {
    fn build(&self, app: &mut App) {
        let mut toggle_actions = ToggleActions::<ControlAction>::default();
        toggle_actions.enabled = false;

        app.insert_resource(toggle_actions)
            .add_startup_system(Self::load_mappings_system)
            .add_enter_system(GameState::InGame, Self::enable_actions)
            .add_exit_system(GameState::InGame, Self::disable_actions)
            .add_system(Self::load_mappings_system.run_on_event::<SettingsApplied>());
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
#[allow(clippy::enum_variant_names)]
pub(crate) enum ControlAction {
    #[strum(serialize = "Camera Forward")]
    CameraForward,
    #[strum(serialize = "Camera Backward")]
    CameraBackward,
    #[strum(serialize = "Camera Left")]
    CameraLeft,
    #[strum(serialize = "Camera Right")]
    CameraRight,
}

#[cfg(test)]
mod tests {
    use bevy::ecs::event::Events;

    use super::*;

    #[test]
    fn loading_settings() {
        let mut app = App::new();
        app.add_plugin(TestControlActionsPlugin);

        app.update();

        let mappings = app.world.resource::<InputMap<ControlAction>>();
        let settings = app.world.resource::<Settings>();
        assert_eq!(
            settings.controls.mappings, *mappings,
            "Added mappings should the same as in settings"
        );

        // Change settings to test reloading
        let mut settings = app.world.resource_mut::<Settings>();
        settings
            .controls
            .mappings
            .insert(KeyCode::Q, ControlAction::CameraForward);

        let mut apply_events = app.world.resource_mut::<Events<SettingsApplied>>();
        apply_events.send(SettingsApplied);

        app.update();

        let settings = app.world.resource::<Settings>();
        let mappings = app.world.resource::<InputMap<ControlAction>>();
        assert_eq!(
            settings.controls.mappings, *mappings,
            "Mappings should be updated on apply event"
        );
    }

    #[test]
    fn actions_toggling() {
        let mut app = App::new();
        app.add_plugin(TestControlActionsPlugin);

        assert!(
            !app.world.resource::<ToggleActions<ControlAction>>().enabled,
            "Control actions should be disabled at startup"
        );

        app.world.insert_resource(NextState(GameState::InGame));
        app.update();

        assert!(
            app.world.resource::<ToggleActions<ControlAction>>().enabled,
            "Control actions should be enabled after entering {}",
            GameState::InGame
        );

        app.world.insert_resource(NextState(GameState::Menu));
        app.update();

        assert!(
            !app.world.resource::<ToggleActions<ControlAction>>().enabled,
            "Control actions should be disabled after exiting {}",
            GameState::InGame
        );
    }

    struct TestControlActionsPlugin;

    impl Plugin for TestControlActionsPlugin {
        fn build(&self, app: &mut App) {
            app.add_loopless_state(GameState::Menu)
                .add_event::<SettingsApplied>()
                .init_resource::<Settings>()
                .add_plugin(ControlActionsPlugin);
        }
    }
}
