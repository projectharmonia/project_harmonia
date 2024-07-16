use std::{fmt::Display, mem};

use bevy::prelude::*;
use bevy_simple_text_input::TextInputValue;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::ui_root::UiRoot;
use project_harmonia_base::{
    core::GameState,
    game_world::{
        actor::SelectedActor,
        city::{ActiveCity, City, CityBundle},
        family::{Family, FamilyDelete, FamilyMembers},
        GameWorld,
    },
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, TabContent, TextButtonBundle, Toggled},
    click::Click,
    dialog::Dialog,
    dialog::DialogBundle,
    label::LabelBundle,
    text_edit::TextEditBundle,
    theme::Theme,
};

pub(super) struct WorldMenuPlugin;

impl Plugin for WorldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::World), Self::setup)
            .add_systems(
                Update,
                (
                    Self::handle_family_clicks,
                    Self::handle_city_clicks,
                    Self::handle_create_clicks,
                    Self::handle_city_dialog_clicks,
                )
                    .run_if(in_state(GameState::World)),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::create_family_nodes,
                    Self::create_city_nodes,
                    Self::remove_entity_nodes,
                )
                    .run_if(in_state(GameState::World)),
            );
    }
}

impl WorldMenuPlugin {
    fn setup(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        game_world: Res<GameWorld>,
        families: Query<(Entity, &Family)>,
        cities: Query<(Entity, &City)>,
    ) {
        info!("entering world menu");
        commands
            .spawn((
                UiRoot,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: theme.padding.global,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(LabelBundle::large(&theme, game_world.name.clone()));

                let tabs_entity = parent
                    .spawn(NodeBundle {
                        style: Style {
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .id();

                for tab in WorldTab::iter() {
                    let content_entity = parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::FlexStart,
                                padding: theme.padding.normal,
                                row_gap: theme.gap.normal,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| match tab {
                            WorldTab::Families => {
                                for (entity, family) in &families {
                                    setup_entity_node::<FamilyButton>(
                                        parent,
                                        &theme,
                                        entity,
                                        &family.name,
                                    );
                                }
                            }
                            WorldTab::Cities => {
                                for (entity, city) in &cities {
                                    setup_entity_node::<CityButton>(
                                        parent, &theme, entity, &city.name,
                                    );
                                }
                            }
                        })
                        .id();

                    tab_commands
                        .spawn((
                            tab,
                            TabContent(content_entity),
                            ExclusiveButton,
                            Toggled(tab == Default::default()),
                            TextButtonBundle::normal(&theme, tab.to_string()),
                        ))
                        .set_parent(tabs_entity);
                }

                parent
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::FlexStart,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent.spawn((
                            CreateEntityButton,
                            TextButtonBundle::normal(&theme, "Create new"),
                        ));
                    });
            });
    }

    fn create_family_nodes(
        mut commands: Commands,
        theme: Res<Theme>,
        families: Query<(Entity, &Family), Added<Family>>,
        tabs: Query<(&TabContent, &WorldTab)>,
        nodes: Query<&WorldEntity>,
    ) {
        for (entity, family) in &families {
            let (tab_content, _) = tabs
                .iter()
                .find(|(_, &tab)| tab == WorldTab::Families)
                .expect("tab with families should be spawned on state enter");
            if nodes.iter().all(|world_entity| world_entity.0 != entity) {
                debug!("creating button for family '{}'", family.name);
                commands.entity(tab_content.0).with_children(|parent| {
                    setup_entity_node::<FamilyButton>(parent, &theme, entity, &family.name);
                });
            }
        }
    }

    fn create_city_nodes(
        mut commands: Commands,
        theme: Res<Theme>,
        cities: Query<(Entity, &City), Added<City>>,
        tabs: Query<(&TabContent, &WorldTab)>,
        nodes: Query<&WorldEntity>,
    ) {
        for (entity, city) in &cities {
            let (tab_content, _) = tabs
                .iter()
                .find(|(_, &tab)| tab == WorldTab::Cities)
                .expect("tab with cities should be spawned on state enter");
            if !nodes.iter().any(|world_entity| world_entity.0 == entity) {
                debug!("creating button for city '{}'", city.name);
                commands.entity(tab_content.0).with_children(|parent| {
                    setup_entity_node::<CityButton>(parent, &theme, entity, &city.name);
                });
            }
        }
    }

    fn handle_family_clicks(
        mut commands: Commands,
        mut delete_events: EventWriter<FamilyDelete>,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<(&WorldEntityNode, &FamilyButton)>,
        nodes: Query<&WorldEntity>,
        families: Query<&FamilyMembers>,
    ) {
        for (entity_node, family_button) in
            buttons.iter_many(click_events.read().map(|event| event.0))
        {
            let world_entity = nodes
                .get(entity_node.0)
                .expect("family button should reference world entity node");
            match family_button {
                FamilyButton::Play => {
                    let members = families
                        .get(world_entity.0)
                        .expect("world entity node should reference a family");
                    let actor_entity = *members
                        .first()
                        .expect("family always have at least one member");

                    info!("starting playing for family `{:?}`", world_entity.0);
                    commands.entity(actor_entity).insert(SelectedActor);
                    game_state.set(GameState::Family);
                }
                FamilyButton::Delete => {
                    info!("deleting family `{:?}`", world_entity.0);
                    delete_events.send(FamilyDelete(world_entity.0));
                }
            }
        }
    }

    fn handle_city_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<(&WorldEntityNode, &CityButton)>,
        nodes: Query<&WorldEntity>,
    ) {
        for (entity_node, family_button) in
            buttons.iter_many(click_events.read().map(|event| event.0))
        {
            let world_entity = nodes
                .get(entity_node.0)
                .expect("family button should reference world entity node");
            // TODO: use event for despawn, otherwise client will despawn the city locally.
            match family_button {
                CityButton::Edit => {
                    info!("starting editing city `{:?}`", world_entity.0);
                    commands.entity(world_entity.0).insert(ActiveCity);
                    game_state.set(GameState::City);
                }
                CityButton::Delete => {
                    info!("deleting city `{:?}`", world_entity.0);
                    commands.entity(world_entity.0).despawn();
                }
            }
        }
    }

    fn handle_create_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        theme: Res<Theme>,
        buttons: Query<(), With<CreateEntityButton>>,
        tabs: Query<(&Toggled, &WorldTab)>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for _ in buttons.iter_many(click_events.read().map(|event| event.0)) {
            let current_tab = tabs
                .iter()
                .find_map(|(toggled, world_tab)| toggled.0.then_some(world_tab))
                .expect("one tab should always be active");

            match current_tab {
                WorldTab::Families => game_state.set(GameState::FamilyEditor),
                WorldTab::Cities => {
                    setup_create_city_dialog(&mut commands, roots.single(), &theme);
                }
            }
        }
    }

    fn handle_city_dialog_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<&CityDialogButton>,
        mut text_edits: Query<&mut TextInputValue, With<CityNameEdit>>,
        dialogs: Query<Entity, With<Dialog>>,
    ) {
        for &dialog_button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            if dialog_button == CityDialogButton::Create {
                info!("creating new city");
                let mut city_name = text_edits.single_mut();
                commands.spawn(CityBundle::new(mem::take(&mut city_name.0)));
            }
            commands.entity(dialogs.single()).despawn_recursive();
        }
    }

    fn remove_entity_nodes(
        mut commands: Commands,
        mut removed_cities: RemovedComponents<City>,
        mut removed_families: RemovedComponents<Family>,
        nodes: Query<(Entity, &WorldEntity)>,
    ) {
        for removed_entity in removed_cities.read().chain(removed_families.read()) {
            let (node_entity, _) = nodes
                .iter()
                .find(|(_, world_entity)| world_entity.0 == removed_entity)
                .expect("each city and family should have corresponding node");
            commands.entity(node_entity).despawn_recursive();
        }
    }
}

fn setup_entity_node<E>(
    parent: &mut ChildBuilder,
    theme: &Theme,
    entity: Entity,
    label: impl Into<String>,
) where
    E: IntoEnumIterator + Clone + Copy + Component + Display,
{
    parent
        .spawn((
            WorldEntity(entity),
            NodeBundle {
                style: Style {
                    padding: theme.padding.normal,
                    column_gap: theme.gap.normal,
                    ..Default::default()
                },
                background_color: theme.panel_color.into(),
                ..Default::default()
            },
        ))
        .with_children(|parent| {
            let node_entity = parent.parent_entity();

            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(LabelBundle::large(theme, label));
                });
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: theme.gap.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    for button in E::iter() {
                        parent.spawn((
                            button,
                            WorldEntityNode(node_entity),
                            TextButtonBundle::normal(theme, button.to_string()),
                        ));
                    }
                });
        });
}

fn setup_create_city_dialog(commands: &mut Commands, root_entity: Entity, theme: &Theme) {
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
                        parent.spawn(LabelBundle::normal(theme, "Create city"));
                        parent.spawn((CityNameEdit, TextEditBundle::new(theme, "New city")));
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    column_gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                for dialog_button in CityDialogButton::iter() {
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

#[derive(Clone, Component, Copy, Default, Display, EnumIter, PartialEq)]
enum WorldTab {
    #[default]
    Families,
    Cities,
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum FamilyButton {
    Play,
    Delete,
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum CityButton {
    Edit,
    Delete,
}

/// References family or city depending on a node.
#[derive(Component)]
struct WorldEntity(Entity);

/// References family node for [`FamilyButton`] or city node for [`CityButton`].
#[derive(Component)]
struct WorldEntityNode(Entity);

/// Button that creates family or city depending on the selected [`WorldTab`].
#[derive(Component)]
struct CreateEntityButton;

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum CityDialogButton {
    Create,
    Cancel,
}

#[derive(Component)]
struct CityNameEdit;
