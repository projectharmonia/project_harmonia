use bevy::{
    ecs::system::SystemParam,
    input::{
        gamepad::GamepadButtonChangedEvent, keyboard::KeyboardInput, mouse::MouseButtonInput,
        ButtonState,
    },
    prelude::*,
};
use leafwing_input_manager::user_input::InputKind;

/// Collects input to detect currently pressed [`InputKind`].
#[derive(SystemParam)]
pub(crate) struct InputEvents<'w, 's> {
    keys: EventReader<'w, 's, KeyboardInput>,
    mouse_buttons: EventReader<'w, 's, MouseButtonInput>,
    gamepad_buttons: EventReader<'w, 's, GamepadButtonChangedEvent>,
    interactions: Query<'w, 's, &'static Interaction>,
}

impl InputEvents<'_, '_> {
    pub(crate) fn input_kind(&mut self) -> Option<InputKind> {
        if let Some(input) = self
            .keys
            .read()
            .filter(|input| input.state == ButtonState::Released)
            .find_map(|input| input.key_code)
        {
            return Some(input.into());
        }

        // Ignore mouse buttons if any UI element is interacting
        // to avoid registering button clicks as input.
        if self
            .interactions
            .iter()
            .all(|&interaction| interaction == Interaction::None)
        {
            if let Some(input) = self
                .mouse_buttons
                .read()
                .find(|input| input.state == ButtonState::Released)
            {
                return Some(input.button.into());
            }
        }

        if let Some(input) = self.gamepad_buttons.read().next() {
            if input.value == 1.0 {
                return Some(input.button_type.into());
            }
        }

        None
    }
}
