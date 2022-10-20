use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::Result;
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadContext, LoadedAsset},
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
                        .unwrap_or_else(|e| panic!("entries should start with {folder:?}: {e}"));
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

/// Converts metadata path (path to a TOML file) into
/// the corresponding scene path loadable by [`AssetServer`].
///
/// # Panics
///
/// Panics if path is an invalid UTF-8 string.
pub(crate) fn scene_path<P: AsRef<Path>>(metadata_path: P) -> String {
    let mut scene_path = metadata_path
        .as_ref()
        .with_extension("gltf")
        .into_os_string()
        .into_string()
        .expect("resource metadata path should be a UTF-8 string");

    scene_path += "#Scene0";
    scene_path
}

#[derive(Deref, DerefMut)]
struct MetadataHandles(Vec<Handle<AssetMetadata>>);

#[derive(Deserialize, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub(crate) struct AssetMetadata {
    pub(crate) general: GeneralMetadata,
    #[serde(flatten)]
    pub(crate) kind: MetadataKind,
}

#[derive(Deserialize)]
pub(crate) struct GeneralMetadata {
    pub(crate) name: String,
    pub(crate) preview_translation: Vec3,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MetadataKind {
    Object(ObjectMetadata),
}

#[derive(Deserialize)]
pub(crate) struct ObjectMetadata {
    pub(crate) category: ObjectCategory,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) enum ObjectCategory {
    Rocks,
    Foliage,
    #[serde(rename = "Outdoor furniture")]
    OutdoorFurniture,
}

impl ObjectCategory {
    #[cfg_attr(coverage, no_coverage)]
    pub(crate) fn is_placable_in_city(self) -> bool {
        match self {
            ObjectCategory::Rocks | ObjectCategory::Foliage | ObjectCategory::OutdoorFurniture => true,
        }
    }
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
            "handles should be populated with assets"
        );
    }
}
