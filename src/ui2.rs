mod camera_2d;
mod main_menu;
mod theme;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use main_menu::MainMenuPlugin;
use theme::ThemePlugin;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ThemePlugin)
            .add(Camera2dPlugin)
            .add(MainMenuPlugin)
    }
}
