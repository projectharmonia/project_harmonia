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
            if strength <= 0.5 {
                return Some(button.into());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use bevy::{ecs::event::Events, ecs::system::SystemState, input::InputPlugin};

    use super::*;

    #[test]
    fn input_events_reads_keyboard() -> Result<()> {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        const KEY: KeyCode = KeyCode::Space;
        let mut keyboard_input = app.world.resource_mut::<Events<KeyboardInput>>();
        keyboard_input.send(KeyboardInput {
            scan_code: 0,
            key_code: Some(KEY),
            state: ButtonState::Released,
        });

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        let input_kind = input_events
            .input_kind()
            .context("Input should be detected")?;

        assert_eq!(
            input_kind,
            InputKind::Keyboard(KEY),
            "Input should be equal to the released keyboard key"
        );

        Ok(())
    }

    #[test]
    fn input_events_reads_mouse() -> Result<()> {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        const BUTTON: MouseButton = MouseButton::Right;
        let mut mouse_button = app.world.resource_mut::<Events<MouseButtonInput>>();
        mouse_button.send(MouseButtonInput {
            button: BUTTON,
            state: ButtonState::Released,
        });

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        let input_kind = input_events
            .input_kind()
            .context("Input should be detected")?;

        assert_eq!(
            input_kind,
            InputKind::Mouse(BUTTON),
            "Input should be equal to the released mouse button"
        );

        Ok(())
    }

    #[test]
    fn input_events_reads_gamepad() -> Result<()> {
        let mut app = App::new();
        app.add_plugin(InputPlugin);

        const BUTTON: GamepadButtonType = GamepadButtonType::Z;
        const PRESSED_STRENGTH: f32 = 0.6;
        let mut gamepad_events = app.world.resource_mut::<Events<GamepadEvent>>();
        gamepad_events.send(GamepadEvent {
            gamepad: Gamepad { id: 0 },
            event_type: GamepadEventType::ButtonChanged(BUTTON, PRESSED_STRENGTH),
        });

        let mut system_state: SystemState<InputEvents> = SystemState::new(&mut app.world);
        let mut input_events = system_state.get_mut(&mut app.world);
        assert_eq!(
            input_events.input_kind(),
            None,
            "Input shouldn't be detected when pressed strength is {PRESSED_STRENGTH}"
        );

        const RELEASED_STRENGTH: f32 = 0.5;
        let mut gamepad_events = app.world.resource_mut::<Events<GamepadEvent>>();
        gamepad_events.send(GamepadEvent {
            gamepad: Gamepad { id: 0 },
            event_type: GamepadEventType::ButtonChanged(BUTTON, RELEASED_STRENGTH),
        });

        let mut input_events = system_state.get_mut(&mut app.world);
        let input_kind = input_events.input_kind().with_context(|| {
            format!("Input should be detected with {RELEASED_STRENGTH} strength")
        })?;

        assert_eq!(
            input_kind,
            InputKind::GamepadButton(BUTTON),
            "Input should be equal to the released gamepad button"
        );

        Ok(())
    }
}
