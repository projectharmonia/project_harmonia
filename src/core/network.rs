use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

use anyhow::Result;
use bevy::prelude::*;
use bevy_replicon::{
    client_just_connected,
    prelude::*,
    renet::transport::{
        ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
        ServerConfig,
    },
};

use super::{game_state::GameState, game_world::WorldName};

pub(super) struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Replication>()
            .register_type::<Transform>()
            .register_type::<Name>()
            .replicate::<Transform>()
            .replicate::<Name>()
            .add_systems(
                Update,
                Self::client_connection_system.run_if(client_just_connected()),
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

pub(crate) fn create_server(port: u16) -> Result<NetcodeServerTransport> {
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let public_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let socket = UdpSocket::bind(public_addr)?;
    let server_config = ServerConfig {
        current_time,
        max_clients: 1,
        protocol_id: PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
        public_addresses: vec![public_addr],
    };
    let transport = NetcodeServerTransport::new(server_config, socket)?;

    Ok(transport)
}

pub(crate) fn create_client(ip: IpAddr, port: u16) -> Result<NetcodeClientTransport> {
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

    Ok(transport)
}
