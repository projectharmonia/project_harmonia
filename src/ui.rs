mod city_hud;
mod connection_dialog;
mod error_message;
mod family_editor_menu;
mod ingame_menu;
mod main_menu;
mod modal_window;
mod selected_object;
mod settings_menu;
mod task_menu;
mod world_browser;
mod world_menu;

use bevy::{app::PluginGroupBuilder, prelude::*};

use city_hud::CityHudPlugin;
use connection_dialog::ConnectionDialogPlugin;
use error_message::ErrorMessagePlugin;
use family_editor_menu::FamilyEditorMenuPlugin;
use ingame_menu::InGameMenuPlugin;
use main_menu::MainMenuPlugin;
use modal_window::ModalWindowPlugin;
use selected_object::SelectedObjectPlugin;
use settings_menu::SettingsMenuPlugin;
use task_menu::TaskMenuPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

const UI_MARGIN: f32 = 20.0;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(CityHudPlugin)
            .add(ConnectionDialogPlugin)
            .add(ErrorMessagePlugin)
            .add(FamilyEditorMenuPlugin)
            .add(InGameMenuPlugin)
            .add(MainMenuPlugin)
            .add(ModalWindowPlugin)
            .add(SelectedObjectPlugin)
            .add(SettingsMenuPlugin)
            .add(TaskMenuPlugin)
            .add(WorldBrowserPlugin)
            .add(WorldMenuPlugin);
    }
}
