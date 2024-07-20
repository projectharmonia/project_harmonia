mod connection_dialog;
mod editor_menu;
mod ingame_menu;
mod main_menu;
mod settings_menu;
mod world_browser;
mod world_menu;

use bevy::prelude::*;

use connection_dialog::ConnectionDialogPlugin;
use editor_menu::EditorMenuPlugin;
use ingame_menu::InGameMenuPlugin;
use main_menu::MainMenuPlugin;
use project_harmonia_base::core::GameState;
use settings_menu::SettingsMenuPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

pub(super) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<MenuState>()
            .enable_state_scoped_entities::<MenuState>()
            .add_plugins((
                ConnectionDialogPlugin,
                EditorMenuPlugin,
                InGameMenuPlugin,
                MainMenuPlugin,
                SettingsMenuPlugin,
                WorldBrowserPlugin,
                WorldMenuPlugin,
            ));
    }
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Menu)]
pub(super) enum MenuState {
    #[default]
    MainMenu,
    WorldBrowser,
}
