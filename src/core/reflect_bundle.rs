//! Simplified version from <https://github.com/bevyengine/bevy/pull/6344>

use bevy::{ecs::world::EntityMut, prelude::*, reflect::FromType};

/// A struct used to operate on reflected [`Bundle`] of a type.
///
/// A [`ReflectBundle`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub(super) struct ReflectBundle {
    /// Function pointer implementing [`ReflectBundle::insert()`].
    pub(super) insert: fn(&mut EntityMut, &dyn Reflect),
}

impl ReflectBundle {
    /// Insert a reflected [`Bundle`] into the entity like [`insert()`](crate::world::EntityMut::insert).
    ///
    /// # Panics
    ///
    /// Panics if there is no such entity.
    pub(super) fn insert(&self, entity: &mut EntityMut, reflect: &dyn Reflect) {
        (self.insert)(entity, reflect);
    }
}

impl<C: Bundle + Reflect + FromWorld> FromType<C> for ReflectBundle {
    fn from_type() -> Self {
        ReflectBundle {
            insert: |entity, reflect| {
                let mut bundle = entity.world_scope(|world| C::from_world(world));
                bundle.apply(reflect);
                entity.insert(bundle);
            },
        }
    }
}
