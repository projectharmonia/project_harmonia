pub(crate) mod object_metadata;

use std::{
    any, env,
    fmt::{self, Formatter},
    fs,
    marker::PhantomData,
    path::Path,
    str,
};

use anyhow::Result;
use bevy::{
    app::PluginGroupBuilder,
    asset::{io::Reader, AssetLoader, AssetPath, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    utils::BoxedFuture,
};
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use walkdir::WalkDir;

use self::object_metadata::ObjectMetadata;

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

const METADATA_EXTENSION: &str = "toml";

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

            let deserializer = toml::Deserializer::new(&data);
            let metadata =
                MetadataDeserializer::new(&self.registry.read()).deserialize(deserializer)?;
            Ok(metadata)
        })
    }

    fn extensions(&self) -> &[&str] {
        &[METADATA_EXTENSION]
    }
}

/// Preloads and stores metadata handles.
#[derive(Resource)]
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
    fn deserializer(registry: &TypeRegistry, general: GeneralMetadata) -> Self::Deserializer<'_>;
}

/// Converts metadata ID into the corresponding scene path loadable by [`AssetServer`].
pub(crate) fn scene_path(
    asset_server: &AssetServer,
    metadata_id: impl Into<AssetId<ObjectMetadata>>,
) -> AssetPath<'static> {
    let metadata_path = asset_server
        .get_path(metadata_id.into())
        .expect("metadata should always come from file");
    let scene_path: AssetPath = metadata_path.path().with_extension("gltf").into();
    scene_path.with_label("Scene0")
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GeneralMetadata {
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) license: String,
    pub(crate) preview_translation: Vec3,
}

/// Deserializes metadata in a form of dictionary with two keys: `general` and `T::NAME`.
///
/// Deserializes [`GeneralMetadata`] and passes it to the deserializer for `T`.
/// Manual deserialization is required because metadata could contain reflected components.
struct MetadataDeserializer<'a, T> {
    registry: &'a TypeRegistry,
    marker: PhantomData<T>,
}

impl<'a, T> MetadataDeserializer<'a, T> {
    fn new(registry: &'a TypeRegistry) -> Self {
        Self {
            registry,
            marker: PhantomData,
        }
    }
}

impl<'de, T: Metadata> DeserializeSeed<'de> for MetadataDeserializer<'_, T> {
    type Value = T;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de, T: Metadata> Visitor<'de> for MetadataDeserializer<'_, T> {
    type Value = T;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
        let Some(key1): Option<String> = map.next_key()? else {
            return Err(de::Error::missing_field("general"));
        };

        if key1 != "general" {
            return Err(de::Error::custom("'general' field should come first"));
        }

        let general = map.next_value()?;

        let Some(key2): Option<String> = map.next_key()? else {
            return Err(de::Error::missing_field(T::SECTION));
        };

        if key2 != T::SECTION {
            return Err(de::Error::unknown_field(&key2, &[T::SECTION]));
        }

        let kind = map.next_value_seed(T::deserializer(self.registry, general))?;

        Ok(kind)
    }
}
