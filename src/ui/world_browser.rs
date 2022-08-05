use std::{fs, path::PathBuf};

use bevy::prelude::*;
use bevy_egui::{
    egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId},
    EguiContext,
};
use bevy_inspector_egui::egui::Button;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{modal_window::ModalWindow, ui_action::UiAction};
use crate::core::{game_paths::GamePaths, game_state::GameState, game_world::WorldName};

pub(super) struct WorldBrowserPlugin;

impl Plugin for WorldBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_exit_system(GameState::Menu, Self::close_world_browser)
            .add_system(Self::world_browser_system.run_if_resource_exists::<WorldBrowser>())
            .add_system(
                Self::create_world_system
                    .run_in_state(GameState::Menu)
                    .run_if_resource_exists::<WorldName>(),
            );
    }
}

impl WorldBrowserPlugin {
    fn close_world_browser(mut commands: Commands) {
        commands.remove_resource::<WorldBrowser>();
    }

    fn world_browser_system(
        mut commands: Commands,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut egui: ResMut<EguiContext>,
        world_browser: Res<WorldBrowser>,
    ) {
        let mut is_open = true;
        ModalWindow::new(&mut is_open, &mut action_state, "World browser").show(
            egui.ctx_mut(),
            |ui| {
                for world in &world_browser.worlds {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                Image::new(TextureId::Managed(0), (64.0, 64.0))
                                    .uv([WHITE_UV, WHITE_UV]),
                            );
                            ui.label(world.to_string_lossy().to_string());
                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                if ui.button("⏵ Play").clicked() {}
                                if ui.button("👥 Host").clicked() {}
                                if ui.button("🗑 Delete").clicked() {}
                            })
                        });
                    });
                }
                ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                    if ui.button("➕ Create new").clicked() {
                        commands.init_resource::<WorldName>();
                    }
                });
            },
        );

        if !is_open {
            commands.remove_resource::<WorldBrowser>();
        }
    }

    fn create_world_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut world_name: ResMut<WorldName>,
    ) {
        let mut is_open = true;
        ModalWindow::new(&mut is_open, &mut action_state, "Create world").show(
            egui.ctx_mut(),
            |ui| {
                ui.text_edit_singleline(&mut world_name.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!world_name.is_empty(), Button::new("Create"))
                        .clicked()
                    {
                        commands.insert_resource(NextState(GameState::InGame));
                    }
                    if ui.button("Cancel").clicked() {
                        commands.remove_resource::<WorldName>();
                    }
                });
            },
        );

        if !is_open {
            commands.remove_resource::<WorldName>();
        }
    }
}

pub(super) struct WorldBrowser {
    worlds: Vec<PathBuf>,
}

impl FromWorld for WorldBrowser {
    fn from_world(world: &mut World) -> Self {
        let mut worlds = Vec::new();
        let game_paths = world.resource::<GamePaths>();
        if let Ok(entries) = fs::read_dir(&game_paths.worlds) {
            for entry in entries.filter_map(Result::ok) {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        worlds.push(entry.path());
                    }
                }
            }
        };

        worlds.push("Test".into());
        Self { worlds }
    }
}

#[derive(Default)]
struct CreateWorldDialog;
