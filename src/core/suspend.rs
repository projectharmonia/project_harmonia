use std::marker::PhantomData;

use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
    reflect::GetTypeRegistration,
};

use super::network::replication::replication_rules::AppReplicationExt;

pub(super) struct SuspendPlugin;

impl Plugin for SuspendPlugin {
    fn build(&self, app: &mut App) {
        app.register_suspend::<Transform>();
    }
}

trait AppSuspendExt {
    /// Registers [`Suspended<T>`] and disables replication of `T` if [`Suspended<T>`] is present.
    ///
    /// Component `T` should be marked for replication.
    fn register_suspend<T: Component + FromWorld + Reflect + GetTypeRegistration>(
        &mut self,
    ) -> &mut Self;
}

impl AppSuspendExt for App {
    fn register_suspend<T: Component + FromWorld + Reflect + GetTypeRegistration>(
        &mut self,
    ) -> &mut Self {
        self.register_type::<T>()
            .not_replicate_if_present::<T, Suspended<T>>()
    }
}

/// Stores value of component `T` and disables its replication.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct Suspended<T: FromWorld + Reflect>(T);

impl<T: FromWorld + Reflect> FromWorld for Suspended<T> {
    fn from_world(world: &mut World) -> Self {
        Self(T::from_world(world))
    }
}

pub(super) trait SuspendCommandsExt {
    /// Adds [`Suspend<T>`] to the entity with the current value of `T`.
    fn suspend<T: Component + Clone + Reflect + FromWorld>(&mut self) -> &mut Self;

    /// Sets value of `T` to the value from [`Suspend<T>`] and removes [`Suspend<T>`] from the entity.
    fn restore_suspended<T: Component + Reflect + FromWorld>(&mut self) -> &mut Self;
}

impl SuspendCommandsExt for EntityCommands<'_, '_, '_> {
    fn suspend<T: Component + Clone + Reflect + FromWorld>(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands().add(Suspend::<T>::new(entity));
        self
    }

    fn restore_suspended<T: Component + Reflect + FromWorld>(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands().add(ResotreSuspended::<T>::new(entity));
        self
    }
}

struct Suspend<T> {
    entity: Entity,
    marker: PhantomData<T>,
}

impl<T> Suspend<T> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }
}

impl<T: Component + Clone + Reflect + FromWorld> Command for Suspend<T> {
    fn write(self, world: &mut World) {
        let mut entity = world.entity_mut(self.entity);
        let component = entity
            .get::<T>()
            .expect("suspended component should be on the entity");
        entity.insert(Suspended(component.clone()));
    }
}

struct ResotreSuspended<T> {
    entity: Entity,
    marker: PhantomData<T>,
}

impl<T> ResotreSuspended<T> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }
}

impl<T: Component + Reflect + FromWorld> Command for ResotreSuspended<T> {
    fn write(self, world: &mut World) {
        let mut entity = world.entity_mut(self.entity);
        let paused = entity
            .remove::<Suspended<T>>()
            .expect("suspended component should be on the entity");
        entity.insert(paused.0);
    }
}
