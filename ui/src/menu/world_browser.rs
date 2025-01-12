use std::{fs, net::Ipv4Addr};

use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{ConnectionConfig, RenetClient, RenetServer},
    RenetChannelsExt,
};
use bevy_simple_text_input::TextInputValue;

use super::MenuState;
use project_harmonia_base::{
    core::GameState,
    error_message::error_message,
    game_paths::GamePaths,
    game_world::{GameLoad, WorldName},
    network::{self, DEFAULT_PORT},
};
use project_harmonia_widgets::{
    button::ButtonKind, dialog::Dialog, label::LabelKind, text_edit::TextEdit, theme::Theme,
};

pub(super) struct WorldBrowserPlugin;

impl Plugin for WorldBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuState::WorldBrowser), Self::setup);
    }
}

impl WorldBrowserPlugin {
    fn setup(
        mut commands: Commands,
        theme: Res<Theme>,
        game_paths: Res<GamePaths>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("entering world browser");
        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(MenuState::WorldBrowser),
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
                    parent.spawn((LabelKind::Large, Text::new("World browser")));
                    parent
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
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::FlexStart,
                            column_gap: theme.gap.normal,
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(ButtonKind::Normal)
                                .with_child(Text::new("Back"))
                                .observe(Self::back);
                            parent.spawn(Node {
                                width: Val::Percent(100.0),
                                ..Default::default()
                            });
                            parent
                                .spawn(ButtonKind::Normal)
                                .with_child(Text::new("Create"))
                                .observe(Self::create);
                            parent
                                .spawn(ButtonKind::Normal)
                                .with_child(Text::new("Join"))
                                .observe(Self::join);
                        });
                });
        });
    }

    fn play(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        buttons: Query<&WorldNode>,
        labels: Query<&Text>,
    ) {
        let world_node = buttons.get(trigger.entity()).unwrap();
        let world_name = labels
            .get(world_node.label_entity)
            .expect("world label should contain text");

        commands.insert_resource(WorldName(world_name.0.clone()));
        commands.trigger(GameLoad);
    }

    fn host(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
        buttons: Query<&WorldNode>,
        labels: Query<&Text>,
    ) {
        let &world_node = buttons.get(trigger.entity()).unwrap();
        let world_name = labels
            .get(world_node.label_entity)
            .expect("world label should contain text");

        commands.entity(*root_entity).with_children(|parent| {
            info!("showing host dialog");
            parent.spawn((Dialog, world_node)).with_children(|parent| {
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
                        parent.spawn((
                            LabelKind::Normal,
                            Text::new(format!("Host {}", &**world_name)),
                        ));

                        parent
                            .spawn(Node {
                                column_gap: theme.gap.normal,
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn((LabelKind::Normal, Text::new("Port:")));
                                parent.spawn((PortEdit, TextInputValue(DEFAULT_PORT.to_string())));
                            });

                        parent
                            .spawn(Node {
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Host"))
                                    .observe(Self::confirm_host.pipe(error_message));
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Cancel"))
                                    .observe(Self::cancel_host);
                            });
                    });
            });
        });
    }

    fn remove(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
        buttons: Query<&WorldNode>,
        labels: Query<&Text>,
    ) {
        let &world_node = buttons.get(trigger.entity()).unwrap();
        let world_name = labels
            .get(world_node.label_entity)
            .expect("world label should contain text");

        commands.entity(*root_entity).with_children(|parent| {
            info!("showing remove dialog");
            parent.spawn((Dialog, world_node)).with_children(|parent| {
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
                        parent.spawn((
                            LabelKind::Normal,
                            Text::new(format!(
                                "Are you sure you want to remove world {}?",
                                &**world_name
                            )),
                        ));

                        parent
                            .spawn(Node {
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Remove"))
                                    .observe(Self::confirm_remove.pipe(error_message));
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Cancel"))
                                    .observe(Self::cancel_remove);
                            });
                    });
            });
        });
    }

    fn confirm_host(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        network_channels: Res<RepliconChannels>,
        dialog: Single<(Entity, &WorldNode), With<Dialog>>,
        port: Single<&TextInputValue, With<PortEdit>>,
        labels: Query<&Text>,
    ) -> Result<()> {
        let (dialog_entity, world_node) = *dialog;

        let server = RenetServer::new(ConnectionConfig {
            server_channels_config: network_channels.get_server_configs(),
            client_channels_config: network_channels.get_client_configs(),
            ..Default::default()
        });
        let transport =
            network::create_server(port.0.parse()?).context("unable to create server")?;

        commands.insert_resource(server);
        commands.insert_resource(transport);

        let world_name = labels
            .get(world_node.label_entity)
            .expect("world label should contain text");
        commands.insert_resource(WorldName(world_name.0.clone()));
        commands.trigger(GameLoad);

        commands.entity(dialog_entity).despawn_recursive();

        Ok(())
    }

    fn cancel_host(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn confirm_remove(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        game_paths: Res<GamePaths>,
        dialogs: Single<(Entity, &WorldNode), With<Dialog>>,
        labels: Query<&Text>,
    ) -> Result<()> {
        let (dialog_entity, world_node) = *dialogs;

        let world_name = labels
            .get(world_node.label_entity)
            .expect("world label should contain text");
        let world_path = game_paths.world_path(world_name);
        fs::remove_file(&world_path).with_context(|| format!("unable to remove {world_path:?}"))?;

        commands.entity(world_node.node_entity).despawn_recursive();
        commands.entity(dialog_entity).despawn_recursive();

        Ok(())
    }

    fn cancel_remove(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        info!("cancelling removal");
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn back(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
        commands.set_state(MenuState::MainMenu);
    }

    fn create(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        commands.entity(*root_entity).with_children(|parent| {
            info!("showing create dialog");
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
                        parent.spawn((LabelKind::Normal, Text::new("Create world")));
                        parent.spawn((TextEdit, TextInputValue("New world".to_string())));
                        parent
                            .spawn(Node {
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Create"))
                                    .observe(Self::confirm_create);
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Cancel"))
                                    .observe(Self::cancel_create);
                            });
                    });
            });
        });
    }

    fn join(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        commands.entity(*root_entity).with_children(|parent| {
            info!("showing join dialog");
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
                        parent.spawn((LabelKind::Normal, Text::new("Join world")));

                        parent
                            .spawn(Node {
                                display: Display::Grid,
                                column_gap: theme.gap.normal,
                                row_gap: theme.gap.normal,
                                grid_template_columns: vec![GridTrack::auto(); 2],
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn((LabelKind::Normal, Text::new("IP:")));
                                parent.spawn((
                                    IpEdit,
                                    TextInputValue(Ipv4Addr::LOCALHOST.to_string()),
                                ));

                                parent.spawn((LabelKind::Normal, Text::new("Port:")));
                                parent.spawn((PortEdit, TextInputValue(DEFAULT_PORT.to_string())));
                            });

                        parent
                            .spawn(Node {
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Join"))
                                    .observe(Self::confirm_join.pipe(error_message));
                                parent
                                    .spawn(ButtonKind::Normal)
                                    .with_child(Text::new("Cancel"))
                                    .observe(Self::cancel_join);
                            });
                    });
            });
        });
    }

    fn confirm_create(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        world_name: Single<&TextInputValue>,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        commands.insert_resource(WorldName(world_name.0.clone()));
        commands.set_state(GameState::InGame);
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn cancel_create(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        info!("cancelling creation");
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn confirm_join(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        network_channels: Res<RepliconChannels>,
        port: Single<&TextInputValue, With<PortEdit>>,
        ip: Single<&TextInputValue, With<IpEdit>>,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) -> Result<()> {
        let client = RenetClient::new(ConnectionConfig {
            server_channels_config: network_channels.get_server_configs(),
            client_channels_config: network_channels.get_client_configs(),
            ..Default::default()
        });
        let transport = network::create_client(port.0.parse()?, ip.0.parse()?)
            .context("unable to create connection")?;

        commands.insert_resource(client);
        commands.insert_resource(transport);
        commands.entity(*dialog_entity).despawn_recursive(); // Despawn only on transport creation.

        Ok(())
    }

    fn cancel_join(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<Dialog>>,
    ) {
        info!("cancelling join");
        commands.entity(*dialog_entity).despawn_recursive();
    }
}

fn setup_world_node(parent: &mut ChildBuilder, theme: &Theme, label: impl Into<String>) {
    parent
        .spawn((
            Node {
                padding: theme.padding.normal,
                column_gap: theme.gap.normal,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            let node_entity = parent.parent_entity();
            let label_entity = parent.spawn((LabelKind::Large, Text::new(label))).id();
            let world_node = WorldNode {
                label_entity,
                node_entity,
            };

            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                })
                .add_child(label_entity);
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: theme.gap.normal,
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn((ButtonKind::Normal, world_node))
                        .with_child(Text::new("Play"))
                        .observe(WorldBrowserPlugin::play);
                    parent
                        .spawn((ButtonKind::Normal, world_node))
                        .with_child(Text::new("Host"))
                        .observe(WorldBrowserPlugin::host);
                    parent
                        .spawn((ButtonKind::Normal, world_node))
                        .with_child(Text::new("Remove"))
                        .observe(WorldBrowserPlugin::remove);
                });
        });
}

/// Associated world node entities.
#[derive(Clone, Component, Copy)]
struct WorldNode {
    label_entity: Entity,
    node_entity: Entity,
}

#[derive(Component)]
#[require(TextEdit)]
struct PortEdit;

#[derive(Component)]
#[require(TextEdit)]
struct IpEdit;
