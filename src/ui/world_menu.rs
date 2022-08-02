mod cities_tab;
mod families_tab;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::core::game_state::GameState;
use cities_tab::CitiesTab;
use families_tab::FamiliesTab;

pub(super) struct WorldMenuPlugin;

impl Plugin for WorldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::InGame, Self::open_world_menu_system)
            .add_system(Self::world_menu_system.run_if_resource_exists::<WorldMenu>());
    }
}

impl WorldMenuPlugin {
    fn world_menu_system(mut egui: ResMut<EguiContext>, mut world_menu: ResMut<WorldMenu>) {
        Window::new("World menu")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    for tab in WorldMenuTab::iter() {
                        ui.selectable_value(&mut world_menu.current_tab, tab, tab.to_string());
                    }
                });
                match world_menu.current_tab {
                    WorldMenuTab::Families => FamiliesTab::new(&["Test1", "Test2"]).show(ui),
                    WorldMenuTab::Cities => CitiesTab::new(&["Test1", "Test2"]).show(ui),
                }
            });
    }

    fn open_world_menu_system(mut commands: Commands) {
        commands.init_resource::<WorldMenu>();
    }
}

#[derive(Default)]
pub(super) struct WorldMenu {
    current_tab: WorldMenuTab,
}

#[derive(Default, Display, Clone, Copy, EnumIter, PartialEq)]
enum WorldMenuTab {
    #[default]
    Families,
    Cities,
}
