use bevy::prelude::*;
use iyes_loopless::prelude::*;

pub(super) struct UiStatePlugin;

impl Plugin for UiStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(UiState::MainMenu);
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(super) enum UiState {
    MainMenu,
    SettingsMenu,
    WorldBrowser,
    InGameMenu,
}
