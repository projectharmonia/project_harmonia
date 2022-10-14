use bevy_renet::{
    renet::{RenetClient, RenetServer},
    RenetClientPlugin, RenetServerPlugin,
};

use super::{network_event::NetworkEventCounter, *};
use crate::core::network::{client::ConnectionSettings, server::ServerSettings};

/// Preset for quickly testing networking.
#[derive(Clone, Copy)]
pub(crate) enum NetworkPreset {
    Server,
    Client,
    ServerAndClient { connected: bool },
}

/// Automates server and / or client creation for unit tests.
pub(crate) struct NetworkPresetPlugin {
    server: bool,
    client: bool,
    connected: bool,
}

impl Plugin for NetworkPresetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MinimalPlugins);

        let event_counter = *app
            .world
            .get_resource_or_insert_with(NetworkEventCounter::default);

        if self.server {
            let server_settings = ServerSettings {
                port: 0,
                ..Default::default()
            };

            app.insert_resource(
                server_settings
                    .create_server(event_counter)
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
                    .create_client(event_counter)
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

impl NetworkPresetPlugin {
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
