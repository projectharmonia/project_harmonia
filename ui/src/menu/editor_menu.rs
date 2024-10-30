use std::mem;

use anyhow::Result;
use bevy::prelude::*;
use bevy_simple_text_input::TextInputValue;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::preview::{Preview, PreviewProcessed};
use project_harmonia_base::{
    game_world::{
        actor::{FirstName, LastName, Sex},
        city::City,
        family::{
            editor::{EditableActor, EditableActorBundle, EditableFamily, FamilyReset},
            FamilyCreate, FamilyScene,
        },
        WorldState,
    },
    message::error_message,
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, ImageButtonBundle, TextButtonBundle, Toggled},
    click::Click,
    dialog::Dialog,
    dialog::DialogBundle,
    label::LabelBundle,
    text_edit::TextEditBundle,
    theme::Theme,
};

pub(super) struct EditorMenuPlugin;

impl Plugin for EditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::remove_actor_buttons)
            .add_systems(OnEnter(WorldState::FamilyEditor), Self::setup)
            .add_systems(
                Update,
                (
                    Self::add_member,
                    Self::update_actor_previews,
                    (
                        Self::switch_actor,
                        (
                            Self::set_sex,
                            Self::update_first_name,
                            Self::update_last_name,
                        ),
                    )
                        .chain(),
                    Self::handle_family_menu_clicks,
                    Self::handle_save_family_clicks.pipe(error_message),
                    Self::handle_place_dialog_clicks,
                    Self::handle_city_place_clicks.run_if(resource_exists::<FamilyScene>),
                )
                    .run_if(in_state(WorldState::FamilyEditor)),
            )
            .add_systems(
                PostUpdate,
                Self::create_actor_buttons.run_if(in_state(WorldState::FamilyEditor)),
            );
    }
}

impl EditorMenuPlugin {
    fn setup(
        mut commands: Commands,
        theme: Res<Theme>,

        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("entering family editor");
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(WorldState::FamilyEditor),
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
        });
    }

    fn add_member(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<(), With<PlusButton>>,
        families: Query<Entity, With<EditableFamily>>,
    ) {
        for _ in buttons.iter_many(click_events.read().map(|event| event.0)) {
            info!("adding new member");
            commands.entity(families.single()).with_children(|parent| {
                parent.spawn(EditableActorBundle::default());
            });
        }
    }

    fn create_actor_buttons(
        mut commands: Commands,
        theme: Res<Theme>,
        actors: Query<Entity, Added<EditableActor>>,
        actor_nodes: Query<Entity, With<ActorsNode>>,
    ) {
        for entity in &actors {
            debug!("creating button for actor `{entity}`");
            commands
                .entity(actor_nodes.single())
                .with_children(|parent| {
                    parent.spawn((
                        EditActor(entity),
                        Preview::Actor(entity),
                        ExclusiveButton,
                        Toggled(true),
                        ImageButtonBundle::placeholder(&theme),
                    ));
                });
        }
    }

    fn update_actor_previews(
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
            debug!("updating preview for actor `{actor_entity}`");
            commands.entity(button_entity).remove::<PreviewProcessed>();
        }
    }

    fn remove_actor_buttons(
        trigger: Trigger<OnRemove, EditableActor>,
        mut commands: Commands,
        buttons: Query<(Entity, &EditActor)>,
    ) {
        if let Some((entity, _)) = buttons
            .iter()
            .find(|(_, edit_actor)| edit_actor.0 == trigger.entity())
        {
            debug!(
                "removing button `{entity}` for despawned actor `{}`",
                trigger.entity()
            );
            commands.entity(entity).despawn_recursive();
        }
    }

    fn switch_actor(
        actor_buttons: Query<(&Toggled, &EditActor), Changed<Toggled>>,
        mut actors: Query<(&mut Visibility, &Sex, &FirstName, &LastName), With<EditableActor>>,
        mut sex_buttons: Query<(&mut Toggled, &Sex), Without<EditActor>>,
        mut first_name_edits: Query<&mut TextInputValue, With<FirstNameEdit>>,
        mut last_name_edits: Query<
            &mut TextInputValue,
            (With<LastNameEdit>, Without<FirstNameEdit>),
        >,
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
                info!("switching actor to `{edit_actor:?}`");

                // Update UI with parameters of the current actor.
                let (mut visibility, &actor_sex, first_name, last_name) = actors
                    .get_mut(edit_actor.0)
                    .expect("actor button should point to a valid actor");
                *visibility = Visibility::Visible;
                first_name_edits.single_mut().0.clone_from(first_name);
                last_name_edits.single_mut().0.clone_from(last_name);

                let (mut sex_toggled, ..) = sex_buttons
                    .iter_mut()
                    .find(|(_, &sex)| sex == actor_sex)
                    .expect("sex buttons should be spawned for each variant");
                sex_toggled.0 = true;
            }
        }
    }

    fn set_sex(
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
                    info!("changing sex to '{button_sex}'");
                    *actor_sex = button_sex;
                }
            }
        }
    }

    fn update_first_name(
        text_edits: Query<&TextInputValue, (Changed<TextInputValue>, With<FirstNameEdit>)>,
        mut actors: Query<(&mut FirstName, &Visibility), With<EditableActor>>,
    ) {
        for text in &text_edits {
            if let Some((mut first_name, _)) = actors
                .iter_mut()
                .filter(|(visibility, _)| !visibility.is_changed()) // Avoid changes on actor switching.
                .find(|(_, &visibility)| visibility == Visibility::Visible)
            {
                debug!("updating first name to '{}'", text.0);
                first_name.0.clone_from(&text.0);
            }
        }
    }

    fn update_last_name(
        text_edits: Query<&TextInputValue, (Changed<TextInputValue>, With<LastNameEdit>)>,
        mut actors: Query<(&mut LastName, &Visibility), With<EditableActor>>,
    ) {
        for text in &text_edits {
            if let Some((mut last_name, _)) = actors
                .iter_mut()
                .filter(|(visibility, _)| !visibility.is_changed()) // Avoid changes on actor switching.
                .find(|(_, &visibility)| visibility == Visibility::Visible)
            {
                debug!("updating second name to '{}'", text.0);
                last_name.0.clone_from(&text.0);
            }
        }
    }

    fn handle_family_menu_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        mut world_state: ResMut<NextState<WorldState>>,
        theme: Res<Theme>,
        buttons: Query<&FamilyMenuButton>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                FamilyMenuButton::Confirm => {
                    commands.entity(roots.single()).with_children(|parent| {
                        setup_save_family_dialog(parent, &theme);
                    });
                }
                FamilyMenuButton::Cancel => world_state.set(WorldState::World),
            }
        }
    }

    fn handle_save_family_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        mut text_edits: Query<&mut TextInputValue, With<FamilyNameEdit>>,
        buttons: Query<&SaveDialogButton>,
        dialogs: Query<Entity, With<Dialog>>,
        cities: Query<(Entity, &Name), With<City>>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) -> Result<()> {
        for &button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                SaveDialogButton::Save => {
                    let mut family_name = text_edits.single_mut();
                    commands.insert_resource(FamilyScene::new(mem::take(&mut family_name.0)));
                    commands.entity(roots.single()).with_children(|parent| {
                        setup_place_family_dialog(parent, &theme, &cities);
                    });
                }
                SaveDialogButton::Cancel => info!("cancelling saving"),
            }

            commands.entity(dialogs.single()).despawn_recursive();
        }

        Ok(())
    }

    fn handle_place_dialog_clicks(
        mut commands: Commands,
        mut reset_events: EventWriter<FamilyReset>,
        mut click_events: EventReader<Click>,
        dialogs: Query<Entity, With<Dialog>>,
        buttons: Query<&PlaceDialogButton>,
    ) {
        for &button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                PlaceDialogButton::CreateNew => {
                    reset_events.send_default();
                }
                PlaceDialogButton::Cancel => info!("cancelling placing"),
            }
            commands.entity(dialogs.single()).despawn_recursive()
        }
    }

    fn handle_city_place_clicks(
        mut commands: Commands,
        mut spawn_events: EventWriter<FamilyCreate>,
        mut reset_events: EventWriter<FamilyReset>,
        mut click_events: EventReader<Click>,
        mut family_scene: ResMut<FamilyScene>,
        buttons: Query<(&CityPlaceButton, &PlaceCity)>,
        dialogs: Query<Entity, With<Dialog>>,
    ) {
        for (button, place_city) in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                CityPlaceButton::PlaceAndPlay => {
                    info!("placing family with select");
                    spawn_events.send(FamilyCreate {
                        city_entity: place_city.0,
                        scene: mem::take(&mut family_scene),
                        select: true,
                    });
                }
                CityPlaceButton::Place => {
                    info!("placing family");
                    spawn_events.send(FamilyCreate {
                        city_entity: place_city.0,
                        scene: mem::take(&mut family_scene),
                        select: false,
                    });
                    commands.entity(dialogs.single()).despawn_recursive();
                    reset_events.send_default();
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
                    parent.spawn((LastNameEdit, TextEditBundle::empty(theme).inactive(theme)));
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

fn setup_save_family_dialog(parent: &mut ChildBuilder, theme: &Theme) {
    info!("showing save family dialog");
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
                    parent.spawn((FamilyNameEdit, TextEditBundle::new(theme, "New family")));
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
}

fn setup_place_family_dialog(
    parent: &mut ChildBuilder,
    theme: &Theme,
    cities: &Query<(Entity, &Name), With<City>>,
) {
    info!("showing placing dialog");
    parent
        .spawn(DialogBundle::new(theme))
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
                                                TextButtonBundle::normal(theme, button.to_string()),
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

#[derive(Component, Debug)]
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
