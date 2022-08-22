use std::mem;

use bevy::{app::AppExit, prelude::*};
use bevy_egui::EguiContext;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::core::{
    game_state::GameState,
    game_world::{GameSaved, GameWorld},
};

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    settings_menu::SettingsMenu,
    ui_action::UiAction,
};

pub(super) struct InGameMenuPlugin;

impl Plugin for InGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::open_ingame_menu_system
                .run_not_in_state(GameState::MainMenu)
                .run_unless_resource_exists::<InGameMenu>(),
        )
        .add_enter_system(GameState::MainMenu, Self::close_ingame_menu)
        .add_system(Self::ingame_menu_system.run_if_resource_exists::<InGameMenu>())
        .add_system(Self::save_as_system.run_if_resource_exists::<SaveAsDialog>())
        .add_system(Self::exit_to_main_menu_system.run_if_resource_exists::<ExitToMainMenuDialog>())
        .add_system(Self::exit_game_system.run_if_resource_exists::<ExitGameDialog>());
    }
}

impl InGameMenuPlugin {
    fn open_ingame_menu_system(mut commands: Commands, action_state: Res<ActionState<UiAction>>) {
        if action_state.just_pressed(UiAction::Back) {
            commands.init_resource::<InGameMenu>();
        }
    }

    fn close_ingame_menu(mut commands: Commands) {
        commands.remove_resource::<InGameMenu>();
    }

    fn ingame_menu_system(
        mut commands: Commands,
        mut save_events: EventWriter<GameSaved>,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
    ) {
        let mut open = true;
        ModalWindow::new("Menu")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("Save").clicked() {
                        save_events.send_default();
                        ui.close_modal();
                    }
                    if ui.button("Save as...").clicked() {
                        commands.init_resource::<SaveAsDialog>();
                    }
                    if ui.button("Settings").clicked() {
                        commands.init_resource::<SettingsMenu>();
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
        mut save_events: EventWriter<GameSaved>,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut game_world: ResMut<GameWorld>,
        mut dialog: ResMut<SaveAsDialog>,
    ) {
        let mut open = true;
        ModalWindow::new("Save as...")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.text_edit_singleline(&mut dialog.world_name);
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        game_world.world_name = mem::take(&mut dialog.world_name);
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
        mut save_events: ResMut<Events<GameSaved>>,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
    ) {
        let mut open = true;
        ModalWindow::new("Exit to main menu")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label("Would you like to save your world before exiting to the main menu?");
                ui.horizontal(|ui| {
                    if ui.button("Save and exit").clicked() {
                        save_events.send_default();
                        // Should be called to avoid the exclusive saving system react on the event twice.
                        // See https://github.com/IyesGames/iyes_loopless/issues/31
                        save_events.update();
                        commands.remove_resource::<GameWorld>();
                        commands.insert_resource(NextState(GameState::MainMenu));
                        ui.close_modal();
                    }
                    if ui.button("Exit to main menu").clicked() {
                        commands.remove_resource::<GameWorld>();
                        commands.insert_resource(NextState(GameState::MainMenu));
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
        mut save_events: EventWriter<GameSaved>,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut exit_events: EventWriter<AppExit>,
    ) {
        let mut open = true;
        ModalWindow::new("Exit game")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label("Are you sure you want to exit the game?");
                ui.horizontal(|ui| {
                    if ui.button("Save and exit").clicked() {
                        save_events.send_default();
                        exit_events.send_default();
                    }
                    if ui.button("Exit without saving").clicked() {
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

#[derive(Default)]
struct InGameMenu;

struct SaveAsDialog {
    world_name: String,
}

impl FromWorld for SaveAsDialog {
    fn from_world(world: &mut World) -> Self {
        SaveAsDialog {
            world_name: world.resource::<GameWorld>().world_name.clone(),
        }
    }
}

#[derive(Default)]
struct ExitToMainMenuDialog;

#[derive(Default)]
struct ExitGameDialog;
