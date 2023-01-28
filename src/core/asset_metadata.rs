use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::Result;
use bevy::{
    asset::{AssetLoader, AssetPlugin, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};
use serde::Deserialize;
use strum::Display;
use walkdir::WalkDir;

const METADATA_EXTENSION: &str = "toml";

pub(super) struct AssetMetadataPlugin;

impl Plugin for AssetMetadataPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<ObjectMetadata>()
            .init_asset_loader::<AssetMetadataLoader>()
            .add_startup_system(Self::load_system);
    }
}

impl AssetMetadataPlugin {
    fn load_system(mut commands: Commands, asset_server: ResMut<AssetServer>) {
        let mut folder: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap_or_default().into();
        folder.push(AssetPlugin::default().asset_folder);

        let mut handles = Vec::new();
        for entry in WalkDir::new(&folder)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if let Some(extension) = entry.path().extension() {
                if extension == METADATA_EXTENSION {
                    let path = entry
                        .path()
                        .strip_prefix(&folder)
                        .unwrap_or_else(|e| panic!("entries should start with {folder:?}: {e}"));
                    handles.push(asset_server.load_untyped(path));
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
            match toml::from_slice::<AssetMetadata>(bytes)? {
                AssetMetadata::Object(metadata) => {
                    load_context.set_default_asset(LoadedAsset::new(metadata))
                }
                AssetMetadata::Cloth => unimplemented!(),
            }
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &[METADATA_EXTENSION]
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

#[derive(Deref, DerefMut, Resource)]
struct MetadataHandles(Vec<HandleUntyped>);

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum AssetMetadata {
    Object(ObjectMetadata),
    Cloth,
}

#[derive(Deserialize)]
pub(crate) struct GeneralMetadata {
    pub(crate) name: String,
    pub(crate) preview_translation: Vec3,
}

#[derive(Deserialize, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub(crate) struct ObjectMetadata {
    #[serde(flatten)]
    pub(crate) general: GeneralMetadata,
    pub(crate) category: ObjectCategory,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Display)]
pub(crate) enum ObjectCategory {
    Rocks,
    Foliage,
    #[serde(rename = "Outdoor furniture")]
    OutdoorFurniture,
}

impl ObjectCategory {
    pub(crate) const CITY_CATEGORIES: &[ObjectCategory] = &[
        ObjectCategory::Rocks,
        ObjectCategory::Foliage,
        ObjectCategory::OutdoorFurniture,
    ];

    pub(crate) fn glyph(self) -> &'static str {
        match self {
            ObjectCategory::Rocks => "🗻",
            ObjectCategory::Foliage => "🍀",
            ObjectCategory::OutdoorFurniture => "🏡",
        }
    }
}
