use avian3d::prelude::*;
use bevy::prelude::*;

pub(super) struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init).add_observer(cleanup);
    }
}

fn init(
    trigger: Trigger<OnAdd, Ghost>,
    ghosts: Query<&mut Ghost>,
    mut targets: Query<(&mut Visibility, Option<&mut CollisionLayers>)>,
) {
    let ghost = ghosts.get(trigger.entity()).unwrap();
    let (mut visibility, collision_layers) = targets.get_mut(ghost.original_entity).unwrap();

    *visibility = Visibility::Hidden;
    debug!(
        "changing visibility to `{:?}` for `{}`",
        *visibility, ghost.original_entity
    );

    if let Some(mut collision_layers) = collision_layers {
        if ghost.filters != LayerMask::NONE {
            collision_layers.filters.remove(ghost.filters);
        }
    }
}

fn cleanup(
    trigger: Trigger<OnRemove, Ghost>,
    ghosts: Query<&mut Ghost>,
    mut targets: Query<(&mut Visibility, Option<&mut CollisionLayers>)>,
) {
    let ghost = ghosts.get(trigger.entity()).unwrap();
    let Ok((mut visibility, collision_layers)) = targets.get_mut(ghost.original_entity) else {
        // If entity missing visibility, consider it despawned.
        return;
    };

    *visibility = Visibility::Inherited;
    debug!(
        "changing visibility to `{:?}` for `{}`",
        *visibility, ghost.original_entity
    );

    if let Some(mut collision_layers) = collision_layers {
        if ghost.filters != LayerMask::NONE {
            collision_layers.filters.add(ghost.filters);
        }
    }
}

/// Entity that displayed instead of the original.
#[derive(Component, Clone, Copy)]
pub(super) struct Ghost {
    /// Entity to which the ghost corresponds.
    ///
    /// Original entity will be hidden until this component is present.
    original_entity: Entity,

    /// Collision layer filters that will be temporarily removed until this component is present.
    filters: LayerMask,
}

impl Ghost {
    pub(super) fn new(original_entity: Entity) -> Self {
        Self {
            original_entity,
            filters: LayerMask::NONE,
        }
    }

    pub(super) fn with_filters(mut self, remove_filters: impl Into<LayerMask>) -> Self {
        self.filters = remove_filters.into();
        self
    }
}
