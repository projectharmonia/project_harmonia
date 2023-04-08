use std::{fs, mem};

use anyhow::{ensure, Context, Result};
use bevy::prelude::*;
use bevy_egui::{
    egui::{
        epaint::WHITE_UV, Align, Align2, Area, Image, ImageButton, Layout, TextEdit, TextureId,
        Window,
    },
    EguiContexts,
};
use bevy_inspector_egui::egui::Button;
use derive_more::Constructor;
use leafwing_input_manager::prelude::ActionState;
use strum::IntoEnumIterator;

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    UI_MARGIN,
};
use crate::core::{
    action::Action,
    actor::{ActorBundle, FirstName, LastName, Sex},
    city::City,
    error,
    family::{FamilyScene, FamilySpawn},
    family_editor::{
        EditableActor, EditableActorBundle, EditableFamily, FamilyReset, SelectedActor,
    },
    game_paths::GamePaths,
    game_state::GameState,
};

pub(super) struct FamilyEditorMenuPlugin;

impl Plugin for FamilyEditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                Self::personality_window_system,
                Self::actors_panel_system,
                Self::buttons_system.pipe(error::report),
                Self::save_family_dialog_system
                    .pipe(error::report)
                    .run_if(resource_exists::<SaveFamilyDialog>()),
                Self::place_family_dialog_system.run_if(resource_exists::<PlaceFamilyDialog>()),
            )
                .in_set(OnUpdate(GameState::FamilyEditor)),
        );
    }
}

impl FamilyEditorMenuPlugin {
    fn personality_window_system(
        mut egui: EguiContexts,
        mut selected_actors: Query<(&mut FirstName, &mut LastName, &mut Sex), With<SelectedActor>>,
    ) {
        let Ok((mut first_name, mut last_name, mut sex)) = selected_actors.get_single_mut() else {
            return;
        };

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
                ui.horizontal(|ui| {
                    for sex_variant in Sex::iter() {
                        if ui
                            .selectable_value(
                                sex.bypass_change_detection(),
                                sex_variant,
                                sex_variant.glyph(),
                            )
                            .changed()
                        {
                            sex.set_changed();
                        }
                    }
                });
            });
    }

    fn actors_panel_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        editable_families: Query<Entity, With<EditableFamily>>,
        editable_actors: Query<Entity, With<EditableActor>>,
        selected_actors: Query<Entity, With<SelectedActor>>,
    ) {
        Window::new("Actors")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    let mut editable_actors = editable_actors.iter().collect::<Vec<_>>();
                    editable_actors.sort(); // To preserve the order, it changes when we insert or remove `SelectedActor`.
                    let selected_entity = selected_actors.get_single();
                    for actor_entity in editable_actors {
                        if ui
                            .add(
                                ImageButton::new(TextureId::Managed(0), (64.0, 64.0))
                                    .uv([WHITE_UV, WHITE_UV]).selected(matches!(selected_entity, Ok(selected_entity) if selected_entity == actor_entity)),
                            )
                            .clicked()
                        {
                            if let Ok(selected_entity) = selected_entity {
                                commands.entity(selected_entity).remove::<SelectedActor>();
                            }
                            commands.entity(actor_entity).insert(SelectedActor);
                        }
                    }
                    if ui.button("➕").clicked() {
                        if let Ok(current_entity) = selected_entity {
                            commands.entity(current_entity).remove::<SelectedActor>();
                        }
                        commands.entity(editable_families.single()).with_children(|parent| {
                            parent.spawn((EditableActorBundle::default(), SelectedActor));
                        });
                    }
                });
            });
    }

    fn buttons_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut game_state: ResMut<NextState<GameState>>,
        editable_actors: Query<Entity, With<EditableActor>>,
        names: Query<(&FirstName, &LastName)>,
        selected_actors: Query<&LastName, With<SelectedActor>>,
    ) -> Result<()> {
        let mut confirmed = false;
        Area::new("Confrirm cancel")
            .anchor(Align2::RIGHT_BOTTOM, (-UI_MARGIN, -UI_MARGIN))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        game_state.set(GameState::World);
                    }
                    confirmed = ui.button("Confirm").clicked();
                });
            });

        if confirmed {
            for (index, entity) in editable_actors.iter().enumerate() {
                let (first_name, last_name) = names
                    .get(entity)
                    .expect("actor should have a first and a last name");
                ensure!(
                    !first_name.is_empty(),
                    "actor {index} do not have a first name"
                );
                ensure!(
                    !last_name.is_empty(),
                    "actor {index} do not have a last name"
                );
            }
            commands.insert_resource(SaveFamilyDialog::new(selected_actors.single().to_string()));
        }

        Ok(())
    }

    fn save_family_dialog_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut save_dialog: ResMut<SaveFamilyDialog>,
        mut action_state: ResMut<ActionState<Action>>,
        game_paths: Res<GamePaths>,
        editable_actors: Query<(&FirstName, &LastName, &Sex), With<EditableActor>>,
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
                let mut actor_bundles = Vec::new();
                for (first_name, last_name, &sex) in &editable_actors {
                    actor_bundles.push(ActorBundle {
                        first_name: first_name.clone(),
                        last_name: last_name.clone(),
                        sex,
                    })
                }
                let family_scene =
                    FamilyScene::new(mem::take(&mut save_dialog.family_name), actor_bundles);

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
        mut egui: EguiContexts,
        mut reset_events: EventWriter<FamilyReset>,
        mut spawn_events: EventWriter<FamilySpawn>,
        mut action_state: ResMut<ActionState<Action>>,
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
                                let select = ui.button("⏵ Place and play").clicked();
                                if ui.button("⬇ Place").clicked() || select {
                                    spawn_events.send(FamilySpawn {
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

#[derive(Resource, Constructor)]
struct SaveFamilyDialog {
    family_name: String,
}

#[derive(Resource)]
struct PlaceFamilyDialog(FamilyScene);
