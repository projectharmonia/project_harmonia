mod camera_2d;
mod main_menu;
mod settings_menu;
mod theme;
mod ui_state;
mod widget;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use main_menu::MainMenuPlugin;
use settings_menu::SettingsMenuPlugin;
use theme::ThemePlugin;
use ui_state::UiStatePlugin;
use widget::WidgetPlugin;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(UiStatePlugin)
            .add(Camera2dPlugin)
            .add(WidgetPlugin)
            .add(MainMenuPlugin)
            .add(SettingsMenuPlugin)
            .add(ThemePlugin)
    }
}
