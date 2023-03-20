use bevy::prelude::*;
use bevy_renet::renet::{RenetClient, RenetServer};

pub(super) struct NetworkSetsPlugin;

impl Plugin for NetworkSetsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets((
            NetworkSet::Authoritve.run_if(not(resource_exists::<RenetClient>())),
            NetworkSet::Server.run_if(resource_exists::<RenetServer>()),
            NetworkSet::ClientConnected.run_if(bevy_renet::client_connected),
            NetworkSet::ClientConnecting.run_if(bevy_renet::client_connecting),
        ));
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum NetworkSet {
    Authoritve,
    Server,
    ClientConnecting,
    ClientConnected,
}
