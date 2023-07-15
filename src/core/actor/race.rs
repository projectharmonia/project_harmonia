pub(crate) mod human;

use bevy::{app::PluginGroupBuilder, prelude::*};
use std::fmt::Debug;

use human::HumanPlugin;

pub(super) struct RacePlugins;

impl PluginGroup for RacePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(HumanPlugin)
    }
}

#[reflect_trait]
pub(crate) trait RaceBundle: Debug + Reflect {
    fn glyph(&self) -> &'static str;
}
