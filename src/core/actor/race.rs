pub(crate) mod human;

use std::any::TypeId;

use bevy::{app::PluginGroupBuilder, prelude::*, reflect::GetTypeRegistration, utils::HashMap};
use bevy_replicon::replication_core::AppReplicationExt;
use bevy_trait_query::{queryable, RegisterExt};

use human::HumanPlugin;

pub(super) struct RacePlugins;

impl PluginGroup for RacePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(HumanPlugin)
    }
}

#[queryable]
pub(crate) trait Race: Reflect {
    fn glyph(&self) -> &'static str;
}

trait RaceExt {
    fn register_race<T, B>(&mut self) -> &mut Self
    where
        T: Race + Component + GetTypeRegistration,
        B: Bundle + GetTypeRegistration;
}

impl RaceExt for App {
    /// Registers the race `T` and maps bundle `B` to it.
    ///
    /// Mapped bundle `TypeId` can be obtained later from [`RaceComponents`]
    fn register_race<T, B>(&mut self) -> &mut Self
    where
        T: Race + Component + GetTypeRegistration,
        B: Bundle + GetTypeRegistration,
    {
        self.world
            .get_resource_or_insert_with::<RaceComponents>(Default::default)
            .insert(TypeId::of::<T>(), TypeId::of::<B>());
        self.register_type::<B>()
            .replicate::<T>()
            .register_component_as::<dyn Race, T>()
    }
}

/// Maps race components to their bundles.
#[derive(Deref, DerefMut, Resource, Default)]
pub(crate) struct RaceComponents(HashMap<TypeId, TypeId>);
