mod cities_tab;
mod families_tab;

use std::mem;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Window},
    EguiContext,
};
use bevy_inspector_egui::egui::Button;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    ui_action::UiAction,
};
use crate::core::{
    city::{City, CityBundle},
    family::Family,
    game_state::GameState,
};
use cities_tab::CitiesTab;
use families_tab::FamiliesTab;

pub(super) struct WorldMenuPlugin;

impl Plugin for WorldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::InGame, Self::open_world_menu_system)
            .add_exit_system(GameState::InGame, Self::close_world_menu_system)
            .add_system(Self::create_city_system.run_if_resource_exists::<CreateCityDialog>())
            .add_system(Self::world_menu_system.run_if_resource_exists::<WorldMenu>());
    }
}

impl WorldMenuPlugin {
    fn open_world_menu_system(mut commands: Commands) {
        commands.init_resource::<WorldMenu>();
    }

    fn close_world_menu_system(mut commands: Commands) {
        commands.remove_resource::<WorldMenu>();
    }

    fn world_menu_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut world_menu: ResMut<WorldMenu>,
        families: Query<&'static Name, With<Family>>,
        cities: Query<&'static Name, With<City>>,
    ) {
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
                    WorldMenuTab::Families => FamiliesTab::new(&mut commands, &families).show(ui),
                    WorldMenuTab::Cities => CitiesTab::new(&mut commands, &cities).show(ui),
                }
            });
    }

    fn create_city_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut create_city_dialog: ResMut<CreateCityDialog>,
    ) {
        let mut open = true;
        ModalWindow::new("Create city")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.text_edit_singleline(&mut create_city_dialog.city_name);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !create_city_dialog.city_name.is_empty(),
                            Button::new("Create"),
                        )
                        .clicked()
                    {
                        commands.spawn_bundle(CityBundle::new(
                            mem::take(&mut create_city_dialog.city_name).into(),
                        ));
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !open {
            commands.remove_resource::<CreateCityDialog>();
        }
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

#[derive(Default)]
struct CreateCityDialog {
    city_name: String,
}
