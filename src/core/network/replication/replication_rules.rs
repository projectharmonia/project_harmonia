use std::marker::PhantomData;

use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
    reflect::GetTypeRegistration,
    utils::{HashMap, HashSet},
};
use bevy_trait_query::imports::{Archetype, ComponentId};

pub(crate) struct ReplicationRulesPlugin;

impl Plugin for ReplicationRulesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Replication>()
            .init_resource::<ReplicationRules>();
    }
}

pub(crate) trait AppReplicationExt {
    /// A shorthand for [`App::register_type`] with [`Self::replicate`].
    fn register_and_replicate<T: Component + GetTypeRegistration>(&mut self) -> &mut Self;

    /// Marks component for replication.
    ///
    /// The component should be registered, implement [`Reflect`] and have `#[reflect(Component)]`.
    fn replicate<T: Component>(&mut self) -> &mut Self;

    /// Registers [`Pause<T>`] as a component that pauses replication for `T` when present.
    ///
    /// The component `T` should be marked for replication.
    fn enable_replication_pause<T: Component + FromWorld + GetTypeRegistration + Reflect>(
        &mut self,
    ) -> &mut Self;

    /// Ignores component `T` replication if component `U` is present on the same entity.
    ///
    /// Component `T` should be marked for replication.
    /// Could be called multiple times for the same component to disable replication
    /// for different presented components.
    fn not_replicate_if_present<T: Component, U: Component>(&mut self) -> &mut Self;
}

impl AppReplicationExt for App {
    fn register_and_replicate<T: Component + GetTypeRegistration>(&mut self) -> &mut Self {
        self.register_type::<T>().replicate::<T>()
    }

    fn replicate<T: Component>(&mut self) -> &mut Self {
        let component_id = self.world.init_component::<T>();
        let mut replication_rules = self.world.resource_mut::<ReplicationRules>();
        replication_rules.replicated.insert(component_id);
        self
    }

    fn enable_replication_pause<T: Component + FromWorld + GetTypeRegistration + Reflect>(
        &mut self,
    ) -> &mut Self {
        self.register_type::<Paused<T>>();
        let component_id = self.world.init_component::<T>();
        let paused_id = self.world.init_component::<Paused<T>>();
        let mut replication_rules = self.world.resource_mut::<ReplicationRules>();
        replication_rules
            .pausable
            .insert(component_id, paused_id);
        self
    }

    fn not_replicate_if_present<T: Component, U: Component>(&mut self) -> &mut Self {
        let ignore_id = self.world.init_component::<T>();
        let present_id = self.world.init_component::<U>();
        let mut replication_rules = self.world.resource_mut::<ReplicationRules>();
        replication_rules
            .ignored_if_present
            .entry(ignore_id)
            .or_default()
            .push(present_id);
        self
    }
}

/// Contains [`ComponentId`]'s that used to decide
/// if a component should be replicated.
#[derive(Resource)]
pub(crate) struct ReplicationRules {
    /// Components that should be replicated.
    pub(super) replicated: HashSet<ComponentId>,

    /// Ignore a key component if any of its value components are present in an archetype.
    ignored_if_present: HashMap<ComponentId, Vec<ComponentId>>,

    /// Contains a pausable [`ComponentId`] as a value for replicated components.
    pausable: HashMap<ComponentId, ComponentId>,

    /// ID of [`Replication`] component, only entities with this components should be replicated.
    replication_id: ComponentId,
}

impl ReplicationRules {
    /// Returns `true` if an entity of an archetype should be replicated.
    pub(crate) fn is_replicated_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.replication_id)
    }

    /// Returns `true` if a component of an archetype should be replicated.
    pub(crate) fn is_replicated_component(
        &self,
        archetype: &Archetype,
        component_id: ComponentId,
    ) -> bool {
        if self.replicated.contains(&component_id) {
            if let Some(ignore_ids) = self.ignored_if_present.get(&component_id) {
                for &ignore_id in ignore_ids {
                    if archetype.contains(ignore_id) {
                        return false;
                    }
                }
            }
            return true;
        }

        false
    }

    pub(crate) fn pausable_id(&self, component_id: ComponentId) -> Option<ComponentId> {
        self.pausable.get(&component_id).copied()
    }
}

impl FromWorld for ReplicationRules {
    fn from_world(world: &mut World) -> Self {
        Self {
            replicated: Default::default(),
            ignored_if_present: Default::default(),
            pausable: Default::default(),
            replication_id: world.init_component::<Replication>(),
        }
    }
}

/// Marks entity for replication.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Replication;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct Paused<T: FromWorld + Reflect>(T);

impl<T: FromWorld + Reflect> FromWorld for Paused<T> {
    fn from_world(world: &mut World) -> Self {
        Self(T::from_world(world))
    }
}

trait ReplicationCommandsExt<T> {
    fn pause_replication(&mut self) -> &mut Self;
    fn restore_replication(&mut self) -> &mut Self;
}

impl<T: Component + Clone + Reflect + FromWorld> ReplicationCommandsExt<T>
    for EntityCommands<'_, '_, '_>
{
    fn pause_replication(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands().add(PauseReplication::<T>::new(entity));
        self
    }

    fn restore_replication(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands().add(RestoreReplication::<T>::new(entity));
        self
    }
}

struct PauseReplication<T> {
    entity: Entity,
    marker: PhantomData<T>,
}

impl<T> PauseReplication<T> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }
}

impl<T: Component + Clone + Reflect + FromWorld> Command for PauseReplication<T> {
    fn write(self, world: &mut World) {
        let mut entity = world.entity_mut(self.entity);
        let component = entity
            .get::<T>()
            .expect("paused component for replication should be on the entity");
        entity.insert(Paused(component.clone()));
    }
}

struct RestoreReplication<T> {
    entity: Entity,
    marker: PhantomData<T>,
}

impl<T> RestoreReplication<T> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }
}

impl<T: Component + Clone + Reflect + FromWorld> Command for RestoreReplication<T> {
    fn write(self, world: &mut World) {
        let mut entity = world.entity_mut(self.entity);
        let paused = entity
            .remove::<Paused<T>>()
            .expect("component for replication pause should be on the entity");
        entity.insert(paused.0);
    }
}
