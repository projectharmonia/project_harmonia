pub mod object_metadata;

use std::{env, fs, marker::PhantomData, path::Path};

use anyhow::Result;
use bevy::{
    app::PluginGroupBuilder,
    asset::{io::Reader, AssetLoader, AssetPath, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    utils::BoxedFuture,
};
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use walkdir::WalkDir;

use object_metadata::ObjectMetadata;

pub(super) struct MetadataPlugins;

impl PluginGroup for MetadataPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(MetadataPlugin::<ObjectMetadata>::default())
    }
}

struct MetadataPlugin<T>(PhantomData<T>);

impl<T> Default for MetadataPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Asset + Metadata> Plugin for MetadataPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_asset::<T>()
            .init_asset_loader::<MetadataLoader<T>>()
            .init_resource::<MetadataHandles<T>>();
    }
}

pub struct MetadataLoader<T> {
    registry: TypeRegistryArc,
    marker: PhantomData<T>,
}

impl<T> FromWorld for MetadataLoader<T> {
    fn from_world(world: &mut World) -> Self {
        Self {
            registry: world.resource::<AppTypeRegistry>().0.clone(),
            marker: PhantomData,
        }
    }
}

const METADATA_EXTENSION: &str = "ron";

impl<T: Asset + Metadata> AssetLoader for MetadataLoader<T> {
    type Asset = T;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut data = String::new();
            reader.read_to_string(&mut data).await?;
            let metadata = ron::Options::default()
                .from_str_seed(&data, T::deserializer(&self.registry.read()))?;

            Ok(metadata)
        })
    }

    fn extensions(&self) -> &[&str] {
        &[METADATA_EXTENSION]
    }
}

/// Preloads and stores metadata handles.
#[derive(Resource)]
#[allow(dead_code)]
struct MetadataHandles<T: Asset>(Vec<Handle<T>>);

impl<T: Asset + Metadata> FromWorld for MetadataHandles<T> {
    fn from_world(world: &mut World) -> Self {
        let assets_dir =
            Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("assets");

        let mut handles = Vec::new();
        let asset_server = world.resource::<AssetServer>();
        for mut dir in fs::read_dir(&assets_dir)
            .expect("unable to read assets")
            .flat_map(|entry| entry.ok())
            .map(|entry| entry.path())
        {
            dir.push(T::DIR);

            for entry in WalkDir::new(&dir)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if let Some(extension) = entry.path().extension() {
                    if extension == METADATA_EXTENSION {
                        let path = entry
                            .path()
                            .strip_prefix(&assets_dir)
                            .unwrap_or_else(|e| panic!("entries should start with {dir:?}: {e}"));

                        debug!("loading metadata for {path:?}");
                        handles.push(asset_server.load(path.to_path_buf()));
                    }
                }
            }
        }

        Self(handles)
    }
}

trait Metadata {
    type Deserializer<'a>: for<'de> DeserializeSeed<'de, Value = Self>;

    /// Name of section in metadata file.
    const SECTION: &'static str;

    /// Directory from which files should be preloaded.
    const DIR: &'static str;

    /// Creates its own deserializer.
    fn deserializer(registry: &TypeRegistry) -> Self::Deserializer<'_>;
}

/// Converts metadata path into the corresponding scene path loadable by [`AssetServer`].
pub fn gltf_asset(metadata_path: &AssetPath, label: &'static str) -> AssetPath<'static> {
    let scene_path: AssetPath = metadata_path.path().with_extension("gltf").into();
    scene_path.with_label(label)
}

#[derive(Serialize, Deserialize)]
pub struct GeneralMetadata {
    pub name: String,
    pub author: String,
    pub license: String,
    pub preview_translation: Vec3,
}
