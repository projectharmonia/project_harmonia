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
pub struct InputEvents<'w, 's> {
    keys: EventReader<'w, 's, KeyboardInput>,
    mouse_buttons: EventReader<'w, 's, MouseButtonInput>,
    gamepad_buttons: EventReader<'w, 's, GamepadButtonChangedEvent>,
    interactions: Query<'w, 's, &'static Interaction>,
}

impl InputEvents<'_, '_> {
    pub fn input_kind(&mut self) -> Option<InputKind> {
        if let Some(input) = self
            .keys
            .read()
            .find(|input| input.state == ButtonState::Released)
        {
            info!("received `{:?}`", input.key_code);
            return Some(input.key_code.into());
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
                info!("received `{:?}`", input.button);
                return Some(input.button.into());
            }
        }

        if let Some(input) = self.gamepad_buttons.read().next() {
            if input.value == 1.0 {
                info!("received `{:?}`", input.button_type);
                return Some(input.button_type.into());
            }
        }

        None
    }
}
