mod back_button;
mod main_menu;
mod settings_menu;
pub(super) mod ui_action;
mod ui_state;

use bevy::{app::PluginGroupBuilder, prelude::*};

use main_menu::MainMenuPlugin;
use settings_menu::SettingsMenuPlugin;
use ui_action::UiActionsPlugin;
use ui_state::UiStatePlugin;

const UI_MARGIN: f32 = 20.0;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(UiStatePlugin)
            .add(MainMenuPlugin)
            .add(SettingsMenuPlugin)
            .add(UiActionsPlugin);
    }
}
