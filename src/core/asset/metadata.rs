use std::{
    any, env,
    fmt::{self, Formatter},
    path::PathBuf,
    str,
};

use anyhow::{Context, Result};
use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext, LoadedAsset},
    prelude::*,
    reflect::{
        serde::TypedReflectDeserializer, TypePath, TypeRegistry, TypeRegistryInternal, TypeUuid,
    },
    utils::BoxedFuture,
};
use derive_more::Constructor;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};
use strum::{Display, EnumDiscriminants, EnumVariantNames, IntoStaticStr, VariantNames};
use walkdir::WalkDir;

const METADATA_EXTENSION: &str = "toml";

pub(super) struct MetadataPlugin;

impl Plugin for MetadataPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<ObjectMetadata>()
            .init_asset_loader::<MetadataLoader>()
            .init_resource::<MetadataHandles>();
    }
}

pub struct MetadataLoader(TypeRegistry);

impl FromWorld for MetadataLoader {
    fn from_world(world: &mut World) -> Self {
        Self(world.resource::<AppTypeRegistry>().0.clone())
    }
}

impl AssetLoader for MetadataLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let data = str::from_utf8(bytes)
                .with_context(|| format!("{:?} contains invalid UTF-8", load_context.path()))?;
            let deserializer = toml::Deserializer::new(data);
            match MetadataDeserializer::new(&self.0.read()).deserialize(deserializer)? {
                Metadata::Object(metadata) => {
                    load_context.set_default_asset(LoadedAsset::new(metadata))
                }
                Metadata::Cloth => unimplemented!(),
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
pub(crate) fn scene_path<'a, P: Into<AssetPath<'a>>>(metadata_path: P) -> AssetPath<'static> {
    let scene_path = metadata_path.into().path().with_extension("gltf");
    AssetPath::new(scene_path, Some("Scene0".to_string()))
}

/// Preloads and stores metadata handles.
#[derive(Resource)]
struct MetadataHandles(Vec<HandleUntyped>);

impl FromWorld for MetadataHandles {
    fn from_world(world: &mut World) -> Self {
        let mut dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap_or_default().into();
        dir.push("assets"); // TODO: Read setting from `AssetIo` trait.

        let mut handles = Vec::new();
        let asset_server = world.resource::<AssetServer>();
        for entry in WalkDir::new(&dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if let Some(extension) = entry.path().extension() {
                if extension == METADATA_EXTENSION {
                    let path = entry
                        .path()
                        .strip_prefix(&dir)
                        .unwrap_or_else(|e| panic!("entries should start with {dir:?}: {e}"));
                    handles.push(asset_server.load_untyped(path));
                }
            }
        }

        Self(handles)
    }
}

#[derive(EnumDiscriminants)]
#[strum_discriminants(
    name(MetadataField),
    derive(Deserialize, EnumVariantNames),
    strum(serialize_all = "snake_case"),
    serde(field_identifier, rename_all = "snake_case")
)]
enum Metadata {
    Object(ObjectMetadata),
    Cloth,
}

pub(crate) struct GeneralMetadata {
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) license: String,
    pub(crate) preview_translation: Vec3,
}

#[derive(TypeUuid, TypePath)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub(crate) struct ObjectMetadata {
    pub(crate) general: GeneralMetadata,
    pub(crate) category: ObjectCategory,
    pub(crate) components: Vec<Box<dyn Reflect>>,
}

/// Fields of [`ObjectMetadata`] for manual deserialization.
#[derive(Deserialize, EnumVariantNames, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[serde(field_identifier, rename_all = "snake_case")]
enum ObjectMetadataField {
    Name,
    Author,
    License,
    PreviewTranslation,
    Category,
    Components,
}

#[derive(Clone, Component, Copy, Deserialize, Display, PartialEq)]
pub(crate) enum ObjectCategory {
    Rocks,
    Foliage,
    #[serde(rename = "Outdoor furniture")]
    OutdoorFurniture,
    Decorations,
}

impl ObjectCategory {
    pub(crate) const CITY_CATEGORIES: &'static [ObjectCategory] = &[
        ObjectCategory::Rocks,
        ObjectCategory::Foliage,
        ObjectCategory::OutdoorFurniture,
    ];

    pub(crate) const FAMILY_CATEGORIES: &'static [ObjectCategory] = &[
        ObjectCategory::Rocks,
        ObjectCategory::Foliage,
        ObjectCategory::OutdoorFurniture,
        ObjectCategory::Decorations,
    ];

    pub(crate) fn glyph(self) -> &'static str {
        match self {
            ObjectCategory::Rocks => "🗻",
            ObjectCategory::Foliage => "🍀",
            ObjectCategory::OutdoorFurniture => "🏡",
            ObjectCategory::Decorations => "🌸",
        }
    }
}

#[derive(Constructor)]
struct MetadataDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for MetadataDeserializer<'_> {
    type Value = Metadata;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_enum(any::type_name::<Metadata>(), MetadataField::VARIANTS, self)
    }
}

impl<'de> Visitor<'de> for MetadataDeserializer<'_> {
    type Value = Metadata;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (field, variant) = data.variant::<MetadataField>()?;
        let asset_metadata = match field {
            MetadataField::Object => Metadata::Object(
                variant.newtype_variant_seed(ObjectMetadataDeserializer::new(self.registry))?,
            ),
            MetadataField::Cloth => Metadata::Cloth,
        };

        Ok(asset_metadata)
    }
}

#[derive(Constructor)]
struct ObjectMetadataDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for ObjectMetadataDeserializer<'_> {
    type Value = ObjectMetadata;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<Self::Value>(),
            ObjectMetadataField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for ObjectMetadataDeserializer<'_> {
    type Value = ObjectMetadata;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
        let mut name = None;
        let mut author = None;
        let mut license = None;
        let mut preview_translation = None;
        let mut category = None;
        let mut components = None;
        while let Some(key) = map.next_key()? {
            match key {
                ObjectMetadataField::Name => {
                    if name.is_some() {
                        return Err(de::Error::duplicate_field(ObjectMetadataField::Name.into()));
                    }
                    name = Some(map.next_value()?);
                }
                ObjectMetadataField::Author => {
                    if author.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectMetadataField::Author.into(),
                        ));
                    }
                    author = Some(map.next_value()?);
                }
                ObjectMetadataField::License => {
                    if license.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectMetadataField::License.into(),
                        ));
                    }
                    license = Some(map.next_value()?);
                }
                ObjectMetadataField::PreviewTranslation => {
                    if preview_translation.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectMetadataField::PreviewTranslation.into(),
                        ));
                    }
                    preview_translation = Some(map.next_value()?);
                }
                ObjectMetadataField::Category => {
                    if category.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectMetadataField::Category.into(),
                        ));
                    }
                    category = Some(map.next_value()?);
                }
                ObjectMetadataField::Components => {
                    if components.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectMetadataField::Components.into(),
                        ));
                    }
                    components =
                        Some(map.next_value_seed(ComponentsDeserializer::new(self.registry))?);
                }
            }
        }
        let name =
            name.ok_or_else(|| de::Error::missing_field(ObjectMetadataField::Name.into()))?;
        let author =
            author.ok_or_else(|| de::Error::missing_field(ObjectMetadataField::Author.into()))?;
        let license =
            license.ok_or_else(|| de::Error::missing_field(ObjectMetadataField::License.into()))?;
        let preview_translation = preview_translation.ok_or_else(|| {
            de::Error::missing_field(ObjectMetadataField::PreviewTranslation.into())
        })?;
        let category = category
            .ok_or_else(|| de::Error::missing_field(ObjectMetadataField::Category.into()))?;
        let components = components
            .ok_or_else(|| de::Error::missing_field(ObjectMetadataField::Components.into()))?;

        Ok(ObjectMetadata {
            general: GeneralMetadata {
                name,
                author,
                license,
                preview_translation,
            },
            category,
            components,
        })
    }
}

#[derive(Constructor)]
struct ComponentsDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn Reflect>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn Reflect>>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut components = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        while let Some(component) =
            seq.next_element_seed(ShortReflectDeserializer::new(self.registry))?
        {
            components.push(component);
        }

        Ok(components)
    }
}

/// Like [`UntypedReflectDeserializer`], but searches for registration by short name.
#[derive(Constructor)]
pub struct ShortReflectDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for ShortReflectDeserializer<'_> {
    type Value = Box<dyn Reflect>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ShortReflectDeserializer<'_> {
    type Value = Box<dyn Reflect>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let type_name = map
            .next_key::<String>()?
            .ok_or_else(|| de::Error::invalid_length(0, &"at least one entry"))?;

        let registration = self
            .registry
            .get_with_short_name(&type_name)
            .ok_or_else(|| de::Error::custom(format!("{type_name} is not registered")))?;
        let value =
            map.next_value_seed(TypedReflectDeserializer::new(registration, self.registry))?;
        Ok(value)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let registration = self
            .registry
            .get_with_short_name(v)
            .ok_or_else(|| de::Error::custom(format!("{v} is not registered")))?;
        let reflect_default = registration
            .data::<ReflectDefault>()
            .ok_or_else(|| de::Error::custom(format!("{v} doesn't have reflect(Default)")))?;
        Ok(reflect_default.default())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::{object::mirror::Mirror, wall::WallObject};

    #[test]
    fn deserialization() -> Result<()> {
        const ASSETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
        let mut type_registry = TypeRegistryInternal::new();
        type_registry.register::<Mirror>();
        type_registry.register::<WallObject>();
        for entry in WalkDir::new(ASSETS_DIR)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if let Some(extension) = entry.path().extension() {
                if extension == METADATA_EXTENSION {
                    let data = fs::read_to_string(entry.path())?;
                    let deserializer = toml::Deserializer::new(&data);
                    MetadataDeserializer::new(&type_registry).deserialize(deserializer)?;
                }
            }
        }

        Ok(())
    }
}