use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub(super) struct UiActionPlugin;

impl Plugin for UiActionPlugin {
    fn build(&self, app: &mut App) {
        let mut input_map = InputMap::default();
        input_map.insert(KeyCode::Escape, UiAction::Back);

        app.init_resource::<ActionState<UiAction>>()
            .insert_resource(input_map);
    }
}

#[derive(Actionlike, Clone, Copy)]
pub(crate) enum UiAction {
    Back,
}
