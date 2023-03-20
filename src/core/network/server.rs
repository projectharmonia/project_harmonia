use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use anyhow::Result;
use bevy::prelude::*;
use bevy_renet::renet::{RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig};
use clap::Args;

use super::{network_event::NetworkEventCounter, DEFAULT_PORT, PROTOCOL_ID};

pub(crate) const SERVER_ID: u64 = 0;
pub(super) const TICK_TIME: Duration = Duration::from_millis(100);

pub(super) struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServerSettings>();
    }
}

#[derive(Args, Clone, Debug, PartialEq, Resource)]
pub(crate) struct ServerSettings {
    /// Server name that will be visible to other players.
    #[clap(short, long, default_value_t = ServerSettings::default().server_name)]
    pub(crate) server_name: String,

    /// Port to use.
    #[clap(short, long, default_value_t = ServerSettings::default().port)]
    pub(crate) port: u16,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            server_name: "My game".to_string(),
            port: DEFAULT_PORT,
        }
    }
}

impl ServerSettings {
    pub(crate) fn create_server(&self, event_counter: NetworkEventCounter) -> Result<RenetServer> {
        const MAX_CLIENTS: usize = 32;
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), self.port);
        let socket = UdpSocket::bind(server_addr)?;
        let server_config = ServerConfig::new(
            MAX_CLIENTS,
            PROTOCOL_ID,
            socket.local_addr()?,
            ServerAuthentication::Unsecure,
        );

        let receive_channels_config = super::channel_configs(event_counter.client);
        let send_channels_config = super::channel_configs(event_counter.server);
        let connection_config = RenetConnectionConfig {
            send_channels_config,
            receive_channels_config,
            ..Default::default()
        };

        RenetServer::new(current_time, server_config, connection_config, socket).map_err(From::from)
    }
}
