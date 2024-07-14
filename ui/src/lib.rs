mod camera_2d;
mod connection_dialog;
mod editor_menu;
mod error_dialog;
mod hud;
mod ingame_menu;
mod main_menu;
mod preview;
mod settings_menu;
mod ui_root;
mod world_browser;
mod world_menu;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use connection_dialog::ConnectionDialogPlugin;
use editor_menu::EditorMenuPlugin;
use error_dialog::ErrorDialogPlugin;
use hud::HudPlugin;
use ingame_menu::InGameMenuPlugin;
use main_menu::MainMenuPlugin;
use preview::PreviewPlugin;
use settings_menu::SettingsMenuPlugin;
use ui_root::UiRootPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

pub struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(Camera2dPlugin)
            .add(ConnectionDialogPlugin)
            .add(ErrorDialogPlugin)
            .add(EditorMenuPlugin)
            .add(HudPlugin)
            .add(UiRootPlugin)
            .add(InGameMenuPlugin)
            .add(MainMenuPlugin)
            .add(PreviewPlugin)
            .add(SettingsMenuPlugin)
            .add(WorldBrowserPlugin)
            .add(WorldMenuPlugin)
    }
}
