use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub(super) struct UiActionsPlugin;

impl Plugin for UiActionsPlugin {
    fn build(&self, app: &mut App) {
        let mut input_map = InputMap::default();
        input_map
            .insert(KeyCode::Escape, UiAction::Back)
            .insert(KeyCode::Tab, UiAction::Scoreboard)
            .insert(KeyCode::Return, UiAction::Chat);

        app.init_resource::<ActionState<UiAction>>()
            .insert_resource(input_map);
    }
}

#[derive(Actionlike, Clone, Copy)]
pub(crate) enum UiAction {
    Back,
    Scoreboard,
    Chat,
}
