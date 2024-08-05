pub mod lot;
pub mod road;
pub(super) mod spline;
pub mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};

use lot::LotPlugin;
use road::RoadPlugin;
use spline::SplinePlugin;
use wall::WallPlugin;

pub(super) struct BuildingPlugins;

impl PluginGroup for BuildingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LotPlugin)
            .add(SplinePlugin)
            .add(WallPlugin)
            .add(RoadPlugin)
    }
}
