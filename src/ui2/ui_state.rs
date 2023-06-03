use bevy::prelude::*;

pub(super) struct UiStatePlugin;

impl Plugin for UiStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<UiState>();
    }
}

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub(super) enum UiState {
    #[default]
    MainMenu,
    Settings,
    WorldBrowser,
    Hud,
}
