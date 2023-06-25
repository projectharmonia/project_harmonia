use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
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

use super::{game_state::GameState, game_world::WorldName};

pub(super) struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Transform>().replicate::<Name>().add_system(
            Self::client_connection_system.in_schedule(OnEnter(ClientState::Connected)),
        );
    }
}

impl NetworkPlugin {
    fn client_connection_system(
        mut commands: Commands,
        mut game_state: ResMut<NextState<GameState>>,
    ) {
        commands.insert_resource(WorldName::default());
        game_state.set(GameState::World);
    }
}

pub(crate) const DEFAULT_PORT: u16 = 4761;
const PROTOCOL_ID: u64 = 7;

pub(crate) fn create_server(
    port: u16,
    server_channels_config: Vec<ChannelConfig>,
    client_channels_config: Vec<ChannelConfig>,
) -> Result<(RenetServer, NetcodeServerTransport)> {
    let server = RenetServer::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        ..Default::default()
    });

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let public_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
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

pub(crate) fn create_client(
    ip: IpAddr,
    port: u16,
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
    let server_addr = SocketAddr::new(ip, port);
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
