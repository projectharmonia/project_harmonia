mod despawn_tracker;
mod removal_tracker;
mod replication_messaging;
mod world_diff;

use bevy::{app::PluginGroupBuilder, prelude::*};

use despawn_tracker::DespawnTrackerPlugin;
use removal_tracker::RemovalTrackerPlugin;
use replication_messaging::ReplicationMessagingPlugin;

pub(super) struct ReplicationPlugins;

impl PluginGroup for ReplicationPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(RemovalTrackerPlugin)
            .add(DespawnTrackerPlugin)
            .add(ReplicationMessagingPlugin);
    }
}
