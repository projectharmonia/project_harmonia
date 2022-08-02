mod ingame_menu;
mod main_menu;
mod modal_window;
mod settings_menu;
pub(super) mod ui_action;
mod world_browser;
mod world_menu;

use bevy::{app::PluginGroupBuilder, prelude::*};

use ingame_menu::InGameMenuPlugin;
use main_menu::MainMenuPlugin;
use settings_menu::SettingsMenuPlugin;
use ui_action::UiActionsPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

const UI_MARGIN: f32 = 20.0;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(MainMenuPlugin)
            .add(WorldBrowserPlugin)
            .add(SettingsMenuPlugin)
            .add(InGameMenuPlugin)
            .add(WorldMenuPlugin)
            .add(UiActionsPlugin);
    }
}
