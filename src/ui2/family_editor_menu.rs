use std::{fs, mem};

use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy_trait_query::One;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    theme::Theme,
    widget::{
        button::{ExclusiveButton, ImageButtonBundle, Pressed, TextButtonBundle},
        text_edit::TextEditBundle,
        ui_root::UiRoot,
        Dialog, DialogBundle, LabelBundle,
    },
};
use crate::core::{
    actor::{race::Race, ActorScene, FirstName, LastName, Sex},
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
        app.add_system(Self::setup_system.in_schedule(OnEnter(GameState::FamilyEditor)))
            .add_systems(
                (
                    Self::plus_button_system,
                    Self::actor_buttons_spawn_system,
                    Self::actor_buttons_despawn_system,
                    Self::actor_buttons_system,
                    Self::sex_buttons_system,
                    Self::first_name_edit_system,
                    Self::last_name_edit_system,
                    Self::family_menu_button_system,
                    Self::save_family_button_system.pipe(error::report),
                    Self::place_dialog_button_system,
                    Self::city_place_button_system,
                )
                    .in_set(OnUpdate(GameState::FamilyEditor)),
            );
    }
}

impl FamilyEditorMenuPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                setup_personality_node(parent, &theme);
                setup_actors_node(parent, &theme);
                setup_family_menu_buttons(parent, &theme);
            });
    }

    // TODO: use visibility instead of `SelectedActor`.
    fn plus_button_system(
        mut commands: Commands,
        buttons: Query<&Interaction, (Changed<Interaction>, With<PlusButton>)>,
        actors: Query<Entity, With<SelectedActor>>,
        families: Query<Entity, With<EditableFamily>>,
    ) {
        if let Ok(&interaction) = buttons.get_single() {
            if interaction == Interaction::Clicked {
                if let Ok(entity) = actors.get_single() {
                    commands.entity(entity).remove::<SelectedActor>();
                }

                commands.entity(families.single()).with_children(|parent| {
                    parent.spawn((EditableActorBundle::default(), SelectedActor));
                });
            }
        }
    }

    fn actor_buttons_spawn_system(
        mut commands: Commands,
        theme: Res<Theme>,
        actors: Query<Entity, Added<EditableActor>>,
        actor_nodes: Query<Entity, With<ActorsNode>>,
    ) {
        for entity in &actors {
            commands
                .entity(actor_nodes.single())
                .with_children(|parent| {
                    parent.spawn((
                        EditActor(entity),
                        ExclusiveButton,
                        Pressed(true),
                        ImageButtonBundle::placeholder(&theme),
                    ));
                });
        }
    }

    fn actor_buttons_despawn_system(
        mut commands: Commands,
        mut removed_actors: RemovedComponents<EditableActor>,
        buttons: Query<(Entity, &EditActor)>,
    ) {
        for actor_entity in &mut removed_actors {
            let (button_entity, _) = buttons
                .iter()
                .find(|(_, edit_actor)| edit_actor.0 == actor_entity)
                .expect("each actor should have a corresponding button");
            commands.entity(button_entity).despawn_recursive();
        }
    }

    fn actor_buttons_system(
        mut commands: Commands,
        buttons: Query<(&Interaction, &EditActor), Changed<Interaction>>,
        actors: Query<Entity, With<SelectedActor>>,
    ) {
        for (&interaction, edit_actor) in &buttons {
            if interaction == Interaction::Clicked && actors.get(edit_actor.0).is_err() {
                commands.entity(actors.single()).remove::<SelectedActor>();
                commands.entity(edit_actor.0).insert(SelectedActor);
            }
        }
    }

    fn sex_buttons_system(
        buttons: Query<(&Interaction, &Sex), (Changed<Interaction>, Without<SelectedActor>)>,
        mut actors: Query<&mut Sex, With<SelectedActor>>,
    ) {
        for (&interaction, &sex) in &buttons {
            if interaction == Interaction::Clicked {
                *actors.single_mut() = sex;
            }
        }
    }

    fn first_name_edit_system(
        text_edits: Query<&Text, (Changed<Text>, With<FirstNameEdit>)>,
        mut actors: Query<&mut FirstName, With<SelectedActor>>,
    ) {
        for text in &text_edits {
            actors.single_mut().0.clone_from(&text.sections[0].value);
        }
    }

    fn last_name_edit_system(
        text_edits: Query<&Text, (Changed<Text>, With<LastNameEdit>)>,
        mut actors: Query<&mut LastName, With<SelectedActor>>,
    ) {
        for text in &text_edits {
            actors.single_mut().0.clone_from(&text.sections[0].value);
        }
    }

    fn family_menu_button_system(
        mut commands: Commands,
        mut game_state: ResMut<NextState<GameState>>,
        theme: Res<Theme>,
        buttons: Query<(&Interaction, &FamilyMenuButton), Changed<Interaction>>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for (&interaction, button) in &buttons {
            if interaction == Interaction::Clicked {
                match button {
                    FamilyMenuButton::Confirm => {
                        setup_save_family_dialog(&mut commands, roots.single(), &theme);
                    }
                    FamilyMenuButton::Cancel => game_state.set(GameState::World),
                }
            }
        }
    }

    fn save_family_button_system(
        mut commands: Commands,
        game_paths: Res<GamePaths>,
        theme: Res<Theme>,
        mut actors: Query<
            (&mut FirstName, &mut LastName, &Sex, One<&dyn Race>),
            With<EditableActor>,
        >,
        mut text_edits: Query<&mut Text, With<FamilyNameEdit>>,
        buttons: Query<(&Interaction, &SaveDialogButton), Changed<Interaction>>,
        dialogs: Query<Entity, With<Dialog>>,
        cities: Query<(Entity, &Name), With<City>>,
        roots: Query<Entity, With<UiRoot>>,
    ) -> Result<()> {
        for (&interaction, &button) in &buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            if button == SaveDialogButton::Save {
                let mut actor_scenes = Vec::new();
                for (mut first_name, mut last_name, &sex, race) in &mut actors {
                    actor_scenes.push(ActorScene {
                        first_name: mem::take(&mut first_name),
                        last_name: mem::take(&mut last_name),
                        sex,
                        race_name: race.type_name().to_string(),
                    })
                }
                let family_scene = FamilyScene::new(
                    mem::take(&mut text_edits.single_mut().sections[0].value).into(),
                    actor_scenes,
                );

                fs::create_dir_all(&game_paths.families)
                    .with_context(|| format!("unable to create {:?}", game_paths.families))?;

                let ron = ron::to_string(&family_scene).expect("unable to serialize family scene");
                let family_path = game_paths.family_path(&family_scene.name);
                fs::write(&family_path, ron)
                    .with_context(|| format!("unable to save game to {family_path:?}"))?;

                setup_place_family_dialog(
                    &mut commands,
                    roots.single(),
                    family_scene,
                    &theme,
                    &cities,
                );
            }

            commands.entity(dialogs.single()).despawn_recursive();
        }

        Ok(())
    }

    fn place_dialog_button_system(
        mut commands: Commands,
        mut reset_events: EventWriter<FamilyReset>,
        dialogs: Query<Entity, With<Dialog>>,
        buttons: Query<(&Interaction, &PlaceDialogButton), Changed<Interaction>>,
    ) {
        for (&interaction, &button) in &buttons {
            if interaction == Interaction::Clicked {
                if button == PlaceDialogButton::CreateNew {
                    reset_events.send_default();
                }
                commands.entity(dialogs.single()).despawn_recursive()
            }
        }
    }

    fn city_place_button_system(
        mut commands: Commands,
        mut spawn_events: EventWriter<FamilySpawn>,
        mut reset_events: EventWriter<FamilyReset>,
        buttons: Query<(&Interaction, &CityPlaceButton, &PlaceCity), Changed<Interaction>>,
        mut dialogs: Query<(Entity, &mut FamilyScene)>,
    ) {
        for (&interaction, button, place_city) in &buttons {
            if interaction == Interaction::Clicked {
                let (dialog_entity, mut scene) = dialogs.single_mut();
                match button {
                    CityPlaceButton::PlaceAndPlay => {
                        spawn_events.send(FamilySpawn {
                            city_entity: place_city.0,
                            scene: mem::take(&mut scene),
                            select: true,
                        });
                    }
                    CityPlaceButton::Place => {
                        spawn_events.send(FamilySpawn {
                            city_entity: place_city.0,
                            scene: mem::take(&mut scene),
                            select: false,
                        });
                        commands.entity(dialog_entity).despawn_recursive();
                        reset_events.send_default();
                    }
                }
            }
        }
    }
}

fn setup_personality_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(30.0), Val::Percent(25.0)),
                flex_direction: FlexDirection::Column,
                gap: theme.gap.normal,
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            // TODO 0.11: Use grid layout
            parent
                .spawn(NodeBundle {
                    style: Style {
                        gap: theme.gap.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    const GRID_GAP: Size = Size::all(Val::Px(10.0));
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                gap: GRID_GAP,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn(LabelBundle::normal(theme, "First name"));
                            parent.spawn(LabelBundle::normal(theme, "Last name"));
                        });
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                gap: theme.gap.normal,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn((FirstNameEdit, TextEditBundle::empty(theme)));
                            parent.spawn((LastNameEdit, TextEditBundle::empty(theme)));
                        });
                });

            parent.spawn(NodeBundle::default()).with_children(|parent| {
                for (index, sex) in Sex::iter().enumerate() {
                    parent.spawn((
                        sex,
                        ExclusiveButton,
                        Pressed(index == 0),
                        TextButtonBundle::normal(theme, sex.to_string()),
                    ));
                }
            });
        });
}

fn setup_actors_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                position: UiRect::new(Val::Px(0.0), Val::Undefined, Val::Undefined, Val::Px(0.0)),
                gap: theme.gap.normal,
                padding: theme.padding.global,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn((
                ActorsNode,
                NodeBundle {
                    style: Style {
                        gap: theme.gap.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));
            parent.spawn((PlusButton, TextButtonBundle::square(theme, "➕")));
        });
}

fn setup_family_menu_buttons(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect::new(Val::Undefined, Val::Px(0.0), Val::Undefined, Val::Px(0.0)),
                gap: theme.gap.normal,
                padding: theme.padding.global,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for button in FamilyMenuButton::iter() {
                parent.spawn((button, TextButtonBundle::normal(theme, button.to_string())));
            }
        });
}

fn setup_save_family_dialog(commands: &mut Commands, root_entity: Entity, theme: &Theme) {
    commands.entity(root_entity).with_children(|parent| {
        parent
            .spawn(DialogBundle::new(theme))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(50.0), Val::Percent(25.0)),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            gap: theme.gap.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent.spawn(LabelBundle::normal(theme, "Save family"));
                        parent.spawn((
                            FamilyNameEdit,
                            TextEditBundle::new(theme, "New family").active(),
                        ));
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                for dialog_button in SaveDialogButton::iter() {
                                    parent.spawn((
                                        dialog_button,
                                        TextButtonBundle::normal(theme, dialog_button.to_string()),
                                    ));
                                }
                            });
                    });
            });
    });
}

fn setup_place_family_dialog(
    commands: &mut Commands,
    root_entity: Entity,
    family_scene: FamilyScene,
    theme: &Theme,
    cities: &Query<(Entity, &Name), With<City>>,
) {
    commands.entity(root_entity).with_children(|parent| {
        parent
            .spawn((family_scene, DialogBundle::new(theme)))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(40.0), Val::Percent(90.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            gap: theme.gap.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent.spawn(LabelBundle::normal(theme, "Place family"));
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    size: Size::all(Val::Percent(100.0)),
                                    justify_content: JustifyContent::Center,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                // TODO: Use combobox.
                                for (entity, name) in cities {
                                    parent
                                        .spawn(NodeBundle {
                                            style: Style {
                                                gap: theme.gap.normal,
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        })
                                        .with_children(|parent| {
                                            parent.spawn(LabelBundle::normal(theme, name));
                                            for button in CityPlaceButton::iter() {
                                                parent.spawn((
                                                    button,
                                                    PlaceCity(entity),
                                                    TextButtonBundle::normal(
                                                        theme,
                                                        button.to_string(),
                                                    ),
                                                ));
                                            }
                                        });
                                }
                            });

                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                for button in PlaceDialogButton::iter() {
                                    parent.spawn((
                                        button,
                                        TextButtonBundle::normal(theme, button.to_string()),
                                    ));
                                }
                            });
                    });
            });
    });
}

#[derive(Component)]
struct FirstNameEdit;

#[derive(Component)]
struct LastNameEdit;

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum FamilyMenuButton {
    Confirm,
    Cancel,
}

#[derive(Component, EnumIter, Clone, Copy, Display, PartialEq)]
enum SaveDialogButton {
    Save,
    Cancel,
}

#[derive(Component)]
struct PlusButton;

#[derive(Component)]
struct ActorsNode;

#[derive(Component)]
struct EditActor(Entity);

#[derive(Component)]
struct FamilyNameEdit;

#[derive(Component, EnumIter, Clone, Copy, Display, PartialEq)]
enum PlaceDialogButton {
    Cancel,
    #[strum(serialize = "Create new")]
    CreateNew,
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum CityPlaceButton {
    Place,
    #[strum(serialize = "Place & play")]
    PlaceAndPlay,
}

#[derive(Component)]
struct PlaceCity(Entity);
