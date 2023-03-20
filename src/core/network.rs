pub(crate) mod client;
pub(crate) mod network_event;
#[cfg(test)]
pub(crate) mod network_preset;
pub(super) mod replication;
pub(crate) mod server;
pub(crate) mod sets;

use bevy::prelude::*;
use bevy_renet::renet::{ChannelConfig, ReliableChannelConfig, UnreliableChannelConfig};

use client::ClientPlugin;
use replication::ReplicationPlugin;
use server::ServerPlugin;
use sets::NetworkSetsPlugin;

pub(super) struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(NetworkSetsPlugin)
            .add_plugin(ServerPlugin)
            .add_plugin(ClientPlugin)
            .add_plugin(ReplicationPlugin);
    }
}

const DEFAULT_PORT: u16 = 4761;
const PROTOCOL_ID: u64 = 7;
const REPLICATION_CHANNEL_ID: u8 = 0;

fn channel_configs(events_count: u8) -> Vec<ChannelConfig> {
    let mut channel_configs = Vec::with_capacity((events_count + 1).into());
    channel_configs.push(ChannelConfig::Unreliable(UnreliableChannelConfig {
        channel_id: REPLICATION_CHANNEL_ID,
        ..Default::default()
    }));
    for channel_id in 1..=events_count {
        channel_configs.push(ChannelConfig::Reliable(ReliableChannelConfig {
            channel_id: REPLICATION_CHANNEL_ID + channel_id,
            ..Default::default()
        }));
    }
    channel_configs
}
