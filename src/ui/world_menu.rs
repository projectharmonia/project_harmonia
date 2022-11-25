mod cities_tab;
mod families_tab;

use std::mem;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Button, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::modal_window::{ModalUiExt, ModalWindow};
use crate::core::{
    action::Action,
    city::{City, CityBundle},
    family::{FamilyDespawn, FamilySelect, Members},
    game_state::GameState,
    network::network_event::client_event::ClientSendBuffer,
};
use cities_tab::CitiesTab;
use families_tab::FamiliesTab;

pub(super) struct WorldMenuPlugin;

impl Plugin for WorldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::create_city_system.run_if_resource_exists::<CreateCityDialog>())
            .add_system(Self::world_menu_system.run_in_state(GameState::World));
    }
}

impl WorldMenuPlugin {
    fn world_menu_system(
        mut current_tab: Local<WorldMenuTab>,
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut despawn_buffer: ResMut<ClientSendBuffer<FamilyDespawn>>,
        mut select_buffer: EventWriter<FamilySelect>,
        families: Query<(Entity, &'static Name), With<Members>>,
        cities: Query<(Entity, &'static Name), With<City>>,
    ) {
        Window::new("World menu")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    for tab in WorldMenuTab::iter() {
                        ui.selectable_value(&mut *current_tab, tab, tab.to_string());
                    }
                });
                match *current_tab {
                    WorldMenuTab::Families => FamiliesTab::new(
                        &mut commands,
                        &mut despawn_buffer,
                        &mut select_buffer,
                        &families,
                    )
                    .show(ui),
                    WorldMenuTab::Cities => CitiesTab::new(&mut commands, &cities).show(ui),
                }
            });
    }

    fn create_city_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<Action>>,
        mut dialog: ResMut<CreateCityDialog>,
    ) {
        let mut open = true;
        ModalWindow::new("Create city")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.text_edit_singleline(&mut dialog.city_name);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!dialog.city_name.is_empty(), Button::new("Create"))
                        .clicked()
                    {
                        commands.spawn(CityBundle::new(mem::take(&mut dialog.city_name).into()));
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

#[derive(Default, Display, Clone, Copy, EnumIter, PartialEq)]
enum WorldMenuTab {
    #[default]
    Families,
    Cities,
}

#[derive(Default, Resource)]
struct CreateCityDialog {
    city_name: String,
}
