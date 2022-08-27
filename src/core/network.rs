pub(super) mod client;
pub(super) mod server;

use bevy::prelude::*;
use bevy_renet::renet::{ChannelConfig, ReliableChannelConfig};

use self::{client::ClientPlugin, server::ServerPlugin};

pub(super) struct NetworkPlugins;

impl PluginGroup for NetworkPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(ServerPlugin).add(ClientPlugin);
    }
}

const DEFAULT_PORT: u16 = 4761;
const PROTOCOL_ID: u64 = 7;
const MAX_CLIENTS: usize = 32;

enum Channel {
    Reliable,
}

impl Channel {
    fn config() -> Vec<ChannelConfig> {
        let reliable_channel = ChannelConfig::Reliable(ReliableChannelConfig {
            channel_id: Channel::Reliable as u8,
            ..Default::default()
        });
        vec![reliable_channel]
    }
}
