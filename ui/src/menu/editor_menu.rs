use std::mem;

use bevy::prelude::*;
use bevy_simple_text_input::TextInputValue;

use crate::preview::{Preview, PreviewProcessed};
use project_harmonia_base::game_world::{
    city::City,
    family::{
        editor::{
            EditorActor, EditorFamily, EditorFamilyReset, EditorFirstName, EditorLastName,
            EditorSelectedActor, EditorSex, FamilyScene,
        },
        FamilyCreate,
    },
    WorldState,
};
use project_harmonia_widgets::{
    button::{ButtonKind, ExclusiveButton, Toggled},
    dialog::Dialog,
    label::LabelKind,
    text_edit::TextEdit,
    theme::Theme,
};

pub(super) struct EditorMenuPlugin;

impl Plugin for EditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::create_actor_buttons)
            .add_observer(Self::remove_actor_buttons)
            .add_observer(Self::display_actor_data)
            .add_systems(OnEnter(WorldState::FamilyEditor), Self::setup)
            .add_systems(
                Update,
                (
                    Self::apply_first_name.never_param_warn(),
                    Self::apply_last_name.never_param_warn(),
                    Self::update_previews,
                )
                    .run_if(in_state(WorldState::FamilyEditor)),
            );
    }
}

impl EditorMenuPlugin {
    fn setup(
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("entering family editor");
        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(WorldState::FamilyEditor),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
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

    fn create_actor_buttons(
        trigger: Trigger<OnAdd, EditorActor>,
        mut commands: Commands,
        node_entity: Single<Entity, With<ActorsNode>>,
    ) {
        debug!("creating button for actor `{}`", trigger.entity());
        commands.entity(*node_entity).with_children(|parent| {
            parent
                .spawn(ActorButton(trigger.entity()))
                .with_child(Preview::Actor(trigger.entity()))
                .observe(Self::select_actor);
        });
    }

    fn remove_actor_buttons(
        trigger: Trigger<OnRemove, EditorActor>,
        mut commands: Commands,
        buttons: Query<(Entity, &ActorButton)>,
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

    // Updates UI with parameters of the current actor.
    fn display_actor_data(
        trigger: Trigger<OnAdd, EditorSelectedActor>,
        actors: Query<(&EditorSex, &EditorFirstName, &EditorLastName)>,
        mut sex_buttons: Query<(&mut Toggled, &EditorSex), Without<ActorButton>>,
        mut first_name_edits: Query<&mut TextInputValue, With<FirstNameEdit>>,
        mut last_name_edits: Query<
            &mut TextInputValue,
            (With<LastNameEdit>, Without<FirstNameEdit>),
        >,
    ) {
        let (&actor_sex, first_name, last_name) = actors.get(trigger.entity()).unwrap();
        first_name_edits.single_mut().0.clone_from(first_name);
        last_name_edits.single_mut().0.clone_from(last_name);

        let (mut sex_toggled, ..) = sex_buttons
            .iter_mut()
            .find(|(_, &sex)| sex == actor_sex)
            .expect("sex buttons should be spawned for each variant");
        sex_toggled.0 = true;
    }

    fn add_actor(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        family_entity: Single<Entity, With<EditorFamily>>,
        selected_entity: Single<Entity, With<EditorSelectedActor>>,
    ) {
        info!("adding new actor");
        commands
            .entity(*selected_entity)
            .remove::<EditorSelectedActor>();
        commands.entity(*family_entity).with_children(|parent| {
            parent.spawn(EditorSelectedActor);
        });
    }

    fn select_actor(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        selected_entity: Single<Entity, With<EditorSelectedActor>>,
        actor_buttons: Query<&ActorButton>,
    ) {
        let actor_button = *actor_buttons.get(trigger.entity()).unwrap();
        info!("selecting actor `{}`", *actor_button);
        commands
            .entity(*selected_entity)
            .remove::<EditorSelectedActor>();
        commands.entity(*actor_button).insert(EditorSelectedActor);
    }

    fn apply_sex(
        trigger: Trigger<Pointer<Click>>,
        mut actor_sex: Single<&mut EditorSex, With<EditorSelectedActor>>,
        buttons: Query<&EditorSex, Without<EditorSelectedActor>>,
    ) {
        let button_sex = *buttons.get(trigger.entity()).unwrap();
        info!("changing sex to '{button_sex:?}'");
        **actor_sex = button_sex;
    }

    fn apply_first_name(
        text: Single<&TextInputValue, (Changed<TextInputValue>, With<FirstNameEdit>)>,
        actors: Single<(&mut EditorFirstName, Ref<EditorSelectedActor>)>,
    ) {
        // Avoid changes on actor switching.
        let (mut first_name, selected) = actors.into_inner();
        if !selected.is_added() {
            debug!("updating first name to '{}'", text.0);
            first_name.0.clone_from(&text.0);
        }
    }

    fn apply_last_name(
        text: Single<&TextInputValue, (Changed<TextInputValue>, With<LastNameEdit>)>,
        actors: Single<(&mut EditorLastName, Ref<EditorSelectedActor>)>,
    ) {
        // Avoid changes on actor switching.
        let (mut last_name, selected) = actors.into_inner();
        if !selected.is_added() {
            debug!("updating last name to '{}'", text.0);
            last_name.0.clone_from(&text.0);
        }
    }

    fn update_previews(
        mut commands: Commands,
        actors: Query<(Entity, Ref<EditorSex>), With<EditorActor>>,
        buttons: Query<(&Children, &ActorButton)>,
        images: Query<Entity, With<PreviewProcessed>>,
    ) {
        for (actor_entity, _) in actors
            .iter()
            .filter(|(_, sex)| sex.is_changed() && !sex.is_added())
        {
            debug!("updating preview for actor `{actor_entity}`");
            let (children, _) = buttons
                .iter()
                .find(|(_, edit_actor)| edit_actor.0 == actor_entity)
                .expect("each actor should have a corresponding button");
            let image_entity = images
                .iter_many(children)
                .next()
                .expect("actor buttons should have images");
            commands.entity(image_entity).remove::<PreviewProcessed>();
        }
    }

    fn confirm_family(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        commands.entity(*root_entity).with_children(|parent| {
            setup_save_family_dialog(parent, &theme);
        });
    }

    fn cancel_family(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
        commands.set_state(WorldState::World);
    }

    fn save_family(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        cities: Query<(Entity, &Name), With<City>>,
        family_name: Single<&TextInputValue, With<FamilyNameEdit>>,
        dialog_entity: Single<Entity, With<Dialog>>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        commands.insert_resource(FamilyScene::new(family_name.0.clone()));
        commands.entity(*root_entity).with_children(|parent| {
            setup_place_family_dialog(parent, &theme, &cities);
        });
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn cancel_saving(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        info!("cancelling saving");
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn create_new(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        commands.trigger(EditorFamilyReset);
        commands.entity(*dialog_entity).despawn_recursive()
    }

    fn cancel_placing(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        info!("cancelling placing");
        commands.entity(*dialog_entity).despawn_recursive()
    }

    fn place_and_play(
        trigger: Trigger<Pointer<Click>>,
        mut spawn_events: EventWriter<FamilyCreate>,
        mut family_scene: ResMut<FamilyScene>,
        buttons: Query<&PlaceCityButton>,
    ) {
        info!("placing family with select");
        let city_button = buttons.get(trigger.entity()).unwrap();
        spawn_events.send(FamilyCreate {
            city_entity: city_button.city_entity,
            scene: mem::take(&mut family_scene),
            select: true,
        });
    }

    fn place(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        mut spawn_events: EventWriter<FamilyCreate>,
        mut family_scene: ResMut<FamilyScene>,
        buttons: Query<&PlaceCityButton>,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        info!("placing family");
        let city_button = buttons.get(trigger.entity()).unwrap();
        spawn_events.send(FamilyCreate {
            city_entity: city_button.city_entity,
            scene: mem::take(&mut family_scene),
            select: false,
        });
        commands.entity(*dialog_entity).despawn_recursive();
        commands.trigger(EditorFamilyReset);
    }
}

fn setup_personality_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                padding: theme.padding.normal,
                row_gap: theme.gap.normal,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            parent
                .spawn((Node {
                    display: Display::Grid,
                    column_gap: theme.gap.normal,
                    row_gap: theme.gap.normal,
                    grid_template_columns: vec![GridTrack::auto(); 2],
                    ..Default::default()
                },))
                .with_children(|parent| {
                    parent.spawn((LabelKind::Normal, Text::new("First name")));
                    parent.spawn(FirstNameEdit);
                    parent.spawn((LabelKind::Normal, Text::new("Last name")));
                    parent.spawn(LastNameEdit);
                });

            parent.spawn(Node::default()).with_children(|parent| {
                parent
                    .spawn((
                        EditorSex::Male,
                        ButtonKind::Normal,
                        ExclusiveButton,
                        Toggled(true),
                    ))
                    .with_child(Text::new("Male"))
                    .observe(EditorMenuPlugin::apply_sex);
                parent
                    .spawn((EditorSex::Female, ButtonKind::Normal, ExclusiveButton))
                    .with_child(Text::new("Female"))
                    .observe(EditorMenuPlugin::apply_sex);
            });
        });
}

fn setup_actors_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                align_self: AlignSelf::FlexEnd,
                column_gap: theme.gap.normal,
                padding: theme.padding.global,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            parent.spawn((
                ActorsNode,
                Node {
                    column_gap: theme.gap.normal,
                    ..Default::default()
                },
            ));
            parent
                .spawn(ButtonKind::Symbol)
                .with_child(Text::new("➕"))
                .observe(EditorMenuPlugin::add_actor);
        });
}

fn setup_family_menu_buttons(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(Node {
            position_type: PositionType::Absolute,
            align_self: AlignSelf::FlexEnd,
            right: Val::Px(0.0),
            column_gap: theme.gap.normal,
            padding: theme.padding.global,
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonKind::Normal)
                .with_child(Text::new("Confirm"))
                .observe(EditorMenuPlugin::confirm_family);
            parent
                .spawn(ButtonKind::Normal)
                .with_child(Text::new("Cancel"))
                .observe(EditorMenuPlugin::cancel_family);
        });
}

fn setup_save_family_dialog(parent: &mut ChildBuilder, theme: &Theme) {
    info!("showing save family dialog");
    parent.spawn(Dialog).with_children(|parent| {
        parent
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: theme.padding.normal,
                    row_gap: theme.gap.normal,
                    ..Default::default()
                },
                theme.panel_background,
            ))
            .with_children(|parent| {
                parent.spawn((LabelKind::Normal, Text::new("Save family")));
                parent.spawn((
                    FamilyNameEdit,
                    // HACK: For some reason it can't be required component, it messes the edit.
                    TextEdit,
                    TextInputValue("New family".to_string()),
                ));
                parent
                    .spawn(Node {
                        column_gap: theme.gap.normal,
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Save"))
                            .observe(EditorMenuPlugin::save_family);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Cancel"))
                            .observe(EditorMenuPlugin::cancel_saving);
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
        .spawn((Dialog, StateScoped(WorldState::FamilyEditor)))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: theme.padding.normal,
                        row_gap: theme.gap.normal,
                        ..Default::default()
                    },
                    theme.panel_background,
                ))
                .with_children(|parent| {
                    parent.spawn((LabelKind::Normal, Text::new("Place family")));
                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // TODO: Use combobox.
                            for (city_entity, name) in cities {
                                parent
                                    .spawn(Node {
                                        column_gap: theme.gap.normal,
                                        ..Default::default()
                                    })
                                    .with_children(|parent| {
                                        parent.spawn((LabelKind::Normal, Text::new(name)));
                                        parent
                                            .spawn(PlaceCityButton { city_entity })
                                            .with_child(Text::new("Place & play"))
                                            .observe(EditorMenuPlugin::place_and_play);
                                        parent
                                            .spawn(PlaceCityButton { city_entity })
                                            .with_child(Text::new("Place"))
                                            .observe(EditorMenuPlugin::place);
                                    });
                            }
                        });

                    parent
                        .spawn(Node {
                            column_gap: theme.gap.normal,
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(ButtonKind::Normal)
                                .with_child(Text::new("Cancel"))
                                .observe(EditorMenuPlugin::cancel_placing);
                            parent
                                .spawn(ButtonKind::Normal)
                                .with_child(Text::new("Create new"))
                                .observe(EditorMenuPlugin::create_new);
                        });
                });
        });
}

#[derive(Component)]
#[require(Name(|| Name::new("First name edit")), TextEdit)]
struct FirstNameEdit;

#[derive(Component)]
#[require(Name(|| Name::new("Last name edit")), TextEdit)]
struct LastNameEdit;

#[derive(Component)]
#[require(Name(|| Name::new("Actors node")), Node)]
struct ActorsNode;

#[derive(Component, Debug, Deref, Clone, Copy)]
#[require(
    Name(|| Name::new("Actor button")), 
    ButtonKind(|| ButtonKind::Image),
    ExclusiveButton,
    Toggled(|| Toggled(true)),
)]
struct ActorButton(Entity);

#[derive(Component)]
struct FamilyNameEdit;

#[derive(Component)]
#[require(Name(|| Name::new("Place city button")), ButtonKind(|| ButtonKind::Normal))]
struct PlaceCityButton {
    city_entity: Entity,
}
