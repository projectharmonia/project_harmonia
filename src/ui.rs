mod camera_2d;
mod connection_dialog;
mod editor_menu;
mod error_dialog;
mod hud;
mod ingame_menu;
mod main_menu;
mod preview;
mod settings_menu;
mod theme;
mod widget;
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
use theme::ThemePlugin;
use widget::WidgetPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(Camera2dPlugin)
            .add(ConnectionDialogPlugin)
            .add(ErrorDialogPlugin)
            .add(EditorMenuPlugin)
            .add(HudPlugin)
            .add(InGameMenuPlugin)
            .add(WidgetPlugin)
            .add(MainMenuPlugin)
            .add(PreviewPlugin)
            .add(SettingsMenuPlugin)
            .add(ThemePlugin)
            .add(WorldBrowserPlugin)
            .add(WorldMenuPlugin)
    }
}
