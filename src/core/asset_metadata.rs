use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use bevy::{
    asset::{AssetLoader, AssetServerSettings, HandleId, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use serde::Deserialize;
use walkdir::WalkDir;

const EXTENSION: &str = "toml";

pub(super) struct AssetMetadataPlugin;

impl Plugin for AssetMetadataPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AssetMetadata>()
            .init_asset_loader::<AssetMetadataLoader>()
            .add_startup_system(Self::load_metadata);
    }
}

impl AssetMetadataPlugin {
    fn load_metadata(
        mut commands: Commands,
        asset_server: ResMut<AssetServer>,
        settings: Res<AssetServerSettings>,
    ) {
        let mut folder: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap_or_default().into();
        folder.push(&settings.asset_folder);

        let mut handles = Vec::new();
        for entry in WalkDir::new(&folder)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if let Some(extension) = entry.path().extension() {
                if extension == EXTENSION {
                    let path = entry
                        .path()
                        .strip_prefix(&folder)
                        .unwrap_or_else(|e| panic!("Entries should start with {folder:?}: {e:#}"));
                    handles.push(asset_server.load::<AssetMetadata, _>(path));
                }
            }
        }

        commands.insert_resource(MetadataHandles(handles));
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
        &[EXTENSION]
    }
}

pub(crate) trait AssetServerMetadataExt {
    /// Loads a scene corresponding to a metadata.
    fn load_from_metadata(&self, metadata: HandleId) -> Result<Handle<Scene>>;
}

impl AssetServerMetadataExt for AssetServer {
    fn load_from_metadata(&self, metadata: HandleId) -> Result<Handle<Scene>> {
        let metadata_path = self
            .get_handle_path(metadata)
            .context("Unable to get metadata path")?;

        let mut scene_path = metadata_path
            .path()
            .with_extension("gltf")
            .into_os_string()
            .into_string()
            .ok()
            .context("Not a UTF-8 asset path")?;

        scene_path += "#Scene0";
        debug!("Loading {scene_path} to generate preview");

        Ok(self.load(&scene_path))
    }
}

#[derive(Deref, DerefMut)]
struct MetadataHandles(Vec<Handle<AssetMetadata>>);

#[derive(Deserialize, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssetMetadata {
    Object(ObjectMetadata),
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