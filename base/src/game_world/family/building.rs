pub mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};
use wall::WallPlugin;

pub(super) struct BuildingPlugins;

impl PluginGroup for BuildingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(WallPlugin)
    }
}
