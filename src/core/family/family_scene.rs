use std::{any, fmt::Formatter, iter};

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
    reflect::{
        serde::{ReflectDeserializer, ReflectSerializer},
        TypeRegistryInternal,
    },
    scene::DynamicEntity,
};
use derive_more::Constructor;
use serde::{
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};
use strum::{EnumVariantNames, IntoStaticStr, VariantNames};

use crate::core::network::{
    entity_serde::{EntityDeserializer, EntitySerializer},
    network_event::client_event::ReflectEvent,
};

/// An event from client which indicates that a family should be spawned.
#[derive(Debug)]
pub(crate) struct FamilySpawn {
    pub(crate) city_entity: Entity,
    pub(crate) scene: FamilyScene,
}

impl MapEntities for FamilySpawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.city_entity = entity_map.get(self.city_entity)?;
        Ok(())
    }
}

impl<'a> ReflectEvent<'a> for FamilySpawn {
    type Serializer = FamilySpawnSerializer<'a>;
    type Deserializer = FamilySpawnDeserializer<'a>;

    fn serializer(event: &'a Self, registry: &'a TypeRegistryInternal) -> Self::Serializer {
        Self::Serializer { event, registry }
    }

    fn deserializer(registry: &'a TypeRegistryInternal) -> Self::Deserializer {
        Self::Deserializer { registry }
    }
}

#[derive(Deserialize, IntoStaticStr, EnumVariantNames)]
#[serde(field_identifier)]
enum FamilySpawnField {
    CityEntity,
    Scene,
}

#[derive(Constructor)]
pub(crate) struct FamilySpawnSerializer<'a> {
    event: &'a FamilySpawn,
    registry: &'a TypeRegistryInternal,
}

impl Serialize for FamilySpawnSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            any::type_name::<FamilySpawn>(),
            FamilySpawnField::VARIANTS.len(),
        )?;
        state.serialize_field(
            FamilySpawnField::CityEntity.into(),
            &EntitySerializer(self.event.city_entity),
        )?;
        state.serialize_field(
            FamilySpawnField::Scene.into(),
            &FamilySceneSerializer::new(&self.event.scene, self.registry),
        )?;
        state.end()
    }
}

#[derive(Debug, Default)]
pub(crate) struct FamilyScene {
    components: Vec<Box<dyn Reflect>>,
    members: Vec<Vec<Box<dyn Reflect>>>,
}

/// Converts to [`FamilyScene`] by assuming that the last element is a family entity.
///
/// # Panics
///
/// Panics if a scene is empty.
impl From<DynamicScene> for FamilyScene {
    fn from(scene: DynamicScene) -> Self {
        let mut entities: Vec<_> = scene
            .entities
            .into_iter()
            .map(|entity| entity.components)
            .collect();
        Self {
            components: entities.pop().expect("scene can't be empty"),
            members: entities,
        }
    }
}

impl From<FamilyScene> for DynamicScene {
    fn from(scene: FamilyScene) -> Self {
        DynamicScene {
            entities: scene
                .members
                .into_iter()
                .chain(iter::once(scene.components))
                .enumerate()
                .map(|(index, components)| DynamicEntity {
                    entity: index as u32,
                    components,
                })
                .collect(),
        }
    }
}

#[derive(Deserialize, IntoStaticStr, EnumVariantNames)]
#[serde(field_identifier)]
enum FamilySceneField {
    Components,
    Members,
}

#[derive(Constructor)]
pub(super) struct FamilySceneSerializer<'a> {
    scene: &'a FamilyScene,
    registry: &'a TypeRegistryInternal,
}

impl Serialize for FamilySceneSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            any::type_name::<FamilyScene>(),
            FamilySceneField::VARIANTS.len(),
        )?;
        state.serialize_field(
            FamilySceneField::Components.into(),
            &ComponentsSerializer::new(&self.scene.components, self.registry),
        )?;
        state.serialize_field(
            FamilySceneField::Members.into(),
            &MembersSerializer::new(&self.scene.members, self.registry),
        )?;
        state.end()
    }
}

#[derive(Constructor)]
struct MembersSerializer<'a> {
    members: &'a Vec<Vec<Box<dyn Reflect>>>,
    registry: &'a TypeRegistryInternal,
}

impl Serialize for MembersSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.members.len()))?;
        for member in self.members {
            seq.serialize_element(&ComponentsSerializer::new(member, self.registry))?;
        }
        seq.end()
    }
}

#[derive(Constructor)]
struct ComponentsSerializer<'a> {
    components: &'a Vec<Box<dyn Reflect>>,
    registry: &'a TypeRegistryInternal,
}

impl Serialize for ComponentsSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.components.len()))?;
        for component in self.components {
            seq.serialize_element(&ReflectSerializer::new(&**component, self.registry))?;
        }
        seq.end()
    }
}

#[derive(Constructor)]
pub(crate) struct FamilySpawnDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for FamilySpawnDeserializer<'_> {
    type Value = FamilySpawn;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<FamilySpawn>(),
            FamilySpawnField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for FamilySpawnDeserializer<'_> {
    type Value = FamilySpawn;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let city_entity = seq.next_element_seed(EntityDeserializer)?.ok_or_else(|| {
            de::Error::invalid_length(FamilySpawnField::CityEntity as usize, &self)
        })?;
        let scene = seq
            .next_element_seed(FamilySceneDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_length(FamilySpawnField::Scene as usize, &self))?;
        Ok(FamilySpawn { city_entity, scene })
    }
}

#[derive(Constructor)]
pub(super) struct FamilySceneDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for FamilySceneDeserializer<'_> {
    type Value = FamilyScene;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<FamilyScene>(),
            FamilySceneField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for FamilySceneDeserializer<'_> {
    type Value = FamilyScene;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let components = seq
            .next_element_seed(ComponentsDeserializer::new(self.registry))?
            .ok_or_else(|| {
                de::Error::invalid_length(FamilySceneField::Components as usize, &self)
            })?;
        let members = seq
            .next_element_seed(MembersDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_length(FamilySceneField::Members as usize, &self))?;
        Ok(FamilyScene {
            components,
            members,
        })
    }
}

#[derive(Constructor)]
struct MembersDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for MembersDeserializer<'_> {
    type Value = Vec<Vec<Box<dyn Reflect>>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for MembersDeserializer<'_> {
    type Value = Vec<Vec<Box<dyn Reflect>>>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut members = Vec::new();
        while let Some(member) =
            seq.next_element_seed(ComponentsDeserializer::new(self.registry))?
        {
            members.push(member);
        }

        Ok(members)
    }
}

#[derive(Constructor)]
struct ComponentsDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn Reflect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for ComponentsDeserializer<'_> {
    type Value = Vec<Box<dyn Reflect>>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut components = Vec::new();
        while let Some(component) =
            seq.next_element_seed(ReflectDeserializer::new(self.registry))?
        {
            components.push(component);
        }

        Ok(components)
    }
}

#[cfg(test)]
mod tests {
    use serde_test::Token;

    use super::*;

    #[test]
    fn family_spawn_ser() {
        const ENTITY_ID: u64 = 0;
        let registry = TypeRegistryInternal::default();
        let event = FamilySpawn {
            city_entity: Entity::from_bits(ENTITY_ID),
            scene: FamilyScene {
                components: vec![Visibility::visible().clone_value()],
                members: vec![Vec::new()],
            },
        };
        let serializer = FamilySpawnSerializer::new(&event, &registry);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Struct {
                    name: any::type_name::<FamilySpawn>(),
                    len: FamilySpawnField::VARIANTS.len(),
                },
                Token::Str(FamilySpawnField::CityEntity.into()),
                Token::U64(ENTITY_ID),
                Token::Str(FamilySpawnField::Scene.into()),
                Token::Struct {
                    name: any::type_name::<FamilyScene>(),
                    len: FamilySceneField::VARIANTS.len(),
                },
                Token::Str(FamilySceneField::Components.into()),
                Token::Seq { len: Some(1) },
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
                Token::SeqEnd,
                Token::Str(FamilySceneField::Members.into()),
                Token::Seq { len: Some(1) },
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::SeqEnd,
                Token::StructEnd,
                Token::StructEnd,
            ],
        );
    }
}
