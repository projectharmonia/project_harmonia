use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};
use bevy_xpbd_3d::prelude::*;

/// Entity that displayed instead of the original.
#[derive(Clone, Copy)]
pub(super) struct Ghost {
    /// Entity to which the ghost corresponds.
    ///
    /// Original entity will be hidden until this component is present.
    original_entity: Entity,

    /// Collision layer filters that will be temporarely removed until this component is present.
    remove_filters: LayerMask,
}

impl Ghost {
    pub(super) fn new(original_entity: Entity) -> Self {
        Self {
            original_entity,
            remove_filters: LayerMask::NONE,
        }
    }

    pub(super) fn with_remove_filters(mut self, remove_filters: impl Into<LayerMask>) -> Self {
        self.remove_filters = remove_filters.into();
        self
    }
}

impl Component for Ghost {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks
            .on_add(|mut world, targeted_entity, _component_id| {
                let ghost = *world.get::<Self>(targeted_entity).unwrap();
                if let Some(mut visibility) = world.get_mut::<Visibility>(ghost.original_entity) {
                    *visibility = Visibility::Hidden;
                }
                if ghost.remove_filters != LayerMask::NONE {
                    if let Some(mut collision_layers) =
                        world.get_mut::<CollisionLayers>(ghost.original_entity)
                    {
                        collision_layers.filters.remove(ghost.remove_filters);
                    }
                }
            })
            .on_remove(|mut world, targeted_entity, _component_id| {
                let ghost = *world.get::<Self>(targeted_entity).unwrap();
                if let Some(mut visibility) = world.get_mut::<Visibility>(ghost.original_entity) {
                    *visibility = Visibility::Inherited;
                }
                if ghost.remove_filters != LayerMask::NONE {
                    if let Some(mut collision_layers) =
                        world.get_mut::<CollisionLayers>(ghost.original_entity)
                    {
                        collision_layers.filters.add(ghost.remove_filters);
                    }
                }
            });
    }
}
