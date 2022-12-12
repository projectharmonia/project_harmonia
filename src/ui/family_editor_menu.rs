use std::{fs, mem};

use anyhow::{ensure, Context, Result};
use bevy::prelude::*;
use bevy_egui::{
    egui::{
        epaint::WHITE_UV, Align, Align2, Area, Image, ImageButton, Layout, TextEdit, TextureId,
        Window,
    },
    EguiContext,
};
use bevy_inspector_egui::egui::Button;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    UI_MARGIN,
};
use crate::core::{
    action::Action,
    city::City,
    doll::{ActiveDoll, DollScene, FirstName, LastName},
    error_message,
    family::{FamilyScene, FamilySpawn},
    family_editor::{EditableDoll, EditableDollBundle, EditableFamily, FamilyReset},
    game_paths::GamePaths,
    game_state::GameState,
    network::network_event::client_event::ClientSendBuffer,
};

pub(super) struct FamilyEditorMenuPlugin;

impl Plugin for FamilyEditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::personality_window_system.run_in_state(GameState::FamilyEditor))
            .add_system(Self::dolls_panel_system.run_in_state(GameState::FamilyEditor))
            .add_system(
                Self::buttons_system
                    .pipe(error_message::err_message_system)
                    .run_in_state(GameState::FamilyEditor),
            )
            .add_system(
                Self::save_family_dialog_system
                    .pipe(error_message::err_message_system)
                    .run_if_resource_exists::<SaveFamilyDialog>(),
            )
            .add_system(
                Self::place_family_dialog_system.run_if_resource_exists::<PlaceFamilyDialog>(),
            );
    }
}

impl FamilyEditorMenuPlugin {
    fn personality_window_system(
        mut egui: ResMut<EguiContext>,
        mut active_dolls: Query<(&mut FirstName, &mut LastName), With<ActiveDoll>>,
    ) {
        if let Ok((mut first_name, mut last_name)) = active_dolls.get_single_mut() {
            Window::new("Personality")
                .anchor(Align2::LEFT_TOP, (0.0, 0.0))
                .resizable(false)
                .show(egui.ctx_mut(), |ui| {
                    if ui
                        .add(
                            TextEdit::singleline(&mut first_name.bypass_change_detection().0)
                                .hint_text("First name"),
                        )
                        .changed()
                    {
                        first_name.set_changed();
                    }
                    if ui
                        .add(
                            TextEdit::singleline(&mut last_name.bypass_change_detection().0)
                                .hint_text("Last name"),
                        )
                        .changed()
                    {
                        last_name.set_changed();
                    }
                });
        }
    }

    fn dolls_panel_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        editable_families: Query<Entity, With<EditableFamily>>,
        editable_dolls: Query<Entity, With<EditableDoll>>,
        active_dolls: Query<Entity, With<ActiveDoll>>,
    ) {
        Window::new("Dolls")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    let active_entity = active_dolls.get_single();
                    for doll_entity in &editable_dolls {
                        if ui
                            .add(
                                ImageButton::new(TextureId::Managed(0), (64.0, 64.0))
                                    .uv([WHITE_UV, WHITE_UV]).selected(matches!(active_entity, Ok(current_doll) if doll_entity == current_doll)),
                            )
                            .clicked()
                        {
                            if let Ok(current_entity) = active_entity {
                                commands.entity(current_entity).remove::<ActiveDoll>();
                            }
                            commands.entity(doll_entity).insert(ActiveDoll);
                        }
                    }
                    if ui.button("➕").clicked() {
                        if let Ok(current_entity) = active_entity {
                            commands.entity(current_entity).remove::<ActiveDoll>();
                        }
                        commands.entity(editable_families.single()).with_children(|parent| {
                            parent.spawn((EditableDollBundle::default(), ActiveDoll));
                        });
                    }
                });
            });
    }

    fn buttons_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        editable_dolls: Query<Entity, With<EditableDoll>>,
        names: Query<(&FirstName, &LastName)>,
    ) -> Result<()> {
        let mut confirmed = false;
        Area::new("Confrirm cancel")
            .anchor(Align2::RIGHT_BOTTOM, (-UI_MARGIN, -UI_MARGIN))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        commands.insert_resource(NextState(GameState::World));
                    }
                    confirmed = ui.button("Confirm").clicked();
                });
            });

        if confirmed {
            for (index, entity) in editable_dolls.iter().enumerate() {
                let (first_name, last_name) = names
                    .get(entity)
                    .expect("doll should have a first and a last name");
                ensure!(
                    !first_name.is_empty(),
                    "doll {index} do not have a first name"
                );
                ensure!(
                    !last_name.is_empty(),
                    "doll {index} do not have a last name"
                );
            }
            commands.init_resource::<SaveFamilyDialog>();
        }

        Ok(())
    }

    fn save_family_dialog_system(
        mut commands: Commands,
        mut save_dialog: ResMut<SaveFamilyDialog>,
        mut action_state: ResMut<ActionState<Action>>,
        mut egui: ResMut<EguiContext>,
        game_paths: Res<GamePaths>,
        editable_dolls: Query<(&FirstName, &LastName), With<EditableDoll>>,
    ) -> Result<()> {
        let mut confirmed = false;
        let mut open = true;
        ModalWindow::new("Save family")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.text_edit_singleline(&mut save_dialog.family_name);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !save_dialog.family_name.is_empty(),
                            Button::new("Save to library"),
                        )
                        .clicked()
                    {
                        confirmed = true;
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !open {
            commands.remove_resource::<SaveFamilyDialog>();

            if confirmed {
                let mut dolls = Vec::new();
                for (first_name, last_name) in &editable_dolls {
                    dolls.push(DollScene {
                        first_name: first_name.clone(),
                        last_name: last_name.clone(),
                    })
                }
                let family_scene = FamilyScene::new(mem::take(&mut save_dialog.family_name), dolls);

                fs::create_dir_all(&game_paths.families)
                    .with_context(|| format!("unable to create {:?}", game_paths.families))?;

                let ron = ron::to_string(&family_scene).expect("unable to serialize family scene");
                let family_path = game_paths.family_path(&family_scene.name);
                fs::write(&family_path, ron)
                    .with_context(|| format!("unable to save game to {family_path:?}"))?;

                commands.insert_resource(PlaceFamilyDialog(family_scene));
            }
        }

        Ok(())
    }

    fn place_family_dialog_system(
        mut commands: Commands,
        mut reset_events: EventWriter<FamilyReset>,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<Action>>,
        mut spawn_buffer: ResMut<ClientSendBuffer<FamilySpawn>>,
        mut place_dialog: ResMut<PlaceFamilyDialog>,
        cities: Query<(Entity, &Name), With<City>>,
    ) {
        let mut open = true;
        ModalWindow::new("Place family")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                for (city_entity, name) in &cities {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                Image::new(TextureId::Managed(0), (64.0, 64.0))
                                    .uv([WHITE_UV, WHITE_UV]),
                            );
                            ui.label(name.as_str());
                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                let mut select = false;
                                if ui.button("⏵ Place and play").clicked() {
                                    select = true;
                                }
                                if ui.button("⬇ Place").clicked() || select {
                                    spawn_buffer.push(FamilySpawn {
                                        city_entity,
                                        scene: mem::take(&mut place_dialog.0),
                                        select,
                                    });
                                    if !select {
                                        reset_events.send_default();
                                    }
                                    ui.close_modal();
                                }
                            })
                        });
                    });
                }
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui.button("➕ Create another").clicked() {
                            reset_events.send_default();
                            ui.close_modal();
                        }
                    });
                });
            });

        if !open {
            commands.remove_resource::<PlaceFamilyDialog>();
        }
    }
}

#[derive(Resource)]
struct SaveFamilyDialog {
    family_name: String,
}

impl FromWorld for SaveFamilyDialog {
    fn from_world(world: &mut World) -> Self {
        let last_name = world
            .query_filtered::<&LastName, With<EditableDoll>>()
            .single(world);

        Self {
            family_name: last_name.to_string(),
        }
    }
}

#[derive(Resource)]
struct PlaceFamilyDialog(FamilyScene);
