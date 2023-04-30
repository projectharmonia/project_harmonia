use bevy::{ecs::world::EntityMut, prelude::*, reflect::FromType};

/// A struct used to operate on reflected [`Bundle`] of a type.
///
/// A [`ReflectBundle`] for type `T` can be obtained via
/// [`bevy_reflect::TypeRegistration::data`].
#[derive(Clone)]
pub(super) struct ReflectBundle {
    insert: fn(&mut EntityMut),
}

impl ReflectBundle {
    /// Inserts a default [`Bundle`] into the entity.
    pub(super) fn insert_default(&self, entity: &mut EntityMut) {
        (self.insert)(entity);
    }
}

impl<C: Bundle + Default> FromType<C> for ReflectBundle {
    fn from_type() -> Self {
        Self {
            insert: |entity| {
                let bundle = C::default();
                entity.insert(bundle);
            },
        }
    }
}
