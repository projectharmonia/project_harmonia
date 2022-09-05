use bevy::{
    ecs::{archetype::Archetype, component::ComponentId},
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::core::{city::City, game_world::GameEntity};

/// Contains [`ComponentId`]'s that used to decide if a component or
/// the whole archetype should be excluded from serialization.
pub(crate) struct IgnoreRules {
    /// Components that should never be serialized.
    ignored: HashSet<ComponentId>,
    /// Ignore a key component if its value is present in an archetype.
    ignored_if_present: HashMap<ComponentId, ComponentId>,
    /// ID of [`GameWorld`] component, only entities with this components should be serialized.
    game_entity_id: ComponentId,
}

impl FromWorld for IgnoreRules {
    fn from_world(world: &mut World) -> Self {
        let ignored = HashSet::from([
            world.init_component::<Camera>(),
            world.init_component::<GlobalTransform>(),
            world.init_component::<Visibility>(),
            world.init_component::<ComputedVisibility>(),
        ]);

        let ignored_if_present = HashMap::from([(
            world.init_component::<Transform>(),
            world.init_component::<City>(),
        )]);

        let game_entity_id = world.init_component::<GameEntity>();

        Self {
            ignored,
            ignored_if_present,
            game_entity_id,
        }
    }
}

impl IgnoreRules {
    /// Returns `true` if an archetype should be ignored.
    pub(crate) fn ignored_archetype(&self, archetype: &Archetype) -> bool {
        !archetype.contains(self.game_entity_id)
    }

    /// Returns `true` if a component of an archetype should be ignored.
    pub(crate) fn ignored_component(
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
