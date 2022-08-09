use std::any;

use bevy::{
    ecs::{archetype::Archetype, component::ComponentId},
    prelude::*,
    utils::{HashMap, HashSet},
};
use once_cell::sync::OnceCell;

use crate::core::{city::City, game_world::GameEntity};

/// Caches [`ComponentId`] that used to decide if a component or
/// the whole archetype should be excluded from serialization.
///
/// Since [`World`] registers [`ComponentId`] on first use, this struct also
/// should be initialized lazily to ensure that all necessary components has been registered.
pub(super) struct IgnoreRules {
    /// Components that should never be serialized.
    ignored: HashSet<ComponentId>,
    /// Ignore a key component if its value is present in an archetype.
    ignored_if_present: HashMap<ComponentId, ComponentId>,
    /// ID of [`GameWorld`] component, only entities with this components should be serialized.
    game_entity_id: ComponentId,
}

impl IgnoreRules {
    /// Initializes on first call and returns [`IgnoredComponents`] with [`ComponentId`] from [`World`].
    pub(super) fn global(world: &World) -> &Self {
        static IGNORED_COMPONENTS: OnceCell<IgnoreRules> = OnceCell::new();
        IGNORED_COMPONENTS.get_or_init(|| {
            let ignored = HashSet::from([
                component_id::<Camera>(world),
                component_id::<GlobalTransform>(world),
                component_id::<Visibility>(world),
                component_id::<ComputedVisibility>(world),
            ]);

            let ignored_if_present = HashMap::from([(
                component_id::<Transform>(world),
                component_id::<City>(world),
            )]);

            let game_entity_id = component_id::<GameEntity>(world);

            Self {
                ignored,
                ignored_if_present,
                game_entity_id,
            }
        })
    }

    /// Returns `true` if an archetype should be ignored.
    pub(super) fn ignored_archetype(&self, archetype: &Archetype) -> bool {
        !archetype.contains(self.game_entity_id)
    }

    /// Returns `true` if a component of an archetype should be ignored.
    pub(super) fn ignored_component(
        &self,
        archetype: &Archetype,
        component_id: ComponentId,
    ) -> bool {
        if let Some(conditional_id) = self.ignored_if_present.get(&component_id) {
            if archetype.contains(*conditional_id) {
                return true;
            }
        }

        self.ignored.contains(&component_id)
    }
}

/// Gets [`ComponentId`] from world or panics with a nice message if it wasn't registered.
fn component_id<C: Component>(world: &World) -> ComponentId {
    world
        .component_id::<C>()
        .unwrap_or_else(|| panic!("Unable to get ID for {}", any::type_name::<C>()))
}
