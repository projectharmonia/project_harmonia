use std::{
    any,
    fmt::{self, Formatter},
};

use bevy::{
    prelude::*,
    reflect::{serde::TypedReflectDeserializer, TypeRegistry},
};
use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use strum::{Display, IntoStaticStr, VariantNames};

use super::{GeneralMetadata, Metadata};
use crate::core::object::{ObjectComponent, ReflectObjectComponent};

#[derive(TypePath, Asset)]
pub(crate) struct ObjectMetadata {
    pub(crate) general: GeneralMetadata,
    pub(crate) category: ObjectCategory,
    pub(crate) components: Vec<Box<dyn ObjectComponent>>,
}

impl Metadata for ObjectMetadata {
    type Deserializer<'a> = ObjectMetadataDeserializer<'a>;

    const SECTION: &'static str = "object";
    const DIR: &'static str = "objects";

    fn deserializer(registry: &TypeRegistry, general: GeneralMetadata) -> Self::Deserializer<'_> {
        ObjectMetadataDeserializer { registry, general }
    }
}

/// Fields of [`ObjectMetadata`] for manual deserialization.
#[derive(Deserialize, VariantNames, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[serde(field_identifier, rename_all = "snake_case")]
enum ObjectMetadataField {
    Category,
    Components,
}

#[derive(Clone, Component, Copy, Deserialize, Display, PartialEq)]
pub(crate) enum ObjectCategory {
    Rocks,
    Foliage,
    #[serde(rename = "Outdoor furniture")]
    OutdoorFurniture,
    Electronics,
    Windows,
    Doors,
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
        ObjectCategory::Electronics,
        ObjectCategory::Windows,
        ObjectCategory::Doors,
    ];

    pub(crate) fn glyph(self) -> &'static str {
        match self {
            ObjectCategory::Rocks => "🗻",
            ObjectCategory::Foliage => "🍀",
            ObjectCategory::OutdoorFurniture => "🏡",
            ObjectCategory::Electronics => "📺",
            ObjectCategory::Windows => "🔲",
            ObjectCategory::Doors => "🚪",
        }
    }
}

pub(super) struct ObjectMetadataDeserializer<'a> {
    registry: &'a TypeRegistry,
    general: GeneralMetadata,
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
        let mut category = None;
        let mut components = None;
        while let Some(key) = map.next_key()? {
            match key {
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

        let category = category
            .ok_or_else(|| de::Error::missing_field(ObjectMetadataField::Category.into()))?;
        let components = components
            .ok_or_else(|| de::Error::missing_field(ObjectMetadataField::Components.into()))?;

        Ok(ObjectMetadata {
            general: self.general,
            category,
            components,
        })
    }
}

struct ComponentsDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> ComponentsDeserializer<'a> {
    fn new(registry: &'a TypeRegistry) -> Self {
        Self { registry }
    }
}

impl<'de> DeserializeSeed<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn ObjectComponent>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn ObjectComponent>>;

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
pub(super) struct ShortReflectDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> ShortReflectDeserializer<'a> {
    fn new(registry: &'a TypeRegistry) -> Self {
        Self { registry }
    }
}

impl<'de> DeserializeSeed<'de> for ShortReflectDeserializer<'_> {
    type Value = Box<dyn ObjectComponent>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ShortReflectDeserializer<'_> {
    type Value = Box<dyn ObjectComponent>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let type_path = map
            .next_key::<String>()?
            .ok_or_else(|| de::Error::invalid_length(0, &"at least one entry"))?;
        let registration = self
            .registry
            .get_with_short_type_path(&type_path)
            .ok_or_else(|| de::Error::custom(format!("{type_path} is not registered")))?;
        let mut reflect =
            map.next_value_seed(TypedReflectDeserializer::new(registration, self.registry))?;

        let rfr = registration.data::<ReflectFromReflect>().ok_or_else(|| {
            de::Error::custom(format!("{type_path} doesn't implement FromReflect"))
        })?;
        reflect = rfr.from_reflect(&*reflect).ok_or_else(|| {
            de::Error::custom(format!("{type_path} can't be converted into actual type"))
        })?;

        let reflect_object = registration
            .data::<ReflectObjectComponent>()
            .ok_or_else(|| de::Error::custom(format!("{type_path} doesn't have reflect(Task)")))?;
        let object_component = reflect_object
            .get_boxed(reflect)
            .map_err(|_| de::Error::custom(format!("{type_path} is not an ObjectComponent")))?;

        Ok(object_component)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let registration = self
            .registry
            .get_with_short_type_path(v)
            .ok_or_else(|| de::Error::custom(format!("{v} is not registered")))?;
        let reflect_default = registration
            .data::<ReflectDefault>()
            .ok_or_else(|| de::Error::custom(format!("{v} doesn't have reflect(Default)")))?;

        let rfr = registration
            .data::<ReflectFromReflect>()
            .ok_or_else(|| de::Error::custom(format!("{v} doesn't implement FromReflect")))?;
        let reflect = rfr
            .from_reflect(&*reflect_default.default())
            .ok_or_else(|| de::Error::custom(format!("{v} can't be converted into actual type")))?;

        let reflect_object = registration
            .data::<ReflectObjectComponent>()
            .ok_or_else(|| de::Error::custom(format!("{v} doesn't have reflect(Task)")))?;
        let object_component = reflect_object
            .get_boxed(reflect)
            .map_err(|_| de::Error::custom(format!("{v} is not an ObjectComponent")))?;

        Ok(object_component)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use walkdir::WalkDir;

    use super::*;
    use crate::core::{
        asset::metadata::{MetadataDeserializer, METADATA_EXTENSION},
        object::{door::Door, wall_mount::WallMount},
    };

    #[test]
    fn deserialization() -> Result<()> {
        const ASSETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/base/objects");
        let mut type_registry = TypeRegistry::new();
        type_registry.register::<Vec2>();
        type_registry.register::<Vec<Vec2>>();
        type_registry.register::<WallMount>();
        type_registry.register::<Door>();

        for entry in WalkDir::new(ASSETS_DIR)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if let Some(extension) = entry.path().extension() {
                if extension == METADATA_EXTENSION {
                    let data = fs::read_to_string(entry.path())?;
                    let deserializer = toml::Deserializer::new(&data);
                    MetadataDeserializer::<ObjectMetadata>::new(&type_registry)
                        .deserialize(deserializer)?;
                }
            }
        }

        Ok(())
    }
}
