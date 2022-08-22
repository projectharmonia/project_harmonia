use std::{fs, mem};

use bevy::prelude::*;
use bevy_egui::{
    egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId},
    EguiContext,
};
use bevy_inspector_egui::egui::Button;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    ui_action::UiAction,
};
use crate::core::{
    game_paths::GamePaths,
    game_state::GameState,
    game_world::{GameLoaded, GameWorld, GameWorldSystem},
};

pub(super) struct WorldBrowserPlugin;

impl Plugin for WorldBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_exit_system(GameState::MainMenu, Self::close_world_browser)
            .add_system(
                Self::world_browser_system
                    .run_if_resource_exists::<WorldBrowser>()
                    .after(GameWorldSystem::Loading),
            )
            .add_system(Self::create_world_system.run_if_resource_exists::<CreateWorldDialog>())
            .add_system(Self::remove_world_system.run_if_resource_exists::<RemoveWorldDialog>());
    }
}

impl WorldBrowserPlugin {
    fn close_world_browser(mut commands: Commands) {
        commands.remove_resource::<WorldBrowser>();
    }

    fn world_browser_system(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoaded>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut egui: ResMut<EguiContext>,
        mut world_browser: ResMut<WorldBrowser>,
    ) {
        let mut is_open = true;
        ModalWindow::new("World browser")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                for (index, world) in world_browser.worlds.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                Image::new(TextureId::Managed(0), (64.0, 64.0))
                                    .uv([WHITE_UV, WHITE_UV]),
                            );
                            ui.label(world.as_str());
                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                if ui.button("⏵ Play").clicked() {
                                    commands.insert_resource(GameWorld::new(mem::take(world)));
                                    commands.insert_resource(NextState(GameState::World));
                                    load_events.send_default();
                                }
                                if ui.button("👥 Host").clicked() {}
                                if ui.button("🗑 Delete").clicked() {
                                    commands.insert_resource(RemoveWorldDialog::new(index));
                                }
                            })
                        });
                    });
                }
                ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                    if ui.button("➕ Create new").clicked() {
                        commands.init_resource::<CreateWorldDialog>();
                    }
                });
            });

        if !is_open {
            commands.remove_resource::<WorldBrowser>();
        }
    }

    fn create_world_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut dialog: ResMut<CreateWorldDialog>,
    ) {
        let mut is_open = true;
        ModalWindow::new("Create world")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.text_edit_singleline(&mut dialog.world_name);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!dialog.world_name.is_empty(), Button::new("Create"))
                        .clicked()
                    {
                        commands.insert_resource(GameWorld::new(mem::take(&mut dialog.world_name)));
                        commands.insert_resource(NextState(GameState::World));
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !is_open {
            commands.remove_resource::<CreateWorldDialog>();
        }
    }

    fn remove_world_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut world_browser: ResMut<WorldBrowser>,
        game_paths: Res<GamePaths>,
        dialog: ResMut<RemoveWorldDialog>,
    ) {
        let mut is_open = true;
        ModalWindow::new("Remove world")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label(format!(
                    "Are you sure you want to remove world {}?",
                    &world_browser.worlds[dialog.world_index]
                ));
                ui.horizontal(|ui| {
                    if ui.button("Remove").clicked() {
                        let world = world_browser.worlds.remove(dialog.world_index);
                        fs::remove_file(game_paths.world_path(&world))
                            .map_err(|e| error!("{e:#}"))
                            .ok();
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !is_open {
            commands.remove_resource::<RemoveWorldDialog>();
        }
    }
}

pub(super) struct WorldBrowser {
    worlds: Vec<String>,
}

impl FromWorld for WorldBrowser {
    fn from_world(world: &mut World) -> Self {
        Self {
            worlds: world
                .resource::<GamePaths>()
                .get_world_names()
                .map_err(|e| error!("{e:#}"))
                .unwrap_or_default(),
        }
    }
}

#[derive(Default)]
struct CreateWorldDialog {
    world_name: String,
}

struct RemoveWorldDialog {
    world_index: usize,
}

impl RemoveWorldDialog {
    fn new(world_index: usize) -> Self {
        Self { world_index }
    }
}
