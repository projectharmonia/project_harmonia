use std::mem;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_simple_text_input::TextInputValue;

use project_harmonia_base::{
    core::GameState,
    error_message::ErrorMessage,
    game_world::{
        actor::SelectedActor,
        city::{ActiveCity, City},
        family::{Family, FamilyDelete, FamilyMembers},
        WorldName, WorldState,
    },
};
use project_harmonia_widgets::{
    button::{ButtonKind, TabContent, Toggled},
    dialog::Dialog,
    label::LabelKind,
    text_edit::TextEdit,
    theme::Theme,
};
use strum::{EnumIter, IntoEnumIterator};

pub(super) struct WorldMenuPlugin;

impl Plugin for WorldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(remove_entity_nodes::<Family>)
            .add_observer(remove_entity_nodes::<City>)
            .add_observer(create_family_nodes)
            .add_observer(create_city_nodes)
            .add_systems(OnEnter(WorldState::World), setup);
    }
}

fn setup(
    mut commands: Commands,
    mut tab_commands: Commands,
    theme: Res<Theme>,
    world_name: Res<WorldName>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    families: Query<(Entity, &Name), With<Family>>,
    cities: Query<(Entity, &Name), With<City>>,
) {
    commands.entity(*root_entity).with_children(|parent| {
        info!("entering world menu");
        parent
            .spawn((
                StateScoped(WorldState::World),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    padding: theme.padding.global,
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((LabelKind::Large, Text::new(world_name.0.clone())));

                let tabs_entity = parent
                    .spawn(Node {
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    })
                    .id();

                for tab in WorldTab::iter() {
                    let content_entity = parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        })
                        .with_children(|parent| match tab {
                            WorldTab::Families => {
                                for (entity, name) in &families {
                                    setup_entity_node(
                                        setup_family_buttons,
                                        parent,
                                        &theme,
                                        entity,
                                        name,
                                    );
                                }
                            }
                            WorldTab::Cities => {
                                for (entity, name) in &cities {
                                    setup_entity_node(
                                        setup_city_buttons,
                                        parent,
                                        &theme,
                                        entity,
                                        name,
                                    );
                                }
                            }
                        })
                        .id();

                    tab_commands
                        .spawn((
                            tab,
                            ButtonKind::Normal,
                            TabContent(content_entity),
                            Toggled(tab == Default::default()),
                        ))
                        .with_child(Text::new(tab.text()))
                        .set_parent(tabs_entity);
                }

                parent
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::FlexStart,
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Exit world"))
                            .observe(exit_world);
                        parent.spawn(Node {
                            width: Val::Percent(100.0),
                            ..Default::default()
                        });
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Create"))
                            .observe(create);
                    });
            });
    });
}

fn setup_entity_node(
    buttons: fn(&mut ChildBuilder, WorldEntity),
    parent: &mut ChildBuilder,
    theme: &Theme,
    entity: Entity,
    label: impl Into<String>,
) {
    parent
        .spawn((
            WorldEntity(entity),
            WorldNode,
            Node {
                padding: theme.padding.normal,
                column_gap: theme.gap.normal,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn((LabelKind::Large, Text::new(label)));
                });
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: theme.gap.normal,
                    ..Default::default()
                })
                .with_children(|parent| {
                    (buttons)(parent, WorldEntity(entity));
                });
        });
}

fn setup_family_buttons(parent: &mut ChildBuilder, world_entity: WorldEntity) {
    parent
        .spawn((ButtonKind::Normal, world_entity))
        .with_child(Text::new("Play"))
        .observe(play_family);
    parent
        .spawn((ButtonKind::Normal, world_entity))
        .with_child(Text::new("Delete"))
        .observe(delete_family);
}

fn play_family(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&WorldEntity>,
    families: Query<&FamilyMembers>,
) {
    let world_entity = **buttons
        .get(trigger.entity())
        .expect("family button should reference world entity node");
    let members = families
        .get(world_entity)
        .expect("world entity node should reference a family");
    let actor_entity = *members
        .first()
        .expect("family always have at least one member");

    info!("starting playing for family `{world_entity}`");
    commands.entity(actor_entity).insert(SelectedActor);
    commands.set_state(WorldState::Family);
}

fn delete_family(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&WorldEntity>,
) {
    let world_entity = **buttons
        .get(trigger.entity())
        .expect("family button should reference world entity node");

    info!("deleting family `{world_entity}`");
    commands.client_trigger_targets(FamilyDelete, world_entity);
}

fn setup_city_buttons(parent: &mut ChildBuilder, world_entity: WorldEntity) {
    parent
        .spawn((ButtonKind::Normal, world_entity))
        .with_child(Text::new("Edit"))
        .observe(edit_city);
    parent
        .spawn((ButtonKind::Normal, world_entity))
        .with_child(Text::new("Delete"))
        .observe(delete_city);
}

fn edit_city(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&WorldEntity>,
) {
    let world_entity = **buttons
        .get(trigger.entity())
        .expect("city button should reference world entity node");

    info!("starting editing city `{world_entity}`");
    commands.entity(world_entity).insert(ActiveCity);
    commands.set_state(WorldState::City);
}

fn delete_city(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    buttons: Query<&WorldEntity>,
) {
    let world_entity = **buttons
        .get(trigger.entity())
        .expect("city button should reference world entity node");

    // TODO: use event for despawn, otherwise client will despawn the city locally.
    info!("deleting city `{world_entity}`");
    commands.entity(world_entity).despawn_recursive();
}

fn setup_create_city_dialog(parent: &mut ChildBuilder, theme: &Theme) {
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
                parent.spawn((LabelKind::Normal, Text::new("Create city")));
                parent.spawn((
                    CityNameEdit,
                    // HACK: For some reason it can't be required component, it messes the edit.
                    TextEdit,
                    TextInputValue("New city".to_string()),
                ));
                parent
                    .spawn(Node {
                        column_gap: theme.gap.normal,
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Create"))
                            .observe(confirm_city_creation);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Cancel"))
                            .observe(cancel_city_creation);
                    });
            });
    });
}

fn confirm_city_creation(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut city_name: Single<&mut TextInputValue, With<CityNameEdit>>,
    dialog_entity: Single<Entity, With<Dialog>>,
) {
    info!("creating new city");
    commands.spawn((City, Name::new(mem::take(&mut city_name.0))));
    commands.entity(*dialog_entity).despawn_recursive();
}

fn cancel_city_creation(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    dialog_entity: Single<Entity, With<Dialog>>,
) {
    commands.entity(*dialog_entity).despawn_recursive();
}

fn exit_world(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.set_state(GameState::Menu);
}

fn create(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    theme: Res<Theme>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    tabs: Query<(&Toggled, &WorldTab)>,
    cities: Query<(), With<City>>,
) {
    let current_tab = tabs
        .iter()
        .find_map(|(toggled, world_tab)| toggled.0.then_some(world_tab))
        .expect("one tab should always be active");

    info!("starting creation for `{current_tab:?}`");
    match current_tab {
        WorldTab::Families => {
            if cities.is_empty() {
                commands.trigger(ErrorMessage::new("You need to create at least one city"));
            } else {
                commands.set_state(WorldState::FamilyEditor);
            }
        }
        WorldTab::Cities => {
            commands.entity(*root_entity).with_children(|parent| {
                setup_create_city_dialog(parent, &theme);
            });
        }
    }
}

fn create_family_nodes(
    trigger: Trigger<OnAdd, Family>,
    mut commands: Commands,
    theme: Res<Theme>,
    families: Query<&Name>,
    tabs: Query<(&TabContent, &WorldTab)>,
    nodes: Query<&WorldEntity, With<WorldNode>>,
) {
    let Some((tab_content, _)) = tabs.iter().find(|(_, &tab)| tab == WorldTab::Families) else {
        return;
    };

    let name = families.get(trigger.entity()).unwrap();
    if nodes.iter().all(|&entity| *entity != trigger.entity()) {
        debug!("creating button for family '{name}'");
        commands.entity(**tab_content).with_children(|parent| {
            setup_entity_node(setup_family_buttons, parent, &theme, trigger.entity(), name);
        });
    }
}

fn create_city_nodes(
    trigger: Trigger<OnAdd, City>,
    mut commands: Commands,
    theme: Res<Theme>,
    cities: Query<&Name>,
    tabs: Query<(&TabContent, &WorldTab)>,
    nodes: Query<&WorldEntity, With<WorldNode>>,
) {
    let Some((tab_content, _)) = tabs.iter().find(|(_, &tab)| tab == WorldTab::Cities) else {
        return;
    };

    let name = cities.get(trigger.entity()).unwrap();
    if nodes.iter().all(|&entity| *entity != trigger.entity()) {
        debug!("creating button for city '{name}'");
        commands.entity(**tab_content).with_children(|parent| {
            setup_entity_node(setup_city_buttons, parent, &theme, trigger.entity(), name);
        });
    }
}

fn remove_entity_nodes<C: Component>(
    trigger: Trigger<OnRemove, C>,
    mut commands: Commands,
    nodes: Query<(Entity, &WorldEntity), With<WorldNode>>,
) {
    if let Some((entity, _)) = nodes
        .iter()
        .find(|(_, &world_entity)| *world_entity == trigger.entity())
    {
        debug!(
            "removing node `{entity}` for despawned entity `{}`",
            trigger.entity()
        );
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Clone, Component, Copy, Default, PartialEq, Debug, EnumIter)]
enum WorldTab {
    #[default]
    Families,
    Cities,
}

impl WorldTab {
    fn text(self) -> &'static str {
        match self {
            WorldTab::Families => "Families",
            WorldTab::Cities => "Cities",
        }
    }
}

/// References family or city depending on a node.
#[derive(Component, Clone, Copy, Deref)]
struct WorldEntity(Entity);

#[derive(Component)]
#[require(Node)]
struct WorldNode;

#[derive(Component)]
struct CityNameEdit;
