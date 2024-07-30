pub mod lot;
pub(super) mod spline;
pub mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};

use lot::LotPlugin;
use spline::SplinePlugin;
use wall::WallPlugin;

pub(super) struct BuildingPlugins;

impl PluginGroup for BuildingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LotPlugin)
            .add(SplinePlugin)
            .add(WallPlugin)
    }
}
