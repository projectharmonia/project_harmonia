use std::{fs, mem};

use anyhow::{Context, Result};
use bevy::prelude::*;
use derive_more::Display;
use strum::{EnumIter, IntoEnumIterator};

use crate::core::{
    game_paths::GamePaths,
    game_state::GameState,
    game_world::{GameLoad, GameWorldPlugin, WorldName},
};

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, ui_root::UiRoot, LabelBundle, Modal, ModalBundle},
};

pub(super) struct WorldBrowserPlugin;

impl Plugin for WorldBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::setup_system.in_schedule(OnEnter(GameState::WorldBrowser)))
            .add_systems(
                (
                    Self::world_buttons_system.after(GameWorldPlugin::loading_system),
                    Self::remove_confirmation_system.pipe(error),
                )
                    .in_set(OnUpdate(GameState::WorldBrowser)),
            );
    }
}

impl WorldBrowserPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>, game_paths: Res<GamePaths>) {
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
                parent.spawn(LabelBundle::large(&theme, "World browser"));
                parent
                    .spawn(NodeBundle {
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
                    })
                    .with_children(|parent| {
                        let world_names = game_paths
                            .get_world_names()
                            .map_err(|e| error!("unable to get world names: {e}"))
                            .unwrap_or_default();
                        for world_name in world_names {
                            setup_world_node(parent, &theme, world_name);
                        }
                    });
            });
    }

    fn world_buttons_system(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        theme: Res<Theme>,
        buttons: Query<(&Interaction, &WorldButton, &WorldNode), Changed<Interaction>>,
        mut labels: Query<&mut Text>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for (&interaction, world_button, &world_node) in &buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            let mut text = labels
                .get_mut(world_node.label_entity)
                .expect("world label should contain text");
            let world_name = &mut text.sections[0].value;
            match world_button {
                WorldButton::Play => {
                    commands.insert_resource(WorldName(mem::take(world_name)));
                    load_events.send_default();
                }
                WorldButton::Host => todo!(),
                WorldButton::Delete => {
                    commands.entity(roots.single()).with_children(|parent| {
                        parent
                            .spawn((ModalBundle::new(&theme), world_node))
                            .with_children(|parent| {
                                parent
                                    .spawn(NodeBundle {
                                        style: Style {
                                            size: Size::new(Val::Percent(50.0), Val::Percent(20.0)),
                                            flex_direction: FlexDirection::Column,
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..Default::default()
                                        },
                                        background_color: theme.panel_color.into(),
                                        ..Default::default()
                                    })
                                    .with_children(|parent| {
                                        parent.spawn(LabelBundle::normal(
                                            &theme,
                                            format!(
                                                "Are you sure you want to remove world {world_name}?",
                                            ),
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
                                                for dialog_button in RemoveDialogButton::iter() {
                                                    parent.spawn((
                                                        dialog_button,
                                                        TextButtonBundle::normal(
                                                            &theme,
                                                            dialog_button.to_string(),
                                                        ),
                                                    ));
                                                }
                                            });
                                    });
                            });
                    });
                }
            }
        }
    }

    fn remove_confirmation_system(
        mut commands: Commands,
        game_paths: Res<GamePaths>,
        dialogs: Query<(Entity, &WorldNode), With<Modal>>,
        buttons: Query<(&Interaction, &RemoveDialogButton)>,
        labels: Query<&Text>,
    ) -> Result<()> {
        for (&interaction, dialog_button) in &buttons {
            if interaction == Interaction::Clicked {
                let (dialog_entity, world_node) = dialogs.single();
                let text = labels
                    .get(world_node.label_entity)
                    .expect("world label should contain text");
                let world_name = &text.sections[0].value;
                match dialog_button {
                    RemoveDialogButton::Remove => {
                        let world_path = game_paths.world_path(world_name);
                        fs::remove_file(&world_path)
                            .with_context(|| format!("unable to remove {world_path:?}"))?;
                        commands.entity(world_node.node_entity).despawn_recursive();
                    }
                    RemoveDialogButton::Cancel => (),
                }
                commands.entity(dialog_entity).despawn_recursive();
            }
        }

        Ok(())
    }
}

fn setup_world_node(parent: &mut ChildBuilder, theme: &Theme, label: impl Into<String>) {
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
            let node_entity = parent.parent_entity();
            let label_entity = parent.spawn(LabelBundle::large(theme, label)).id();
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .add_child(label_entity);
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
                    for button in WorldButton::iter() {
                        parent.spawn((
                            button,
                            WorldNode {
                                label_entity,
                                node_entity,
                            },
                            TextButtonBundle::normal(theme, button.to_string()),
                        ));
                    }
                });
        });
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum WorldButton {
    Play,
    Host,
    Delete,
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum RemoveDialogButton {
    Remove,
    Cancel,
}

/// Associated world node entities.
#[derive(Clone, Component, Copy)]
struct WorldNode {
    label_entity: Entity,
    node_entity: Entity,
}
