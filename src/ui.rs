mod main_menu;
mod modal_window;
mod settings_menu;
pub(super) mod ui_action;
mod world_browser;

use bevy::{app::PluginGroupBuilder, prelude::*};

use main_menu::MainMenuPlugin;
use settings_menu::SettingsMenuPlugin;
use ui_action::UiActionsPlugin;
use world_browser::WorldBrowserPlugin;

const UI_MARGIN: f32 = 20.0;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(MainMenuPlugin)
            .add(WorldBrowserPlugin)
            .add(SettingsMenuPlugin)
            .add(UiActionsPlugin);
    }
}
