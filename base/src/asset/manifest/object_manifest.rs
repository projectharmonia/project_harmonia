use std::{
    any,
    fmt::{self, Formatter},
    path::Path,
};

use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::{serde::TypedReflectDeserializer, TypeRegistry, TypeRegistryArc},
    scene::ron,
};
use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use strum::{IntoStaticStr, VariantNames};

use super::{GeneralManifest, ManifestFormat, MapPaths, ReflectMapPaths};
use crate::asset;

pub struct ObjectLoader {
    registry: TypeRegistryArc,
}

impl FromWorld for ObjectLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

impl AssetLoader for ObjectLoader {
    type Asset = ObjectManifest;
    type Settings = ();
    type Error = anyhow::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut string = String::new();
        reader.read_to_string(&mut string).await?;

        let dir = load_context.path().parent();
        let seed = ObjectManifestDeserializer {
            registry: &self.registry.read(),
            dir,
        };

        let manifest = ron::Options::default().from_str_seed(&string, seed)?;

        Ok(manifest)
    }

    fn extensions(&self) -> &'static [&'static str] {
        ManifestFormat::Object.extensions()
    }
}

#[derive(TypePath, Asset)]
pub struct ObjectManifest {
    pub general: GeneralManifest,
    pub scene: AssetPath<'static>,
    pub category: ObjectCategory,
    pub preview_translation: Vec3,
    pub components: Vec<Box<dyn PartialReflect>>,
    pub place_components: Vec<Box<dyn PartialReflect>>,
    pub spawn_components: Vec<Box<dyn PartialReflect>>,
}

impl MapPaths for ObjectManifest {
    fn map_paths(&mut self, dir: &Path) {
        asset::change_parent_dir(&mut self.scene, dir);
    }
}

/// Fields of [`ObjectManifest`] for manual deserialization.
#[derive(Deserialize, VariantNames, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[serde(field_identifier, rename_all = "snake_case")]
enum ObjectManifestField {
    General,
    Scene,
    Category,
    PreviewTranslation,
    Components,
    PlaceComponents,
    SpawnComponents,
}

#[derive(Clone, Component, Copy, Deserialize, PartialEq)]
pub enum ObjectCategory {
    Rocks,
    Foliage,
    OutdoorFurniture,
    OutdoorActivities,
    Street,
    Electronics,
    Furniture,
    Windows,
    Doors,
}

impl ObjectCategory {
    pub const CITY_CATEGORIES: &'static [ObjectCategory] = &[
        ObjectCategory::Rocks,
        ObjectCategory::Foliage,
        ObjectCategory::OutdoorFurniture,
        ObjectCategory::OutdoorActivities,
        ObjectCategory::Street,
    ];

    pub const FAMILY_CATEGORIES: &'static [ObjectCategory] = &[
        ObjectCategory::Rocks,
        ObjectCategory::Foliage,
        ObjectCategory::OutdoorFurniture,
        ObjectCategory::Electronics,
        ObjectCategory::Furniture,
        ObjectCategory::Windows,
        ObjectCategory::Doors,
    ];

    pub fn glyph(self) -> &'static str {
        match self {
            ObjectCategory::Rocks => "🗻",
            ObjectCategory::Foliage => "🍀",
            ObjectCategory::OutdoorFurniture => "🏡",
            ObjectCategory::OutdoorActivities => "🔤",
            ObjectCategory::Street => "🚃",
            ObjectCategory::Electronics => "📺",
            ObjectCategory::Furniture => "💺",
            ObjectCategory::Windows => "🔲",
            ObjectCategory::Doors => "🚪",
        }
    }
}

pub(super) struct ObjectManifestDeserializer<'a> {
    pub(super) registry: &'a TypeRegistry,
    pub(super) dir: Option<&'a Path>,
}

impl<'de> DeserializeSeed<'de> for ObjectManifestDeserializer<'_> {
    type Value = ObjectManifest;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<Self::Value>(),
            ObjectManifestField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for ObjectManifestDeserializer<'_> {
    type Value = ObjectManifest;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
        let mut general = None;
        let mut scene = None;
        let mut category = None;
        let mut preview_translation = None;
        let mut components = None;
        let mut place_components = None;
        let mut spawn_components = None;
        while let Some(key) = map.next_key()? {
            match key {
                ObjectManifestField::General => {
                    if general.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::General.into(),
                        ));
                    }
                    general = Some(map.next_value()?);
                }
                ObjectManifestField::Scene => {
                    if scene.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::General.into(),
                        ));
                    }
                    scene = Some(map.next_value()?);
                }
                ObjectManifestField::Category => {
                    if category.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::Category.into(),
                        ));
                    }
                    category = Some(map.next_value()?);
                }
                ObjectManifestField::PreviewTranslation => {
                    if preview_translation.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::PreviewTranslation.into(),
                        ));
                    }
                    preview_translation = Some(map.next_value()?);
                }
                ObjectManifestField::Components => {
                    if components.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::Components.into(),
                        ));
                    }
                    components = Some(
                        map.next_value_seed(ComponentsDeserializer::new(self.registry, self.dir))?,
                    );
                }
                ObjectManifestField::PlaceComponents => {
                    if place_components.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::PlaceComponents.into(),
                        ));
                    }
                    place_components = Some(
                        map.next_value_seed(ComponentsDeserializer::new(self.registry, self.dir))?,
                    );
                }
                ObjectManifestField::SpawnComponents => {
                    if spawn_components.is_some() {
                        return Err(de::Error::duplicate_field(
                            ObjectManifestField::SpawnComponents.into(),
                        ));
                    }
                    spawn_components = Some(
                        map.next_value_seed(ComponentsDeserializer::new(self.registry, self.dir))?,
                    );
                }
            }
        }

        let general =
            general.ok_or_else(|| de::Error::missing_field(ObjectManifestField::General.into()))?;
        let scene =
            scene.ok_or_else(|| de::Error::missing_field(ObjectManifestField::Scene.into()))?;
        let category = category
            .ok_or_else(|| de::Error::missing_field(ObjectManifestField::Category.into()))?;
        let preview_translation = preview_translation.ok_or_else(|| {
            de::Error::missing_field(ObjectManifestField::PreviewTranslation.into())
        })?;
        let components = components.unwrap_or_default();
        let place_components = place_components.unwrap_or_default();
        let spawn_components = spawn_components.unwrap_or_default();

        let mut manifest = ObjectManifest {
            general,
            scene,
            category,
            preview_translation,
            components,
            place_components,
            spawn_components,
        };

        if let Some(dir) = self.dir {
            asset::change_parent_dir(&mut manifest.scene, dir);
        }

        Ok(manifest)
    }
}

struct ComponentsDeserializer<'a> {
    registry: &'a TypeRegistry,
    dir: Option<&'a Path>,
}

impl<'a> ComponentsDeserializer<'a> {
    fn new(registry: &'a TypeRegistry, dir: Option<&'a Path>) -> Self {
        Self { registry, dir }
    }
}

impl<'de> DeserializeSeed<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn PartialReflect>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn PartialReflect>>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut components = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        while let Some(component) =
            seq.next_element_seed(ShortReflectDeserializer::new(self.registry, self.dir))?
        {
            components.push(component);
        }

        Ok(components)
    }
}

/// Like [`UntypedReflectDeserializer`], but searches for registration by short name.
pub(super) struct ShortReflectDeserializer<'a> {
    registry: &'a TypeRegistry,
    dir: Option<&'a Path>,
}

impl<'a> ShortReflectDeserializer<'a> {
    fn new(registry: &'a TypeRegistry, dir: Option<&'a Path>) -> Self {
        Self { registry, dir }
    }
}

impl<'de> DeserializeSeed<'de> for ShortReflectDeserializer<'_> {
    type Value = Box<dyn PartialReflect>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ShortReflectDeserializer<'_> {
    type Value = Box<dyn PartialReflect>;

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
            .ok_or_else(|| de::Error::custom(format!("`{type_path}` is not registered")))?;
        let mut partial_reflect =
            map.next_value_seed(TypedReflectDeserializer::new(registration, self.registry))?;

        if let Some(dir) = self.dir {
            if let Some(reflect_map) = self
                .registry
                .get_type_data::<ReflectMapPaths>(registration.type_id())
            {
                let from_reflect = self
                    .registry
                    .get_type_data::<ReflectFromReflect>(registration.type_id())
                    .unwrap_or_else(|| panic!("`{type_path}` should reflect `FromReflect`"));

                let mut reflect = from_reflect.from_reflect(&*partial_reflect).unwrap();
                reflect_map.get_mut(&mut *reflect).unwrap().map_paths(dir);
                partial_reflect = reflect.into_partial_reflect();
            }
        }

        Ok(partial_reflect)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let registration = self
            .registry
            .get_with_short_type_path(v)
            .ok_or_else(|| de::Error::custom(format!("`{v}` is not registered")))?;
        let reflect_default = registration
            .data::<ReflectDefault>()
            .ok_or_else(|| de::Error::custom(format!("`{v}` doesn't have reflect(Default)")))?;

        Ok(reflect_default.default().into_partial_reflect())
    }
}
