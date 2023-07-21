use std::{
    any,
    fmt::{self, Formatter},
};

use bevy::{
    ecs::entity::EntityMap,
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        TypeRegistryInternal,
    },
};
use bevy_replicon::prelude::*;
use derive_more::Constructor;
use serde::{
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct},
    Deserializer, Serialize, Serializer,
};
use strum::{EnumVariantNames, IntoStaticStr, VariantNames};

use super::Budget;
use crate::core::actor::race::{RaceBundle, ReflectRaceBundle};

#[derive(Debug, Event)]
pub(crate) struct FamilySpawn {
    pub(crate) city_entity: Entity,
    pub(crate) scene: FamilyScene,
    pub(crate) select: bool,
}

impl MapEventEntities for FamilySpawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapError> {
        self.city_entity = entity_map
            .get(self.city_entity)
            .ok_or(MapError(self.city_entity))?;

        Ok(())
    }
}

#[derive(IntoStaticStr, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
enum FamilySpawnField {
    CityEntity,
    Scene,
    Select,
}

#[derive(Component, Debug, Default)]
pub(crate) struct FamilyScene {
    pub(crate) name: Name,
    pub(crate) budget: Budget,
    pub(crate) actors: Vec<Box<dyn RaceBundle>>,
}

impl FamilyScene {
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            budget: Default::default(),
            actors: Default::default(),
        }
    }
}

#[derive(IntoStaticStr, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
enum FamilySceneField {
    Name,
    Budget,
    Actors,
}

#[derive(Constructor)]
pub(super) struct FamilySpawnSerializer<'a> {
    event: &'a FamilySpawn,
    registry: &'a TypeRegistryInternal,
}

impl BuildEventSerializer<FamilySpawn> for FamilySpawnSerializer<'_> {
    type EventSerializer<'a> = FamilySpawnSerializer<'a>;

    fn new<'a>(
        event: &'a FamilySpawn,
        registry: &'a TypeRegistryInternal,
    ) -> Self::EventSerializer<'a> {
        Self::EventSerializer { registry, event }
    }
}

impl Serialize for FamilySpawnSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            any::type_name::<FamilySpawn>(),
            FamilySpawnField::VARIANTS.len(),
        )?;
        state.serialize_field(FamilySpawnField::CityEntity.into(), &self.event.city_entity)?;
        state.serialize_field(
            FamilySpawnField::Scene.into(),
            &FamilySceneSerializer::new(&self.event.scene, self.registry),
        )?;
        state.serialize_field(FamilySpawnField::Select.into(), &self.event.select)?;
        state.end()
    }
}

#[derive(Constructor)]
pub(crate) struct FamilySceneSerializer<'a> {
    family_scene: &'a FamilyScene,
    registry: &'a TypeRegistryInternal,
}

impl Serialize for FamilySceneSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            any::type_name::<FamilyScene>(),
            FamilySceneField::VARIANTS.len(),
        )?;
        state.serialize_field(FamilySceneField::Name.into(), &self.family_scene.name)?;
        state.serialize_field(FamilySceneField::Budget.into(), &self.family_scene.budget)?;
        state.serialize_field(
            FamilySceneField::Actors.into(),
            &ActorsSerializer::new(&self.family_scene.actors, self.registry),
        )?;
        state.end()
    }
}

#[derive(Constructor)]
struct ActorsSerializer<'a> {
    actors: &'a [Box<dyn RaceBundle>],
    registry: &'a TypeRegistryInternal,
}

impl Serialize for ActorsSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.actors.len()))?;
        for race_bundle in self.actors {
            seq.serialize_element(&ReflectSerializer::new(
                race_bundle.as_reflect(),
                self.registry,
            ))?;
        }
        seq.end()
    }
}

pub(super) struct FamilySpawnDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl BuildEventDeserializer for FamilySpawnDeserializer<'_> {
    type EventDeserializer<'a> = FamilySpawnDeserializer<'a>;

    fn new(registry: &TypeRegistryInternal) -> Self::EventDeserializer<'_> {
        Self::EventDeserializer { registry }
    }
}

impl<'de> DeserializeSeed<'de> for FamilySpawnDeserializer<'_> {
    type Value = FamilySpawn;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<Self::Value>(),
            FamilySpawnField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for FamilySpawnDeserializer<'_> {
    type Value = FamilySpawn;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let city_entity = seq.next_element()?.ok_or_else(|| {
            de::Error::invalid_length(FamilySpawnField::CityEntity as usize, &self)
        })?;
        let scene = seq
            .next_element_seed(FamilySceneDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_length(FamilySpawnField::Scene as usize, &self))?;
        let select = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(FamilySpawnField::Select as usize, &self))?;

        Ok(FamilySpawn {
            city_entity,
            scene,
            select,
        })
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
            any::type_name::<Self::Value>(),
            FamilySceneField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for FamilySceneDeserializer<'_> {
    type Value = FamilyScene;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let name = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(FamilySceneField::Name as usize, &self))?;
        let budget = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(FamilySceneField::Budget as usize, &self))?;
        let actors = seq
            .next_element_seed(ActorsDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_length(FamilySceneField::Actors as usize, &self))?;

        Ok(FamilyScene {
            name,
            budget,
            actors,
        })
    }
}

#[derive(Constructor)]
struct ActorsDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl<'de> DeserializeSeed<'de> for ActorsDeserializer<'_> {
    type Value = Vec<Box<dyn RaceBundle>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for ActorsDeserializer<'_> {
    type Value = Vec<Box<dyn RaceBundle>>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut actors = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        while let Some(reflect) =
            seq.next_element_seed(UntypedReflectDeserializer::new(self.registry))?
        {
            let type_name = reflect.type_name();
            let registration = self
                .registry
                .get(reflect.type_id())
                .ok_or_else(|| de::Error::custom(format!("{type_name} is not registered")))?;
            let reflect_race = registration.data::<ReflectRaceBundle>().ok_or_else(|| {
                de::Error::custom(format!("{type_name} doesn't have reflect(RaceBundle)"))
            })?;
            let race_bundle = reflect_race.get_boxed(reflect).map_err(|reflect| {
                de::Error::custom(format!("{} is not a race RaceBundle", reflect.type_name()))
            })?;

            actors.push(race_bundle);
        }

        Ok(actors)
    }
}

#[cfg(test)]
mod tests {
    use serde_test::Token;

    use super::*;

    #[test]
    fn family_spawn_ser() {
        const NAME: &str = "Dummy family";
        let mut registry = TypeRegistryInternal::new();
        registry.register::<DummyRaceBundle>();
        let family_spawn = FamilySpawn {
            city_entity: Entity::PLACEHOLDER,
            scene: FamilyScene {
                name: NAME.into(),
                budget: Budget(100),
                actors: vec![Box::<DummyRaceBundle>::default()],
            },
            select: true,
        };
        let serializer = FamilySpawnSerializer::new(&family_spawn, &registry);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Struct {
                    name: any::type_name::<FamilySpawn>(),
                    len: FamilySpawnField::VARIANTS.len(),
                },
                Token::Str(FamilySpawnField::CityEntity.into()),
                Token::U64(family_spawn.city_entity.to_bits()),
                Token::Str(FamilySpawnField::Scene.into()),
                Token::Struct {
                    name: any::type_name::<FamilyScene>(),
                    len: FamilySceneField::VARIANTS.len(),
                },
                Token::Str(FamilySceneField::Name.into()),
                Token::Str(NAME),
                Token::Str(FamilySceneField::Budget.into()),
                Token::NewtypeStruct { name: "Budget" },
                Token::U32(family_spawn.scene.budget.0),
                Token::Str(FamilySceneField::Actors.into()),
                Token::Seq { len: Some(1) },
                Token::Map { len: Some(1) },
                Token::Str(any::type_name::<DummyRaceBundle>()),
                Token::Struct {
                    name: "DummyRaceBundle",
                    len: 1,
                },
                Token::Str("dummy"),
                Token::Struct {
                    name: "DummyComponent",
                    len: 0,
                },
                Token::StructEnd,
                Token::StructEnd,
                Token::MapEnd,
                Token::SeqEnd,
                Token::StructEnd,
                Token::Str(FamilySpawnField::Select.into()),
                Token::Bool(family_spawn.select),
                Token::StructEnd,
            ],
        );
    }

    #[derive(Reflect, Bundle, Default, Debug)]
    struct DummyRaceBundle {
        dummy: DummyComponent,
    }

    impl RaceBundle for DummyRaceBundle {
        fn glyph(&self) -> &'static str {
            unimplemented!()
        }
    }

    #[derive(Component, Reflect, Default, Debug)]
    struct DummyComponent;
}
