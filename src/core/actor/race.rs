pub(crate) mod human;

use bevy::{app::PluginGroupBuilder, prelude::*, reflect::GetTypeRegistration};
use bevy_replicon::prelude::*;
use bevy_trait_query::{queryable, RegisterExt};

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

trait AppRaceExt {
    fn register_race<T: Race + GetTypeRegistration + Component>(&mut self) -> &mut Self;
}

impl AppRaceExt for App {
    fn register_race<T: Race + GetTypeRegistration + Component>(&mut self) -> &mut Self {
        self.replicate::<T>().register_component_as::<dyn Race, T>()
    }
}
