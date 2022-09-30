use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
    reflect::FromType,
};

/// Like [`ReflectMapEntities`], but maps only a single entity instead of all entities from [`EntityMap`].
#[derive(Clone)]
pub(crate) struct ReflectMapEntity {
    map_entities: fn(&mut World, &EntityMap, Entity) -> Result<(), MapEntitiesError>,
}

impl ReflectMapEntity {
    pub(crate) fn map_entities(
        &self,
        world: &mut World,
        entity_map: &EntityMap,
        entity: Entity,
    ) -> Result<(), MapEntitiesError> {
        (self.map_entities)(world, entity_map, entity)
    }
}

impl<C: Component + MapEntities> FromType<C> for ReflectMapEntity {
    fn from_type() -> Self {
        ReflectMapEntity {
            map_entities: |world, entity_map, entity| {
                let mut component = world
                    .get_mut::<C>(entity)
                    .expect("entity should have reflected component");
                component.map_entities(entity_map)?;
                Ok(())
            },
        }
    }
}
