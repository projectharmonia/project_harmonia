use anyhow::{Context, Result};
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use serde::Deserialize;

use super::errors::log_err_system;

pub(super) struct AssetMetadataPlugin;

impl Plugin for AssetMetadataPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AssetMetadata>()
            .init_asset_loader::<AssetMetadataLoader>()
            .add_startup_system(Self::load_metadata.chain(log_err_system));
    }
}

impl AssetMetadataPlugin {
    fn load_metadata(mut commands: Commands, asset_server: ResMut<AssetServer>) -> Result<()> {
        let handles = asset_server
            .load_folder("base")
            .context("Unable to load base game assets metadata")?;
        commands.insert_resource(MetadataHandles(handles));

        asset_server
            .watch_for_changes()
            .context("Unable to subscribe for listening for changes")
    }
}

#[derive(Default)]
pub struct AssetMetadataLoader;

impl AssetLoader for AssetMetadataLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let metadata = toml::from_slice::<AssetMetadata>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(metadata));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["toml"]
    }
}

#[derive(Deref, DerefMut)]
struct MetadataHandles(Vec<HandleUntyped>);

#[derive(Deserialize, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssetMetadata {
    Object(ObjectMetadata),
}

impl AssetMetadata {
    #[cfg_attr(coverage, no_coverage)]
    pub(crate) fn object(&self) -> Option<&ObjectMetadata> {
        match self {
            Self::Object(object) => Some(object),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct ObjectMetadata {
    pub(crate) name: String,
    pub(crate) category: ObjectCategory,
}

impl ObjectMetadata {
    #[cfg_attr(coverage, no_coverage)]
    pub(crate) fn is_placable_in_city(&self) -> bool {
        match self.category {
            ObjectCategory::Rocks => true,
        }
    }
}

#[derive(Deserialize)]
pub(crate) enum ObjectCategory {
    Rocks,
}

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetPlugin, core::CorePlugin};

    use super::*;

    #[test]
    fn loading_metadata() {
        let mut app = App::new();
        app.add_plugin(CorePlugin)
            .add_plugin(AssetPlugin)
            .add_plugin(AssetMetadataPlugin);

        app.update();

        assert!(
            !app.world.resource::<MetadataHandles>().is_empty(),
            "Handles should be populated with assets"
        );
    }
}
