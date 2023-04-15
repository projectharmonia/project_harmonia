use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
};

pub(super) trait ComponentCommandsExt {
    fn remove_by_name(&mut self, name: String) -> &mut Self;

    fn insert_reflect<T>(&mut self, components: T) -> &mut Self
    where
        T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static;
}

impl ComponentCommandsExt for EntityCommands<'_, '_, '_> {
    fn remove_by_name(&mut self, name: String) -> &mut Self {
        let command = RemoveByName {
            entity: self.id(),
            name,
        };
        self.commands().add(command);
        self
    }

    fn insert_reflect<T>(&mut self, components: T) -> &mut Self
    where
        T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static,
    {
        let command = InsertReflect {
            entity: self.id(),
            components,
        };
        self.commands().add(command);
        self
    }
}

struct RemoveByName {
    entity: Entity,
    name: String,
}

impl Command for RemoveByName {
    fn write(self, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let registration = registry
            .get_with_name(&self.name)
            .unwrap_or_else(|| panic!("{} should be registered", self.name));
        let reflect_component = registration
            .data::<ReflectComponent>()
            .unwrap_or_else(|| panic!("{} should have reflect(Component)", self.name));
        let mut entity = world.entity_mut(self.entity);
        reflect_component.remove(&mut entity);
    }
}

struct InsertReflect<T: IntoIterator<Item = Box<dyn Reflect>>> {
    entity: Entity,
    components: T,
}

impl<T> Command for InsertReflect<T>
where
    T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static,
{
    fn write(self, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let mut entity = world.entity_mut(self.entity);
        for component in self.components {
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::CommandQueue;

    use super::*;

    #[test]
    fn insertion() {
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
    fn remove_by_name() {
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

    #[derive(Component, Default, Reflect)]
    #[reflect(Component)]
    struct DummyComponent;
}
