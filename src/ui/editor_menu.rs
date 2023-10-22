use std::mem;

use anyhow::Result;
use bevy::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    preview::{Preview, PreviewProcessed},
    theme::Theme,
    widget::{
        button::{ExclusiveButton, ImageButtonBundle, TextButtonBundle, Toggled},
        click::Click,
        text_edit::{ActiveEdit, TextEditBundle},
        ui_root::UiRoot,
        Dialog, DialogBundle, LabelBundle,
    },
};
use crate::core::{
    actor::{FirstName, LastName, Sex},
    city::City,
    error_report,
    family::{
        editor::{EditableActor, EditableActorBundle, EditableFamily, FamilyReset},
        FamilyScene, FamilySpawn,
    },
    game_state::GameState,
};

pub(super) struct EditorMenuPlugin;

impl Plugin for EditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::FamilyEditor), Self::setup_system)
            .add_systems(
                Update,
                (
                    Self::plus_button_system,
                    Self::actor_buttons_spawn_system,
                    Self::actor_buttons_update_system,
                    Self::actor_buttons_despawn_system,
                    (
                        Self::actor_buttons_system,
                        (
                            Self::sex_buttons_system,
                            Self::first_name_edit_system,
                            Self::last_name_edit_system,
                        ),
                    )
                        .chain(),
                    Self::family_menu_button_system,
                    Self::save_family_button_system.pipe(error_report::report),
                    Self::place_dialog_button_system,
                    Self::city_place_button_system,
                )
                    .run_if(in_state(GameState::FamilyEditor)),
            );
    }
}

impl EditorMenuPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                UiRoot,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                setup_personality_node(parent, &theme);
                setup_actors_node(parent, &theme);
                setup_family_menu_buttons(parent, &theme);
            });
    }

    fn plus_button_system(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<(), With<PlusButton>>,
        families: Query<Entity, With<EditableFamily>>,
    ) {
        for event in &mut click_events {
            if buttons.get(event.0).is_ok() {
                commands.entity(families.single()).with_children(|parent| {
                    parent.spawn(EditableActorBundle::default());
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
                        Preview::actor(entity, &theme.button.image),
                        ExclusiveButton,
                        Toggled(true),
                        ImageButtonBundle::placeholder(&theme),
                    ));
                });
        }
    }

    fn actor_buttons_update_system(
        mut commands: Commands,
        actors: Query<(Entity, Ref<Sex>), With<EditableActor>>,
        buttons: Query<(Entity, &EditActor)>,
    ) {
        for (actor_entity, _) in actors
            .iter()
            .filter(|(_, sex)| sex.is_changed() && !sex.is_added())
        {
            let (button_entity, _) = buttons
                .iter()
                .find(|(_, edit_actor)| edit_actor.0 == actor_entity)
                .expect("each actor should have a corresponding button");
            commands.entity(button_entity).remove::<PreviewProcessed>();
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
        actor_buttons: Query<(&Toggled, &EditActor), Changed<Toggled>>,
        mut actors: Query<(&mut Visibility, &Sex, &FirstName, &LastName), With<EditableActor>>,
        mut sex_buttons: Query<(&mut Toggled, &Sex), Without<EditActor>>,
        mut first_name_edits: Query<&mut Text, With<FirstNameEdit>>,
        mut last_name_edits: Query<&mut Text, (With<LastNameEdit>, Without<FirstNameEdit>)>,
    ) {
        for (actor_toggled, edit_actor) in &actor_buttons {
            if actor_toggled.0 {
                // Hide previous.
                if let Some((mut visibility, ..)) = actors
                    .iter_mut()
                    .find(|(visibility, ..)| **visibility == Visibility::Visible)
                {
                    *visibility = Visibility::Hidden;
                }

                // Update UI with parameters of the current actor.
                let (mut visibility, &actor_sex, first_name, last_name) = actors
                    .get_mut(edit_actor.0)
                    .expect("actor button should point to a valid actor");
                *visibility = Visibility::Visible;
                first_name_edits.single_mut().sections[0]
                    .value
                    .clone_from(first_name);
                last_name_edits.single_mut().sections[0]
                    .value
                    .clone_from(last_name);

                let (mut sex_toggled, ..) = sex_buttons
                    .iter_mut()
                    .find(|(_, &sex)| sex == actor_sex)
                    .expect("sex buttons should be spawned for each variant");
                sex_toggled.0 = true;
            }
        }
    }

    fn sex_buttons_system(
        buttons: Query<(&Toggled, &Sex), (Changed<Toggled>, Without<EditableActor>)>,
        mut actors: Query<(&mut Sex, &Visibility), With<EditableActor>>,
    ) {
        for (toggled, &button_sex) in &buttons {
            if toggled.0 {
                if let Some((mut actor_sex, _)) = actors
                    .iter_mut()
                    .filter(|(visibility, _)| !visibility.is_changed()) // Avoid changes on actor switching.
                    .find(|(_, &visibility)| visibility == Visibility::Visible)
                {
                    *actor_sex = button_sex;
                }
            }
        }
    }

    fn first_name_edit_system(
        text_edits: Query<&Text, (Changed<Text>, With<FirstNameEdit>)>,
        mut actors: Query<(&mut FirstName, &Visibility), With<EditableActor>>,
    ) {
        for text in &text_edits {
            if let Some((mut first_name, _)) = actors
                .iter_mut()
                .filter(|(visibility, _)| !visibility.is_changed()) // Avoid changes on actor switching.
                .find(|(_, &visibility)| visibility == Visibility::Visible)
            {
                first_name.0.clone_from(&text.sections[0].value);
            }
        }
    }

    fn last_name_edit_system(
        text_edits: Query<&Text, (Changed<Text>, With<LastNameEdit>)>,
        mut actors: Query<(&mut LastName, &Visibility), With<EditableActor>>,
    ) {
        for text in &text_edits {
            if let Some((mut last_name, _)) = actors
                .iter_mut()
                .filter(|(visibility, _)| !visibility.is_changed()) // Avoid changes on actor switching.
                .find(|(_, &visibility)| visibility == Visibility::Visible)
            {
                last_name.0.clone_from(&text.sections[0].value);
            }
        }
    }

    fn family_menu_button_system(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        theme: Res<Theme>,
        buttons: Query<&FamilyMenuButton>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for event in &mut click_events {
            if let Ok(button) = buttons.get(event.0) {
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
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        mut text_edits: Query<&mut Text, With<FamilyNameEdit>>,
        buttons: Query<&SaveDialogButton>,
        dialogs: Query<Entity, With<Dialog>>,
        cities: Query<(Entity, &Name), With<City>>,
        roots: Query<Entity, With<UiRoot>>,
    ) -> Result<()> {
        for event in &mut click_events {
            let Ok(&button) = buttons.get(event.0) else {
                continue;
            };

            if button == SaveDialogButton::Save {
                let mut family_name = text_edits.single_mut();
                let family_scene =
                    FamilyScene::new(mem::take(&mut family_name.sections[0].value).into());

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
        mut click_events: EventReader<Click>,
        dialogs: Query<Entity, With<Dialog>>,
        buttons: Query<&PlaceDialogButton>,
    ) {
        for event in &mut click_events {
            if let Ok(&button) = buttons.get(event.0) {
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
        mut click_events: EventReader<Click>,
        buttons: Query<(&CityPlaceButton, &PlaceCity)>,
        mut dialogs: Query<(Entity, &mut FamilyScene)>,
    ) {
        for event in &mut click_events {
            if let Ok((button, place_city)) = buttons.get(event.0) {
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
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                padding: theme.padding.normal,
                row_gap: theme.gap.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        column_gap: theme.gap.normal,
                        row_gap: theme.gap.normal,
                        grid_template_columns: vec![GridTrack::auto(); 2],
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(LabelBundle::normal(theme, "First name"));
                    parent.spawn((FirstNameEdit, TextEditBundle::empty(theme)));
                    parent.spawn(LabelBundle::normal(theme, "Last name"));
                    parent.spawn((LastNameEdit, TextEditBundle::empty(theme)));
                });

            parent.spawn(NodeBundle::default()).with_children(|parent| {
                for sex in Sex::iter() {
                    parent.spawn((
                        sex,
                        ExclusiveButton,
                        Toggled(sex == Default::default()),
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
                align_self: AlignSelf::FlexEnd,
                column_gap: theme.gap.normal,
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
                        column_gap: theme.gap.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));
            parent.spawn((PlusButton, TextButtonBundle::symbol(theme, "➕")));
        });
}

fn setup_family_menu_buttons(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                align_self: AlignSelf::FlexEnd,
                right: Val::Px(0.0),
                column_gap: theme.gap.normal,
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
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent.spawn(LabelBundle::normal(theme, "Save family"));
                        parent.spawn((
                            FamilyNameEdit,
                            ActiveEdit,
                            TextEditBundle::new(theme, "New family"),
                        ));
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    column_gap: theme.gap.normal,
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
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
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
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
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
                                                column_gap: theme.gap.normal,
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
                                    column_gap: theme.gap.normal,
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
