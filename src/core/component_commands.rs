use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
};
use derive_more::Constructor;

pub(super) trait ComponentCommandsExt {
    fn insert_components<T>(&mut self, components: T) -> &mut Self
    where
        T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static;
}

impl ComponentCommandsExt for EntityCommands<'_, '_, '_> {
    fn insert_components<T>(&mut self, components: T) -> &mut Self
    where
        T: IntoIterator<Item = Box<dyn Reflect>> + Send + 'static,
    {
        let command = InsertComponents::new(self.id(), components);
        self.commands().add(command);
        self
    }
}

#[derive(Constructor)]
struct InsertComponents<T: IntoIterator<Item = Box<dyn Reflect>>> {
    entity: Entity,
    components: T,
}

impl<T> Command for InsertComponents<T>
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
            .insert_components([DummyComponent.clone_value()]);

        queue.apply(&mut world);

        assert!(world.entity(entity).contains::<DummyComponent>());
    }

    #[derive(Component, Default, Reflect)]
    #[reflect(Component)]
    struct DummyComponent;
}
