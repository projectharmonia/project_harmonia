mod button;
mod camera_2d;
mod checkbox;
mod main_menu;
mod settings_menu;
mod theme;
mod ui_state;

use bevy::{app::PluginGroupBuilder, prelude::*};

use button::ButtonPlugin;
use camera_2d::Camera2dPlugin;
use checkbox::CheckboxPlugin;
use main_menu::MainMenuPlugin;
use settings_menu::SettingsMenuPlugin;
use theme::ThemePlugin;
use ui_state::UiStatePlugin;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(UiStatePlugin)
            .add(ButtonPlugin)
            .add(Camera2dPlugin)
            .add(CheckboxPlugin)
            .add(MainMenuPlugin)
            .add(SettingsMenuPlugin)
            .add(ThemePlugin)
    }
}
