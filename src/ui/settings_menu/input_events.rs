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
