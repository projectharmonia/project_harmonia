mod building_hud;
mod city_hud;
mod connection_dialog;
mod error_message;
mod family_editor_menu;
mod game_inspector;
mod ingame_menu;
mod life_hud;
mod main_menu;
mod modal_window;
mod mode_buttons;
mod objects_view;
mod settings_menu;
mod task_menu;
mod world_browser;
mod world_menu;

use bevy::{app::PluginGroupBuilder, prelude::*};

use building_hud::BuildingHudPlugin;
use city_hud::CityHudPlugin;
use connection_dialog::ConnectionDialogPlugin;
use error_message::ErrorMessagePlugin;
use family_editor_menu::FamilyEditorMenuPlugin;
use game_inspector::GameInspectorPlugin;
use ingame_menu::InGameMenuPlugin;
use life_hud::LifeHudPlugin;
use main_menu::MainMenuPlugin;
use modal_window::ModalWindowPlugin;
use mode_buttons::ModeButtonsPlugin;
use settings_menu::SettingsMenuPlugin;
use task_menu::TaskMenuPlugin;
use world_browser::WorldBrowserPlugin;
use world_menu::WorldMenuPlugin;

const UI_MARGIN: f32 = 20.0;

pub(super) struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BuildingHudPlugin)
            .add(CityHudPlugin)
            .add(ConnectionDialogPlugin)
            .add(ErrorMessagePlugin)
            .add(FamilyEditorMenuPlugin)
            .add(GameInspectorPlugin)
            .add(InGameMenuPlugin)
            .add(LifeHudPlugin)
            .add(MainMenuPlugin)
            .add(ModalWindowPlugin)
            .add(ModeButtonsPlugin)
            .add(SettingsMenuPlugin)
            .add(TaskMenuPlugin)
            .add(WorldBrowserPlugin)
            .add(WorldMenuPlugin)
    }
}
