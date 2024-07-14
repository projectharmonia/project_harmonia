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
use settings_menu::SettingsMenuPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

pub(super) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
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
