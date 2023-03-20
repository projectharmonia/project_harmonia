use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use anyhow::Result;
use bevy::prelude::*;
use bevy_renet::renet::{ClientAuthentication, RenetClient, RenetConnectionConfig};
use clap::Args;

use super::{network_event::NetworkEventCounter, DEFAULT_PORT, PROTOCOL_ID};

pub(super) struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionSettings>();
    }
}

#[derive(Args, Clone, Debug, PartialEq, Resource)]
pub(crate) struct ConnectionSettings {
    /// Server IP address.
    #[clap(short, long, default_value_t = ConnectionSettings::default().ip)]
    pub(crate) ip: String,

    /// Server port.
    #[clap(short, long, default_value_t = ConnectionSettings::default().port)]
    pub(crate) port: u16,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
        }
    }
}

impl ConnectionSettings {
    pub(crate) fn create_client(&self, event_counter: NetworkEventCounter) -> Result<RenetClient> {
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let client_id = current_time.as_millis() as u64;
        let ip = self.ip.parse()?;
        let server_addr = SocketAddr::new(ip, self.port);
        let socket = UdpSocket::bind((ip, 0))?;
        let authentication = ClientAuthentication::Unsecure {
            client_id,
            protocol_id: PROTOCOL_ID,
            server_addr,
            user_data: None,
        };

        let receive_channels_config = super::channel_configs(event_counter.server);
        let send_channels_config = super::channel_configs(event_counter.client);
        let connection_config = RenetConnectionConfig {
            send_channels_config,
            receive_channels_config,
            ..Default::default()
        };

        RenetClient::new(current_time, socket, connection_config, authentication)
            .map_err(From::from)
    }
}
