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
                        flex_direction: FlexDirection::Column,
                        size: Size::all(Val::Percent(100.0)),
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

                let world_names = game_paths
                    .get_world_names()
                    .map_err(|e| error!("unable to get world names: {e}"))
                    .unwrap_or_default();
                for world_name in world_names {
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
                            let label_entity =
                                parent.spawn(LabelBundle::large(&theme, world_name)).id();
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
                                            WorldLabel(label_entity),
                                            WorldNode(node_entity),
                                            TextButtonBundle::normal(&theme, button.to_string()),
                                        ));
                                    }
                                });
                        });
                }
            });
    }

    fn world_buttons_system(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        theme: Res<Theme>,
        world_buttons: Query<
            (&Interaction, &WorldButton, &WorldLabel, &WorldNode),
            Changed<Interaction>,
        >,
        mut labels: Query<&mut Text>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for (&interaction, world_button, &world_label, &world_node) in &world_buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            let mut text = labels
                .get_mut(world_label.0)
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
                            .spawn((ModalBundle::new(&theme), world_node, world_label))
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
        dialogs: Query<(Entity, &WorldNode, &WorldLabel), With<Modal>>,
        buttons: Query<(&Interaction, &RemoveDialogButton)>,
        labels: Query<&Text>,
    ) -> Result<()> {
        for (&interaction, dialog_button) in &buttons {
            if interaction == Interaction::Clicked {
                let (dialog_entity, world_node, world_label) = dialogs.single();
                let text = labels
                    .get(world_label.0)
                    .expect("world label should contain text");
                let world_name = &text.sections[0].value;
                match dialog_button {
                    RemoveDialogButton::Remove => {
                        let world_path = game_paths.world_path(world_name);
                        fs::remove_file(&world_path)
                            .with_context(|| format!("unable to remove {world_path:?}"))?;
                        commands.entity(world_node.0).despawn_recursive();
                    }
                    RemoveDialogButton::Cancel => (),
                }
                commands.entity(dialog_entity).despawn_recursive();
            }
        }

        Ok(())
    }
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

#[derive(Clone, Component, Copy)]
struct WorldLabel(Entity);

#[derive(Clone, Component, Copy)]
struct WorldNode(Entity);
