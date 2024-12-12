use std::{fs, mem, net::Ipv4Addr};

use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{ConnectionConfig, RenetClient, RenetServer},
    RenetChannelsExt,
};
use bevy_simple_text_input::TextInputValue;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::MenuState;
use project_harmonia_base::{
    core::GameState,
    game_paths::GamePaths,
    game_world::{GameLoad, WorldName},
    message::error_message,
    network::{self, DEFAULT_PORT},
};
use project_harmonia_widgets::{
    button::TextButtonBundle, click::Click, dialog::Dialog, dialog::DialogBundle,
    label::LabelBundle, text_edit::TextEditBundle, theme::Theme,
};

pub(super) struct WorldBrowserPlugin;

impl Plugin for WorldBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuState::WorldBrowser), Self::setup)
            .add_systems(
                Update,
                (
                    Self::handle_world_clicks,
                    Self::handle_host_dialog_clicks.pipe(error_message),
                    Self::handle_remove_dialog_clicks.pipe(error_message),
                    Self::handle_back_clicks,
                    Self::handle_world_browser_clicks,
                    Self::handle_create_dialog_clicks,
                    Self::handle_join_dialog_clicks.pipe(error_message),
                )
                    .run_if(in_state(MenuState::WorldBrowser)),
            );
    }
}

impl WorldBrowserPlugin {
    fn setup(
        mut commands: Commands,
        theme: Res<Theme>,
        game_paths: Res<GamePaths>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("entering world browser");
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(MenuState::WorldBrowser),
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
                    parent.spawn(LabelBundle::large(&theme, "World browser"));
                    parent
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
                        .with_children(|parent| {
                            let world_names = game_paths
                                .get_world_names()
                                .map_err(|e| error!("unable to get world names: {e}"))
                                .unwrap_or_default();
                            for name in world_names {
                                setup_world_node(parent, &theme, name);
                            }
                        });

                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::FlexStart,
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn((BackButton, TextButtonBundle::normal(&theme, "Back")));
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                            for button in WorldBrowserButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(&theme, button.to_string()),
                                ));
                            }
                        });
                });
        });
    }

    fn handle_world_clicks(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        buttons: Query<(&WorldButton, &WorldNode)>,
        labels: Query<&Text>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        for (world_button, &world_node) in
            buttons.iter_many(click_events.read().map(|event| event.0))
        {
            let world_name = labels
                .get(world_node.label_entity)
                .expect("world label should contain text");
            match world_button {
                WorldButton::Play => {
                    commands.insert_resource(WorldName(world_name.sections[0].value.clone()));
                    load_events.send_default();
                }
                WorldButton::Host => {
                    commands.entity(roots.single()).with_children(|parent| {
                        setup_host_world_dialog(
                            parent,
                            &theme,
                            world_node,
                            &world_name.sections[0].value,
                        );
                    });
                }
                WorldButton::Remove => {
                    commands.entity(roots.single()).with_children(|parent| {
                        setup_remove_world_dialog(
                            parent,
                            &theme,
                            world_node,
                            &world_name.sections[0].value,
                        );
                    });
                }
            }
        }
    }

    fn handle_host_dialog_clicks(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        mut click_events: EventReader<Click>,
        network_channels: Res<RepliconChannels>,
        dialogs: Query<(Entity, &WorldNode), With<Dialog>>,
        buttons: Query<&HostDialogButton>,
        text_edits: Query<&TextInputValue, With<PortEdit>>,
        mut labels: Query<&mut Text>,
    ) -> Result<()> {
        for &button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            let (dialog_entity, world_node) = dialogs.single();
            match button {
                HostDialogButton::Host => {
                    let server = RenetServer::new(ConnectionConfig {
                        server_channels_config: network_channels.get_server_configs(),
                        client_channels_config: network_channels.get_client_configs(),
                        ..Default::default()
                    });
                    let port = text_edits.single();
                    let transport = network::create_server(port.0.parse()?)
                        .context("unable to create server")?;

                    commands.insert_resource(server);
                    commands.insert_resource(transport);

                    let mut world_name = labels
                        .get_mut(world_node.label_entity)
                        .expect("world label should contain text");
                    commands
                        .insert_resource(WorldName(mem::take(&mut world_name.sections[0].value)));

                    load_events.send_default();
                }
                HostDialogButton::Cancel => info!("cancelling hosting"),
            }
            commands.entity(dialog_entity).despawn_recursive();
        }

        Ok(())
    }

    fn handle_remove_dialog_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        game_paths: Res<GamePaths>,
        dialogs: Query<(Entity, &WorldNode), With<Dialog>>,
        buttons: Query<&RemoveDialogButton>,
        labels: Query<&Text>,
    ) -> Result<()> {
        for &button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            let (dialog_entity, world_node) = dialogs.single();
            let world_name = labels
                .get(world_node.label_entity)
                .expect("world label should contain text");
            match button {
                RemoveDialogButton::Remove => {
                    let world_path = game_paths.world_path(&world_name.sections[0].value);
                    fs::remove_file(&world_path)
                        .with_context(|| format!("unable to remove {world_path:?}"))?;
                    commands.entity(world_node.node_entity).despawn_recursive();
                }
                RemoveDialogButton::Cancel => info!("cancelling removal"),
            }
            commands.entity(dialog_entity).despawn_recursive();
        }

        Ok(())
    }

    fn handle_back_clicks(
        mut click_events: EventReader<Click>,
        mut menu_state: ResMut<NextState<MenuState>>,
        buttons: Query<(), With<BackButton>>,
    ) {
        for _ in buttons.iter_many(click_events.read().map(|event| event.0)) {
            menu_state.set(MenuState::MainMenu);
        }
    }

    fn handle_world_browser_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        buttons: Query<&WorldBrowserButton>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            commands
                .entity(roots.single())
                .with_children(|parent| match button {
                    WorldBrowserButton::Create => setup_create_world_dialog(parent, &theme),
                    WorldBrowserButton::Join => setup_join_world_dialog(parent, &theme),
                });
        }
    }

    fn handle_create_dialog_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<&CreateDialogButton>,
        mut text_edits: Query<&mut TextInputValue, With<WorldNameEdit>>,
        dialogs: Query<Entity, With<Dialog>>,
    ) {
        for &button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                CreateDialogButton::Create => {
                    let mut world_name = text_edits.single_mut();
                    commands.insert_resource(WorldName(mem::take(&mut world_name.0)));
                    game_state.set(GameState::InGame);
                }
                CreateDialogButton::Cancel => info!("cancelling creation"),
            }
            commands.entity(dialogs.single()).despawn_recursive();
        }
    }

    fn handle_join_dialog_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        network_channels: Res<RepliconChannels>,
        buttons: Query<&JoinDialogButton>,
        port_edits: Query<&TextInputValue, With<PortEdit>>,
        ip_edits: Query<&TextInputValue, With<IpEdit>>,
        dialogs: Query<Entity, With<Dialog>>,
    ) -> Result<()> {
        for &button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                JoinDialogButton::Join => {
                    let client = RenetClient::new(ConnectionConfig {
                        server_channels_config: network_channels.get_server_configs(),
                        client_channels_config: network_channels.get_client_configs(),
                        ..Default::default()
                    });
                    let ip = ip_edits.single();
                    let port = port_edits.single();
                    let transport = network::create_client(ip.0.parse()?, port.0.parse()?)
                        .context("unable to create connection")?;

                    commands.insert_resource(client);
                    commands.insert_resource(transport);
                }
                JoinDialogButton::Cancel => {
                    info!("cancelling join");
                    commands.entity(dialogs.single()).despawn_recursive();
                }
            }
        }

        Ok(())
    }
}

fn setup_world_node(parent: &mut ChildBuilder, theme: &Theme, label: impl Into<String>) {
    parent
        .spawn(NodeBundle {
            style: Style {
                padding: theme.padding.normal,
                column_gap: theme.gap.normal,
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
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .add_child(label_entity);
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

fn setup_host_world_dialog(
    parent: &mut ChildBuilder,
    theme: &Theme,
    world_node: WorldNode,
    world_name: &str,
) {
    info!("showing host dialog");
    parent
        .spawn((DialogBundle::new(theme), world_node))
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
                    parent.spawn(LabelBundle::normal(theme, format!("Host {world_name}")));

                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                column_gap: theme.gap.normal,
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn(LabelBundle::normal(theme, "Port:"));
                            parent.spawn((
                                PortEdit,
                                TextEditBundle::new(theme, DEFAULT_PORT.to_string()),
                            ));
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
                            for button in HostDialogButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(theme, button.to_string()),
                                ));
                            }
                        });
                });
        });
}

fn setup_remove_world_dialog(
    parent: &mut ChildBuilder,
    theme: &Theme,
    world_node: WorldNode,
    world_name: &str,
) {
    info!("showing remove dialog");
    parent
        .spawn((DialogBundle::new(theme), world_node))
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
                    parent.spawn(LabelBundle::normal(
                        theme,
                        format!("Are you sure you want to remove world {world_name}?",),
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
                            for button in RemoveDialogButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(theme, button.to_string()),
                                ));
                            }
                        });
                });
        });
}

fn setup_create_world_dialog(parent: &mut ChildBuilder, theme: &Theme) {
    info!("showing create dialog");
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
                    parent.spawn(LabelBundle::normal(theme, "Create world"));
                    parent.spawn((WorldNameEdit, TextEditBundle::new(theme, "New world")));
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            for button in CreateDialogButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(theme, button.to_string()),
                                ));
                            }
                        });
                });
        });
}

fn setup_join_world_dialog(parent: &mut ChildBuilder, theme: &Theme) {
    info!("showing join dialog");
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
                    parent.spawn(LabelBundle::normal(theme, "Join world"));

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
                            parent.spawn(LabelBundle::normal(theme, "IP:"));
                            parent.spawn((
                                IpEdit,
                                TextEditBundle::new(theme, Ipv4Addr::LOCALHOST.to_string()),
                            ));

                            parent.spawn(LabelBundle::normal(theme, "Port:"));
                            parent.spawn((
                                PortEdit,
                                TextEditBundle::new(theme, DEFAULT_PORT.to_string())
                                    .inactive(theme),
                            ));
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
                            for button in JoinDialogButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(theme, button.to_string()),
                                ));
                            }
                        });
                });
        });
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum WorldButton {
    Play,
    Host,
    Remove,
}

#[derive(Component, EnumIter, Clone, Copy, Display, PartialEq)]
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

#[derive(Component)]
struct BackButton;

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum WorldBrowserButton {
    Create,
    Join,
}

#[derive(Component, EnumIter, Clone, Copy, Display, PartialEq)]
enum CreateDialogButton {
    Create,
    Cancel,
}

#[derive(Component)]
struct WorldNameEdit;

#[derive(Component)]
struct PortEdit;

#[derive(Component)]
struct IpEdit;

#[derive(Component, EnumIter, Clone, Copy, Display, PartialEq)]
enum HostDialogButton {
    Host,
    Cancel,
}

#[derive(Component, EnumIter, Clone, Copy, Display, PartialEq)]
enum JoinDialogButton {
    Join,
    Cancel,
}
