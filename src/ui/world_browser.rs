use std::{fs, mem};

use anyhow::{Context, Ok, Result};
use bevy::prelude::*;
use bevy_egui::{
    egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId},
    EguiContext,
};
use bevy_inspector_egui::egui::{Button, DragValue, Grid};
use derive_more::Constructor;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use tap::TapFallible;

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    ui_action::UiAction,
};
use crate::core::{
    error,
    game_paths::GamePaths,
    game_state::GameState,
    game_world::{GameLoaded, GameWorld, GameWorldSystem},
    network::{client::ConnectionSettings, server::ServerSettings},
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
            .add_system(
                Self::join_world_system
                    .chain(error::err_message_system)
                    .run_if_resource_exists::<JoinWorldDialog>(),
            )
            .add_system(
                Self::host_world_system
                    .chain(error::err_message_system)
                    .run_if_resource_exists::<HostWorldDialog>(),
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
                for (index, world_name) in world_browser.world_names.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                Image::new(TextureId::Managed(0), (64.0, 64.0))
                                    .uv([WHITE_UV, WHITE_UV]),
                            );
                            ui.label(world_name.as_str());
                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                if ui.button("⏵ Play").clicked() {
                                    commands.insert_resource(GameWorld::new(mem::take(world_name)));
                                    load_events.send_default();
                                }
                                if ui.button("👥 Host").clicked() {
                                    commands.insert_resource(HostWorldDialog::new(index));
                                }
                                if ui.button("🗑 Delete").clicked() {
                                    commands.insert_resource(RemoveWorldDialog::new(index));
                                }
                            })
                        });
                    });
                }
                ui.with_layout(Layout::left_to_right().with_cross_align(Align::Max), |ui| {
                    if ui.button("➕ Create new").clicked() {
                        commands.init_resource::<CreateWorldDialog>();
                    }
                    if ui.button("🖧 Join").clicked() {
                        commands.init_resource::<JoinWorldDialog>();
                    }
                });
            });

        if !is_open {
            commands.remove_resource::<WorldBrowser>();
        }
    }

    fn join_world_system(
        mut commands: Commands,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut egui: ResMut<EguiContext>,
        mut connection_settings: ResMut<ConnectionSettings>,
    ) -> Result<()> {
        let mut is_open = true;
        let mut is_confirmed = false;
        ModalWindow::new("Join world")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                Grid::new("Connection settings grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("IP:");
                        ui.text_edit_singleline(&mut connection_settings.ip);
                        ui.end_row();
                        ui.label("Port:");
                        ui.add(DragValue::new(&mut connection_settings.port));
                        ui.end_row();
                    });
                if ui.button("Join").clicked() {
                    is_confirmed = true;
                    ui.close_modal();
                }
            });

        if !is_open {
            commands.remove_resource::<JoinWorldDialog>();

            if is_confirmed {
                let client = connection_settings
                    .create_client()
                    .context("unable to create connection")?;
                commands.insert_resource(client);
            }
        }

        Ok(())
    }

    fn host_world_system(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoaded>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut egui: ResMut<EguiContext>,
        mut world_browser: ResMut<WorldBrowser>,
        mut server_settings: ResMut<ServerSettings>,
        dialog: Res<HostWorldDialog>,
    ) -> Result<()> {
        let mut is_open = true;
        let mut is_confirmed = false;
        ModalWindow::new("Host world")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                Grid::new("Server settings grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Server name:");
                        ui.text_edit_singleline(&mut server_settings.server_name);
                        ui.end_row();
                        ui.label("Port:");
                        ui.add(DragValue::new(&mut server_settings.port));
                        ui.end_row();
                    });
                if ui.button("Host").clicked() {
                    is_confirmed = true;
                    ui.close_modal();
                }
            });

        if !is_open {
            commands.remove_resource::<HostWorldDialog>();

            if is_confirmed {
                let world_name = world_browser.world_names.remove(dialog.world_index);
                commands.insert_resource(GameWorld::new(world_name));
                load_events.send_default();
                let server = server_settings
                    .create_server()
                    .context("unable to create server")?;
                commands.insert_resource(server);
            }
        }

        Ok(())
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
        dialog: Res<RemoveWorldDialog>,
    ) {
        let mut is_open = true;
        ModalWindow::new("Remove world")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label(format!(
                    "Are you sure you want to remove world {}?",
                    &world_browser.world_names[dialog.world_index]
                ));
                ui.horizontal(|ui| {
                    if ui.button("Remove").clicked() {
                        let world = world_browser.world_names.remove(dialog.world_index);
                        let world_path = game_paths.world_path(&world);
                        fs::remove_file(&world_path)
                            .tap_err(|e| error!("unable to remove {world_path:?}: {e}"))
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
    world_names: Vec<String>,
}

impl FromWorld for WorldBrowser {
    fn from_world(world: &mut World) -> Self {
        Self {
            world_names: world
                .resource::<GamePaths>()
                .get_world_names()
                .tap_err(|e| error!("unable to get world names: {e}"))
                .unwrap_or_default(),
        }
    }
}

#[derive(Default)]
struct JoinWorldDialog;

#[derive(Constructor)]
struct HostWorldDialog {
    world_index: usize,
}

#[derive(Default)]
struct CreateWorldDialog {
    world_name: String,
}

#[derive(Constructor)]
struct RemoveWorldDialog {
    world_index: usize,
}
