//! Provides custom [`serialize`] and [`deserialize`] functions for [`Entity`] to serialize all bits.
//! See <https://github.com/bevyengine/bevy/issues/6143>.
//! Made for use with `#[serde(with = ...)]` macro.
use std::fmt::{self, Formatter};

use bevy::prelude::*;
use serde::{
    de::{Error, Visitor},
    Deserializer, Serializer,
};

pub(crate) fn serialize<S>(entity: &Entity, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(entity.to_bits())
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Entity, D::Error> {
    deserializer.deserialize_u64(EntityVisitor)
}

struct EntityVisitor;

impl<'de> Visitor<'de> for EntityVisitor {
    type Value = Entity;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("Entity")
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Entity::from_bits(v))
    }
}
