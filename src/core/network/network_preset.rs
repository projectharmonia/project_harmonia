use bevy_renet::{
    renet::{RenetClient, RenetServer},
    RenetClientPlugin, RenetServerPlugin,
};

use super::{network_event::NetworkEventCounter, *};
use crate::core::network::{client::ConnectionSettings, server::ServerSettings};

/// Automates server and / or client creation for unit tests.
pub(super) struct NetworkPresetPlugin {
    pub(super) client: bool,
    pub(super) server: bool,
}

impl NetworkPresetPlugin {
    /// Creates only client.
    #[allow(dead_code)]
    pub(super) fn client() -> Self {
        Self {
            client: true,
            server: false,
        }
    }

    /// Creates only server.
    pub(super) fn server() -> Self {
        Self {
            client: false,
            server: true,
        }
    }

    /// Creates client connected to server.
    pub(super) fn client_and_server() -> Self {
        Self {
            client: true,
            server: true,
        }
    }
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
            .add_plugin(RenetServerPlugin::default());
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
            .add_plugin(RenetClientPlugin::default());
        }

        if self.client && self.server {
            loop {
                app.update();
                if app.world.resource::<RenetClient>().is_connected() {
                    break;
                }
            }
        }
    }
}
