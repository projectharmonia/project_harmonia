use bevy::{
    ecs::system::{EntityCommand, EntityCommands},
    prelude::*,
};

use super::reflect_bundle::ReflectBundle;

pub(super) trait ComponentCommandsExt {
    fn remove_by_name(&mut self, name: String) -> &mut Self;

    fn insert_reflect<T>(&mut self, components: T) -> &mut Self
    where
        T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static;

    fn insert_reflect_bundle(&mut self, bundle: Box<dyn Reflect>) -> &mut Self;
}

impl ComponentCommandsExt for EntityCommands<'_, '_, '_> {
    fn remove_by_name(&mut self, name: String) -> &mut Self {
        self.add(RemoveByName(name));
        self
    }

    fn insert_reflect<T>(&mut self, components: T) -> &mut Self
    where
        T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static,
    {
        self.add(InsertReflect(components));
        self
    }

    fn insert_reflect_bundle(&mut self, bundle: Box<dyn Reflect>) -> &mut Self {
        self.add(InsertReflectBundle(bundle));
        self
    }
}

struct RemoveByName(String);

impl EntityCommand for RemoveByName {
    fn apply(self, entity: Entity, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let registration = registry
            .get_with_name(&self.0)
            .unwrap_or_else(|| panic!("{} should be registered", self.0));
        let reflect_component = registration
            .data::<ReflectComponent>()
            .unwrap_or_else(|| panic!("{} should have reflect(Component)", self.0));
        let mut entity = world.entity_mut(entity);
        reflect_component.remove(&mut entity);
    }
}

struct InsertReflect<T: IntoIterator<Item = Box<dyn Reflect>>>(T);

impl<T> EntityCommand for InsertReflect<T>
where
    T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static,
{
    fn apply(self, entity: Entity, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let mut entity = world.entity_mut(entity);
        for component in self.0 {
            let type_name = component.type_name();
            let registration = registry
                .get_with_name(type_name)
                .unwrap_or_else(|| panic!("{type_name} should be registered"));
            let reflect_component = registration
                .data::<ReflectComponent>()
                .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));

            reflect_component.apply_or_insert(&mut entity, &*component);
        }
    }
}

struct InsertReflectBundle(Box<dyn Reflect>);

impl EntityCommand for InsertReflectBundle {
    fn apply(self, entity: Entity, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let type_name = self.0.type_name();
        let registration = registry
            .get_with_name(type_name)
            .unwrap_or_else(|| panic!("{type_name} should be registered"));
        let reflect_bundle = registration
            .data::<ReflectBundle>()
            .unwrap_or_else(|| panic!("{type_name} should have reflect(Bundle)"));

        reflect_bundle.insert(&mut world.entity_mut(entity), &*self.0);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::CommandQueue;

    use super::*;

    #[test]
    fn removal_by_name() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource::<AppTypeRegistry>()
            .write()
            .register::<DummyComponent>();

        let entity = world.spawn(DummyComponent).id();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .remove_by_name(DummyComponent.type_name().to_string());

        queue.apply(&mut world);

        assert!(!world.entity(entity).contains::<DummyComponent>());
    }

    #[test]
    fn reflect_insertion() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource::<AppTypeRegistry>()
            .write()
            .register::<DummyComponent>();

        let entity = world.spawn_empty().id();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .insert_reflect([DummyComponent.clone_value()]);

        queue.apply(&mut world);

        assert!(world.entity(entity).contains::<DummyComponent>());
    }

    #[test]
    fn reflect_bundle_insertion() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource::<AppTypeRegistry>()
            .write()
            .register::<DummyBundle>();

        let entity = world.spawn_empty().id();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .insert_reflect_bundle(DummyBundle::default().clone_value());

        queue.apply(&mut world);

        assert!(world.entity(entity).contains::<DummyComponent>());
    }

    #[derive(Bundle, Default, Reflect)]
    #[reflect(Bundle)]
    struct DummyBundle {
        dummy: DummyComponent,
    }

    #[derive(Component, Default, Reflect)]
    #[reflect(Component)]
    struct DummyComponent;
}
