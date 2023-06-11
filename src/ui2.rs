mod camera_2d;
mod main_menu;
mod settings_menu;
mod theme;
mod widget;
mod world_browser;
mod world_menu;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use main_menu::MainMenuPlugin;
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
            .add(WidgetPlugin)
            .add(MainMenuPlugin)
            .add(SettingsMenuPlugin)
            .add(ThemePlugin)
            .add(WorldBrowserPlugin)
            .add(WorldMenuPlugin)
    }
}
