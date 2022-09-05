use std::any;

use bevy::{
    prelude::*,
    reflect::{
        serde::{ReflectDeserializer, ReflectSerializer},
        TypeRegistry, TypeRegistryInternal,
    },
    utils::HashMap,
};
use derive_more::Constructor;
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};
use strum::{EnumVariantNames, IntoStaticStr, VariantNames};

/// Type of component or resource change.
pub(super) enum ComponentDiff {
    /// Indicates that a component was added or changed, contains serialized [`Reflect`].
    Changed(Box<dyn Reflect>),
    /// Indicates that a component was removed, contains component name.
    Removed(String),
}

impl ComponentDiff {
    /// Returns changed component type name.
    pub(super) fn type_name(&self) -> &str {
        match self {
            ComponentDiff::Changed(reflect) => reflect.type_name(),
            ComponentDiff::Removed(type_name) => type_name,
        }
    }
}

#[derive(Deserialize, IntoStaticStr, EnumVariantNames)]
#[serde(field_identifier)]
enum ComponentDiffField {
    Changed,
    Removed,
}

/// Changed world data and current tick from server.
///
/// Sent from server to clients.
pub(super) struct WorldDiff {
    pub(super) tick: u32,
    pub(super) entities: HashMap<Entity, Vec<ComponentDiff>>,
}

impl WorldDiff {
    /// Creates a new [`WorldDiff`] with a tick and empty entities.
    pub(super) fn new(tick: u32) -> Self {
        Self {
            tick,
            entities: Default::default(),
        }
    }
}

#[derive(Deserialize, IntoStaticStr, EnumVariantNames)]
#[serde(field_identifier)]
enum WorldDiffField {
    Tick,
    Entities,
}

#[derive(Constructor)]
pub(super) struct WorldDiffSerializer<'a> {
    pub(super) registry: &'a TypeRegistry,
    pub(super) world_diff: &'a WorldDiff,
}

impl<'a> Serialize for WorldDiffSerializer<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            any::type_name::<WorldDiff>(),
            WorldDiffField::VARIANTS.len(),
        )?;
        state.serialize_field(WorldDiffField::Tick.into(), &self.world_diff.tick)?;
        state.serialize_field(
            WorldDiffField::Entities.into(),
            &EntitiesSerializer::new(&self.registry.read(), &self.world_diff.entities),
        )?;
        state.end()
    }
}

#[derive(Constructor)]
struct EntitiesSerializer<'a> {
    registry: &'a TypeRegistryInternal,
    entities: &'a HashMap<Entity, Vec<ComponentDiff>>,
}

impl<'a> Serialize for EntitiesSerializer<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.entities.len()))?;
        for (entity, components) in self.entities {
            map.serialize_entry(
                entity,
                &ComponentsSerializer::new(self.registry, components),
            )?;
        }
        map.end()
    }
}

#[derive(Constructor)]
struct ComponentsSerializer<'a> {
    registry: &'a TypeRegistryInternal,
    components: &'a Vec<ComponentDiff>,
}

impl<'a> Serialize for ComponentsSerializer<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.components.len()))?;
        for component_diff in self.components {
            seq.serialize_element(&ComponentDiffSerializer::new(self.registry, component_diff))?;
        }
        seq.end()
    }
}

#[derive(Constructor)]
struct ComponentDiffSerializer<'a> {
    registry: &'a TypeRegistryInternal,
    component_diff: &'a ComponentDiff,
}

impl<'a> Serialize for ComponentDiffSerializer<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.component_diff {
            ComponentDiff::Changed(reflect) => serializer.serialize_newtype_variant(
                any::type_name::<ComponentDiff>(),
                ComponentDiffField::Changed as u32,
                ComponentDiffField::Changed.into(),
                &ReflectSerializer::new(&**reflect, self.registry),
            ),
            ComponentDiff::Removed(type_name) => serializer.serialize_newtype_variant(
                any::type_name::<ComponentDiff>(),
                ComponentDiffField::Removed as u32,
                ComponentDiffField::Removed.into(),
                type_name,
            ),
        }
    }
}

#[derive(Constructor)]
pub(super) struct WorldDiffDeserializer<'a> {
    pub(super) registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for WorldDiffDeserializer<'a> {
    type Value = WorldDiff;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<WorldDiff>(),
            WorldDiffField::VARIANTS,
            self,
        )
    }
}

impl<'a, 'de> Visitor<'de> for WorldDiffDeserializer<'a> {
    type Value = WorldDiff;

    #[cfg_attr(coverage, no_coverage)]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let tick = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(WorldDiffField::Tick as usize, &self))?;
        let entities = seq
            .next_element_seed(EntitiesDeserializer::new(&self.registry.read()))?
            .ok_or_else(|| de::Error::invalid_length(WorldDiffField::Entities as usize, &self))?;
        Ok(WorldDiff { tick, entities })
    }
}

#[derive(Constructor)]
struct EntitiesDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'a, 'de> DeserializeSeed<'de> for EntitiesDeserializer<'a> {
    type Value = HashMap<Entity, Vec<ComponentDiff>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'a, 'de> Visitor<'de> for EntitiesDeserializer<'a> {
    type Value = HashMap<Entity, Vec<ComponentDiff>>;

    #[cfg_attr(coverage, no_coverage)]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut entities = HashMap::with_capacity(map.size_hint().unwrap_or_default());
        while let Some(key) = map.next_key()? {
            let value = map.next_value_seed(ComponentsDeserializer::new(self.registry))?;
            entities.insert(key, value);
        }

        Ok(entities)
    }
}

#[derive(Constructor)]
struct ComponentsDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentsDeserializer<'a> {
    type Value = Vec<ComponentDiff>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'a, 'de> Visitor<'de> for ComponentsDeserializer<'a> {
    type Value = Vec<ComponentDiff>;

    #[cfg_attr(coverage, no_coverage)]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut components = Vec::new();
        while let Some(component_diff) =
            seq.next_element_seed(ComponentDiffDeserializer::new(self.registry))?
        {
            components.push(component_diff);
        }

        Ok(components)
    }
}

#[derive(Constructor)]
struct ComponentDiffDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentDiffDeserializer<'a> {
    type Value = ComponentDiff;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_enum(
            any::type_name::<ComponentDiff>(),
            ComponentDiffField::VARIANTS,
            self,
        )
    }
}

impl<'a, 'de> Visitor<'de> for ComponentDiffDeserializer<'a> {
    type Value = ComponentDiff;

    #[cfg_attr(coverage, no_coverage)]
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (field, variant) = data.variant::<ComponentDiffField>()?;
        let component_diff = match field {
            ComponentDiffField::Changed => ComponentDiff::Changed(
                variant.newtype_variant_seed(ReflectDeserializer::new(self.registry))?,
            ),
            ComponentDiffField::Removed => ComponentDiff::Removed(variant.newtype_variant()?),
        };

        Ok(component_diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::Token;

    const COMPONENT_NAME: &str = "My component";
    const ENTITY_ID: u32 = 0;
    const TICK: u32 = 0;

    #[test]
    fn component_diff_removed_ser() {
        let registry = TypeRegistryInternal::new();
        let component_diff = ComponentDiff::Removed(COMPONENT_NAME.to_string());
        let serializer = ComponentDiffSerializer::new(&registry, &component_diff);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::NewtypeVariant {
                    name: any::type_name::<ComponentDiff>(),
                    variant: ComponentDiffField::Removed.into(),
                },
                Token::Str(COMPONENT_NAME),
            ],
        );
    }

    #[test]
    fn component_diff_changed_ser() {
        let mut registry = TypeRegistryInternal::new();
        registry.register::<Visibility>();
        let component_diff = ComponentDiff::Changed(Visibility::visible().clone_value());
        let serializer = ComponentDiffSerializer::new(&registry, &component_diff);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::NewtypeVariant {
                    name: any::type_name::<ComponentDiff>(),
                    variant: ComponentDiffField::Changed.into(),
                },
                Token::Map { len: Some(2) },
                Token::Str("type"),
                Token::Str(any::type_name::<Visibility>()),
                Token::Str("struct"),
                Token::Map { len: Some(1) },
                Token::Str("is_visible"),
                Token::Map { len: Some(2) },
                Token::Str("type"),
                Token::Str(any::type_name::<bool>()),
                Token::Str("value"),
                Token::Bool(true),
                Token::MapEnd,
                Token::MapEnd,
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn components_ser_empty() {
        let registry = TypeRegistryInternal::new();
        let components = Vec::new();
        let serializer = ComponentsSerializer::new(&registry, &components);

        serde_test::assert_ser_tokens(&serializer, &[Token::Seq { len: Some(0) }, Token::SeqEnd]);
    }

    #[test]
    fn components_ser() {
        let registry = TypeRegistryInternal::new();
        let components = Vec::from([ComponentDiff::Removed(COMPONENT_NAME.to_string())]);
        let serializer = ComponentsSerializer::new(&registry, &components);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Seq { len: Some(1) },
                Token::NewtypeVariant {
                    name: any::type_name::<ComponentDiff>(),
                    variant: ComponentDiffField::Removed.into(),
                },
                Token::Str(COMPONENT_NAME),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn entities_ser_empty() {
        let registry = TypeRegistryInternal::new();
        let entities = HashMap::new();
        let serializer = EntitiesSerializer::new(&registry, &entities);

        serde_test::assert_ser_tokens(&serializer, &[Token::Map { len: Some(0) }, Token::MapEnd]);
    }

    #[test]
    fn entities_ser() {
        let registry = TypeRegistryInternal::new();
        let entities = HashMap::from([(
            Entity::from_raw(ENTITY_ID),
            Vec::from([ComponentDiff::Removed(COMPONENT_NAME.to_string())]),
        )]);
        let serializer = EntitiesSerializer::new(&registry, &entities);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Map { len: Some(1) },
                Token::U32(ENTITY_ID),
                Token::Seq { len: Some(1) },
                Token::NewtypeVariant {
                    name: any::type_name::<ComponentDiff>(),
                    variant: ComponentDiffField::Removed.into(),
                },
                Token::Str(COMPONENT_NAME),
                Token::SeqEnd,
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn world_diff_ser_empty() {
        let registry = TypeRegistry::default();
        let world_diff = WorldDiff::new(TICK);
        let serializer = WorldDiffSerializer::new(&registry, &world_diff);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Struct {
                    name: any::type_name::<WorldDiff>(),
                    len: 2,
                },
                Token::Str(WorldDiffField::Tick.into()),
                Token::U32(TICK),
                Token::Str(WorldDiffField::Entities.into()),
                Token::Map { len: Some(0) },
                Token::MapEnd,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn world_diff_ser() {
        let registry = TypeRegistry::default();
        let world_diff = WorldDiff {
            tick: TICK,
            entities: HashMap::from([(
                Entity::from_raw(ENTITY_ID),
                Vec::from([ComponentDiff::Removed(COMPONENT_NAME.to_string())]),
            )]),
        };
        let serializer = WorldDiffSerializer::new(&registry, &world_diff);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Struct {
                    name: any::type_name::<WorldDiff>(),
                    len: 2,
                },
                Token::Str(WorldDiffField::Tick.into()),
                Token::U32(TICK),
                Token::Str(WorldDiffField::Entities.into()),
                Token::Map { len: Some(1) },
                Token::U32(ENTITY_ID),
                Token::Seq { len: Some(1) },
                Token::NewtypeVariant {
                    name: any::type_name::<ComponentDiff>(),
                    variant: ComponentDiffField::Removed.into(),
                },
                Token::Str(COMPONENT_NAME),
                Token::SeqEnd,
                Token::MapEnd,
                Token::StructEnd,
            ],
        );
    }
}
