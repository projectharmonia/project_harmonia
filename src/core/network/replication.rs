mod removal_tracker;
mod replication_message;
mod world_diff;

use bevy::{app::PluginGroupBuilder, prelude::*};

use removal_tracker::RemovalTrackerPlugin;
use replication_message::ReplicationMessagePlugin;

pub(super) struct ReplicationPlugins;

impl PluginGroup for ReplicationPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(RemovalTrackerPlugin)
            .add(ReplicationMessagePlugin);
    }
}
