use std::fmt::Display;

use bevy::prelude::*;
use derive_more::Display;
use strum::{EnumIter, IntoEnumIterator};

use super::{
    theme::Theme,
    widget::{
        button::{ExclusiveButton, Pressed, TabContent, TextButtonBundle},
        ui_root::UiRoot,
        LabelBundle,
    },
};
use crate::core::{
    actor::ActiveActor,
    city::{ActiveCity, City},
    family::{FamilyActors, FamilyDespawn},
    game_state::GameState,
    game_world::WorldName,
};

pub(super) struct WorldMenuPlugin;

impl Plugin for WorldMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::setup_system.in_schedule(OnEnter(GameState::World)))
            .add_systems(
                (Self::family_button_system, Self::city_button_system)
                    .in_set(OnUpdate(GameState::World)),
            );
    }
}

impl WorldMenuPlugin {
    fn setup_system(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        world_name: Res<WorldName>,
        families: Query<(Entity, &Name), With<FamilyActors>>,
        cities: Query<(Entity, &Name), With<City>>,
    ) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: theme.padding.global,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                parent.spawn(LabelBundle::large(&theme, world_name.0.clone()));

                let tabs_entity = parent
                    .spawn(NodeBundle {
                        style: Style {
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .id();

                for (index, tab) in WorldTab::iter().enumerate() {
                    let content_entity = parent
                        .spawn((
                            tab,
                            NodeBundle {
                                style: Style {
                                    size: Size::all(Val::Percent(100.0)),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::FlexStart,
                                    padding: theme.padding.normal,
                                    gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .with_children(|parent| match tab {
                            WorldTab::Families => {
                                for (entity, name) in &families {
                                    setup_entity_node::<FamilyButton>(parent, &theme, entity, name);
                                }
                            }
                            WorldTab::Cities => {
                                for (entity, name) in &cities {
                                    setup_entity_node::<CityButton>(parent, &theme, entity, name);
                                }
                            }
                        })
                        .id();

                    tab_commands
                        .spawn((
                            TabContent(content_entity),
                            ExclusiveButton,
                            Pressed(index == 0),
                            TextButtonBundle::normal(&theme, tab.to_string()),
                        ))
                        .set_parent(tabs_entity);
                }

                parent
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Auto),
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

    fn family_button_system(
        mut commands: Commands,
        mut despawn_events: EventWriter<FamilyDespawn>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<(&Interaction, &WorldEntity, &FamilyButton), Changed<Interaction>>,
        families: Query<&FamilyActors>,
    ) {
        for (&interaction, world_entity, family_button) in &buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            match family_button {
                FamilyButton::Play => {
                    let actors = families
                        .get(world_entity.0)
                        .expect("world entity with family buttons should reference a family");
                    let actor_entity = *actors
                        .first()
                        .expect("family always have at least one member");

                    commands.entity(actor_entity).insert(ActiveActor);
                    game_state.set(GameState::Family);
                }
                FamilyButton::Delete => despawn_events.send(FamilyDespawn(world_entity.0)),
            }
        }
    }

    fn city_button_system(
        mut commands: Commands,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<(&Interaction, &WorldEntity, &CityButton), Changed<Interaction>>,
    ) {
        for (&interaction, world_entity, family_button) in &buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            // TODO: use event for despawn, otherwise client will despawn the city locally.
            match family_button {
                CityButton::Edit => {
                    commands.entity(world_entity.0).insert(ActiveCity);
                    game_state.set(GameState::City);
                }
                CityButton::Delete => commands.entity(world_entity.0).despawn(),
            }
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
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(50.0), Val::Percent(25.0)),
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
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
                        gap: theme.gap.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    for button in E::iter() {
                        parent.spawn((
                            button,
                            WorldEntity(entity),
                            TextButtonBundle::normal(theme, button.to_string()),
                        ));
                    }
                });
        });
}

#[derive(Clone, Component, Copy, Display, EnumIter)]
enum WorldTab {
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

/// References family for [`FamilyButton`] or city for [`CityButton`].
#[derive(Component)]
struct WorldEntity(Entity);

/// Button that creates family or city depending on the selected [`WorldTab`].
#[derive(Component)]
struct CreateEntityButton;
