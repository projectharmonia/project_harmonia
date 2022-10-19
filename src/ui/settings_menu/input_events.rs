use bevy::{
    ecs::system::SystemParam,
    input::{keyboard::KeyboardInput, mouse::MouseButtonInput, ButtonState},
    prelude::*,
};
use leafwing_input_manager::user_input::InputKind;

/// Helper for collecting input
#[derive(SystemParam)]
pub(super) struct InputEvents<'w, 's> {
    keys: EventReader<'w, 's, KeyboardInput>,
    mouse_buttons: EventReader<'w, 's, MouseButtonInput>,
    gamepad_events: EventReader<'w, 's, GamepadEvent>,
}

impl InputEvents<'_, '_> {
    pub(super) fn input_kind(&mut self) -> Option<InputKind> {
        if let Some(input) = self
            .keys
            .iter()
            .filter(|input| input.state == ButtonState::Released)
            .find_map(|input| input.key_code)
        {
            return Some(input.into());
        }

        if let Some(input) = self
            .mouse_buttons
            .iter()
            .find(|input| input.state == ButtonState::Released)
        {
            return Some(input.button.into());
        }

        if let Some(GamepadEventType::ButtonChanged(button, strength)) = self
            .gamepad_events
            .iter()
            .map(|event| event.event_type.to_owned())
            .next()
        {
            if strength == 1.0 {
                return Some(button.into());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::event::Events, ecs::system::SystemState, input::InputPlugin};

    use super::*;

    #[test]
    fn no_input() {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        assert_eq!(input_events.input_kind(), None);
    }

    #[test]
    fn keyboard_release() {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        const KEY: KeyCode = KeyCode::Space;
        let mut keyboard_events = app.world.resource_mut::<Events<KeyboardInput>>();
        keyboard_events.send(KeyboardInput {
            scan_code: 0,
            key_code: Some(KEY),
            state: ButtonState::Released,
        });

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        assert!(matches!(
            input_events.input_kind(),
            Some(InputKind::Keyboard(KEY))
        ));
    }

    #[test]
    fn mouse_release() {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        const BUTTON: MouseButton = MouseButton::Right;
        let mut mouse_button_events = app.world.resource_mut::<Events<MouseButtonInput>>();
        mouse_button_events.send(MouseButtonInput {
            button: BUTTON,
            state: ButtonState::Released,
        });

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        assert!(matches!(
            input_events.input_kind(),
            Some(InputKind::Mouse(BUTTON))
        ));
    }

    #[test]
    fn gamepad_release() {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        const BUTTON: GamepadButtonType = GamepadButtonType::Z;
        let mut gamepad_events = app.world.resource_mut::<Events<GamepadEvent>>();
        gamepad_events.send(GamepadEvent {
            gamepad: Gamepad { id: 0 },
            event_type: GamepadEventType::ButtonChanged(BUTTON, 1.0),
        });

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        assert!(matches!(
            input_events.input_kind(),
            Some(InputKind::GamepadButton(BUTTON))
        ));
    }
}
