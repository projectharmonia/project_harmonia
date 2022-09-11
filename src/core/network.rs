pub(super) mod client;
pub(super) mod replication;
pub(super) mod server;

use bevy::prelude::*;
use bevy_renet::renet::{ChannelConfig, ReliableChannelConfig, UnreliableChannelConfig};

use client::ClientPlugin;
use replication::ReplicationPlugins;
use server::ServerPlugin;

pub(super) struct NetworkPlugins;

impl PluginGroup for NetworkPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(ServerPlugin).add(ClientPlugin);

        ReplicationPlugins.build(group)
    }
}

const DEFAULT_PORT: u16 = 4761;
const PROTOCOL_ID: u64 = 7;
const MAX_CLIENTS: usize = 32;

enum Channel {
    Reliable,
    Replication,
}

impl Channel {
    fn config() -> Vec<ChannelConfig> {
        let reliable_channel = ChannelConfig::Reliable(ReliableChannelConfig {
            channel_id: Channel::Reliable as u8,
            ..Default::default()
        });
        let replication_channel = ChannelConfig::Unreliable(UnreliableChannelConfig {
            channel_id: Channel::Replication as u8,
            ..Default::default()
        });
        vec![reliable_channel, replication_channel]
    }
}

#[cfg(test)]
mod tests {
    use bevy_renet::{
        renet::{RenetClient, RenetServer},
        RenetClientPlugin, RenetServerPlugin,
    };

    use super::*;
    use crate::core::network::{client::ConnectionSettings, server::ServerSettings};

    /// Preset for quickly testing networking
    #[derive(Clone, Copy)]
    pub(crate) enum NetworkPreset {
        Server,
        Client,
        ServerAndClient { connected: bool },
    }

    /// Automates server and / or client creation for unit tests
    pub(crate) struct TestNetworkPlugin {
        server: bool,
        client: bool,
        connected: bool,
    }

    impl Plugin for TestNetworkPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins(MinimalPlugins);

            if self.server {
                let server_settings = ServerSettings {
                    port: 0,
                    ..Default::default()
                };

                app.insert_resource(
                    server_settings
                        .create_server()
                        .expect("server should be created"),
                )
                .add_plugin(RenetServerPlugin);
            }

            if self.client {
                let connection_settings = ConnectionSettings {
                    port: if self.server {
                        app.world.resource::<RenetServer>().addr().port()
                    } else {
                        0
                    },
                    ..Default::default()
                };

                app.insert_resource(
                    connection_settings
                        .create_client()
                        .expect("client should be created"),
                )
                .add_plugin(RenetClientPlugin);
            }

            if self.connected {
                app.update();
                app.update();
                app.update();
                assert!(app.world.resource::<RenetClient>().is_connected());
            }
        }
    }

    impl TestNetworkPlugin {
        pub(crate) fn new(preset: NetworkPreset) -> Self {
            match preset {
                NetworkPreset::Server => Self {
                    server: true,
                    client: false,
                    connected: false,
                },
                NetworkPreset::Client => Self {
                    server: false,
                    client: true,
                    connected: false,
                },
                NetworkPreset::ServerAndClient { connected } => Self {
                    server: true,
                    client: true,
                    connected,
                },
            }
        }
    }
}
