use std::mem;

use bevy::{app::AppExit, prelude::*};
use bevy_egui::{egui::Button, EguiContexts};
use bevy_renet::renet::RenetClient;
use leafwing_input_manager::{common_conditions::action_just_pressed, prelude::*};

use crate::core::{
    action::Action,
    game_state::GameState,
    game_world::{GameSave, GameWorldPlugin, WorldName},
    lot::{creating_lot::CreatingLot, moving_lot::MovingLot},
    object::placing_object::PlacingObject,
};

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    settings_menu::SettingsMenu,
};

pub(super) struct InGameMenuPlugin;

impl Plugin for InGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::open_ingame_menu_system
                .run_if(action_just_pressed(Action::Cancel))
                .run_if(not(resource_exists::<InGameMenu>()))
                .run_if(not(any_with_component::<PlacingObject>()))
                .run_if(not(any_with_component::<CreatingLot>()))
                .run_if(not(any_with_component::<MovingLot>()))
                .run_if(not(any_with_component::<CreatingLot>()))
                .run_if(in_state(GameState::Family).or_else(in_state(GameState::City))),
            Self::ingame_menu_system.run_if(resource_exists::<InGameMenu>()),
            Self::save_as_system.run_if(resource_exists::<SaveAsDialog>()),
        ))
        .add_systems(
            (
                Self::exit_game_system.run_if(resource_exists::<ExitGameDialog>()),
                Self::exit_to_main_menu_system.run_if(resource_exists::<ExitToMainMenuDialog>()),
            )
                .before(GameWorldPlugin::saving_system),
        );
    }
}

impl InGameMenuPlugin {
    fn open_ingame_menu_system(mut commands: Commands) {
        commands.init_resource::<InGameMenu>();
    }

    fn ingame_menu_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut save_events: EventWriter<GameSave>,
        mut action_state: ResMut<ActionState<Action>>,
        mut game_state: ResMut<NextState<GameState>>,
        client: Option<Res<RenetClient>>,
        state: Res<State<GameState>>,
    ) {
        let mut open = true;
        ModalWindow::new("Menu")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.vertical_centered(|ui| {
                    if ui
                        .add_enabled(client.is_none(), Button::new("Save"))
                        .clicked()
                    {
                        save_events.send_default();
                        ui.close_modal();
                    }
                    if ui.button("Save as...").clicked() {
                        commands.init_resource::<SaveAsDialog>();
                    }
                    if ui.button("Settings").clicked() {
                        commands.init_resource::<SettingsMenu>();
                    }
                    if state.0 != GameState::World && ui.button("Manage world").clicked() {
                        game_state.set(GameState::World);
                        ui.close_modal();
                    }
                    if ui.button("Exit to main menu").clicked() {
                        commands.init_resource::<ExitToMainMenuDialog>();
                    }
                    if ui.button("Exit game").clicked() {
                        commands.init_resource::<ExitGameDialog>();
                    }
                });
            });

        if !open {
            commands.remove_resource::<InGameMenu>();
        }
    }

    fn save_as_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut save_events: EventWriter<GameSave>,
        mut action_state: ResMut<ActionState<Action>>,
        mut world_name: ResMut<WorldName>,
        mut dialog: ResMut<SaveAsDialog>,
    ) {
        let mut open = true;
        ModalWindow::new("Save as...")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.text_edit_singleline(&mut dialog.world_name);
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        world_name.0 = mem::take(&mut dialog.world_name);
                        save_events.send_default();
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !open {
            commands.remove_resource::<SaveAsDialog>();
        }
    }

    fn exit_to_main_menu_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut save_events: EventWriter<GameSave>,
        mut action_state: ResMut<ActionState<Action>>,
        mut game_state: ResMut<NextState<GameState>>,
        client: Option<Res<RenetClient>>,
    ) {
        let mut open = true;
        ModalWindow::new("Exit to main menu")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label("Are you sure you want to exit to the main menu?");
                ui.horizontal(|ui| {
                    if client.is_none() && ui.button("Save and exit").clicked() {
                        save_events.send_default();
                        game_state.set(GameState::MainMenu);
                        commands.remove_resource::<InGameMenu>();
                        ui.close_modal();
                    }
                    if ui.button("Exit to main menu").clicked() {
                        game_state.set(GameState::MainMenu);
                        commands.remove_resource::<InGameMenu>();
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !open {
            commands.remove_resource::<ExitToMainMenuDialog>();
        }
    }

    fn exit_game_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut save_events: EventWriter<GameSave>,
        mut exit_events: EventWriter<AppExit>,
        mut action_state: ResMut<ActionState<Action>>,
        client: Option<Res<RenetClient>>,
    ) {
        let mut open = true;
        ModalWindow::new("Exit game")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label("Are you sure you want to exit the game?");
                ui.horizontal(|ui| {
                    if client.is_none() && ui.button("Save and exit").clicked() {
                        save_events.send_default();
                        exit_events.send_default();
                    }
                    if ui.button("Exit game").clicked() {
                        exit_events.send_default();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !open {
            commands.remove_resource::<ExitGameDialog>();
        }
    }
}

#[derive(Default, Resource)]
struct InGameMenu;

#[derive(Resource)]
struct SaveAsDialog {
    world_name: String,
}

impl FromWorld for SaveAsDialog {
    fn from_world(world: &mut World) -> Self {
        SaveAsDialog {
            world_name: world.resource::<WorldName>().0.clone(),
        }
    }
}

#[derive(Default, Resource)]
struct ExitToMainMenuDialog;

#[derive(Default, Resource)]
struct ExitGameDialog;
