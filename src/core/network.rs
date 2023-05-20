use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use anyhow::Result;
use bevy::prelude::*;
use bevy_replicon::{
    prelude::*,
    renet::{
        transport::{
            ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport,
            ServerAuthentication, ServerConfig,
        },
        ChannelConfig, ConnectionConfig, RenetClient, RenetServer,
    },
};
use clap::Args;

use super::{game_state::GameState, game_world::WorldName};

pub(super) struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Transform>()
            .replicate::<Name>()
            .init_resource::<ServerSettings>()
            .init_resource::<ConnectionSettings>()
            .add_system(
                Self::client_world_creation_system.in_schedule(OnEnter(ClientState::Connected)),
            );
    }
}

impl NetworkPlugin {
    fn client_world_creation_system(
        mut commands: Commands,
        mut game_state: ResMut<NextState<GameState>>,
    ) {
        commands.insert_resource(WorldName::default());
        game_state.set(GameState::World);
    }
}

const DEFAULT_PORT: u16 = 4761;
const PROTOCOL_ID: u64 = 7;

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
    pub(crate) fn create_server(
        &self,
        server_channels_config: Vec<ChannelConfig>,
        client_channels_config: Vec<ChannelConfig>,
    ) -> Result<(RenetServer, NetcodeServerTransport)> {
        let server = RenetServer::new(ConnectionConfig {
            server_channels_config,
            client_channels_config,
            ..Default::default()
        });

        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let public_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), self.port);
        let socket = UdpSocket::bind(public_addr)?;
        let server_config = ServerConfig {
            max_clients: 1,
            protocol_id: PROTOCOL_ID,
            public_addr,
            authentication: ServerAuthentication::Unsecure,
        };
        let transport = NetcodeServerTransport::new(current_time, server_config, socket)?;

        Ok((server, transport))
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
    pub(crate) fn create_client(
        &self,
        server_channels_config: Vec<ChannelConfig>,
        client_channels_config: Vec<ChannelConfig>,
    ) -> Result<(RenetClient, NetcodeClientTransport)> {
        let client = RenetClient::new(ConnectionConfig {
            server_channels_config,
            client_channels_config,
            ..Default::default()
        });

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
        let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;

        Ok((client, transport))
    }
}
