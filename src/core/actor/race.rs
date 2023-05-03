pub(crate) mod human;

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_trait_query::queryable;

use human::HumanPlugin;

pub(super) struct RacePlugins;

impl PluginGroup for RacePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(HumanPlugin)
    }
}

#[queryable]
#[reflect_trait]
pub(crate) trait Race: Reflect {
    fn glyph(&self) -> &'static str;
}
