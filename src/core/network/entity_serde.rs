//! Provides custom [`serialize`] and [`deserialize`] functions for [`Entity`] to serialize all bits.
//! See <https://github.com/bevyengine/bevy/issues/6143>.
//! Made for use with `#[serde(with = ...)]` macro.
use std::fmt::{self, Formatter};

use bevy::prelude::*;
use serde::{
    de::{DeserializeSeed, Error, Visitor},
    Deserializer, Serialize, Serializer,
};

pub(crate) struct EntitySerializer(pub(crate) Entity);

impl Serialize for EntitySerializer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self::serialize(&self.0, serializer)
    }
}

pub(crate) struct EntityDeserializer;

impl<'de> DeserializeSeed<'de> for EntityDeserializer {
    type Value = Entity;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self::deserialize(deserializer)
    }
}

impl<'de> Visitor<'de> for EntityDeserializer {
    type Value = Entity;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("Entity")
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Entity::from_bits(v))
    }
}

pub(crate) fn serialize<S: Serializer>(entity: &Entity, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u64(entity.to_bits())
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Entity, D::Error> {
    deserializer.deserialize_u64(EntityDeserializer)
}
