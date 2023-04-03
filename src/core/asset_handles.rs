use bevy::{asset::Asset, prelude::*};
use strum::IntoEnumIterator;

/// Resource to preload assets and keep them loaded until the resource exists.
#[derive(Resource)]
pub(super) struct AssetHandles<T: AssetCollection>(Vec<Handle<T::AssetType>>);

impl<T: AssetCollection + IntoEnumIterator> FromWorld for AssetHandles<T> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let handles = T::iter()
            .map(|value| asset_server.load(value.asset_path()))
            .collect();
        Self(handles)
    }
}

impl<T: AssetCollection + Into<usize>> AssetHandles<T> {
    pub(super) fn handle(&self, value: T) -> Handle<T::AssetType> {
        self.0[value.into()].clone()
    }
}

/// Associates type with asset collection.
pub(super) trait AssetCollection {
    type AssetType: Asset;

    /// Returns associated asset path based on the current value.
    fn asset_path(&self) -> &'static str;
}
