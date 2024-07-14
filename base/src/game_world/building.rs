pub mod lot;
pub mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};

use lot::LotPlugin;
use wall::WallPlugin;

pub(super) struct BuildingPlugins;

impl PluginGroup for BuildingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LotPlugin)
            .add(WallPlugin)
    }
}
