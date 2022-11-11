use bevy::{
    ecs::{archetype::Archetype, component::ComponentId},
    prelude::*,
    utils::{HashMap, HashSet},
};

use super::parent_sync::ParentSync;
use crate::core::{
    city::City,
    doll::{FirstName, LastName},
    family::{Budget, Members},
    game_world::GameEntity,
    object::ObjectPath,
};

/// Contains [`ComponentId`]'s that used to decide
/// if a component should be serialized.
pub(crate) struct SaveRules {
    /// Components that should be serialized.
    pub(crate) persistent: HashSet<ComponentId>,
    /// Ignore a key component if its value is present in an archetype.
    ignored_if_present: HashMap<ComponentId, ComponentId>,
    /// ID of [`GameWorld`] component, only entities with this components should be serialized.
    game_entity_id: ComponentId,
}

impl FromWorld for SaveRules {
    fn from_world(world: &mut World) -> Self {
        let persistent = HashSet::from([
            world.init_component::<Transform>(),
            world.init_component::<Name>(),
            world.init_component::<City>(),
            world.init_component::<Members>(),
            world.init_component::<Budget>(),
            world.init_component::<ObjectPath>(),
            world.init_component::<GameEntity>(),
            world.init_component::<ParentSync>(),
            world.init_component::<FirstName>(),
            world.init_component::<LastName>(),
        ]);

        let ignored_if_present = HashMap::from([
            (
                world.init_component::<Transform>(),
                world.init_component::<City>(),
            ),
            (
                world.init_component::<Name>(),
                world.init_component::<FirstName>(),
            ),
        ]);

        let game_entity_id = world.init_component::<GameEntity>();

        Self {
            persistent,
            ignored_if_present,
            game_entity_id,
        }
    }
}

impl SaveRules {
    /// Returns `true` if an entity of an archetype should be serialized.
    pub(crate) fn persistent_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.game_entity_id)
    }

    /// Returns `true` if a component of an archetype should be serialized.
    pub(crate) fn persistent_component(
        &self,
        archetype: &Archetype,
        component_id: ComponentId,
    ) -> bool {
        if self.persistent.contains(&component_id) {
            if let Some(&conditional_id) = self.ignored_if_present.get(&component_id) {
                if archetype.contains(conditional_id) {
                    return false;
                }
            }
            return true;
        }

        false
    }
}
