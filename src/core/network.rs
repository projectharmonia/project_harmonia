pub(crate) mod client;
pub(super) mod entity_serde;
pub(crate) mod network_event;
#[cfg(test)]
pub(crate) mod network_preset;
pub(super) mod replication;
pub(crate) mod server;

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
pub(super) const SERVER_ID: u64 = 0;
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
