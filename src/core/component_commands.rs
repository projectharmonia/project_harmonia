use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
};
use derive_more::Constructor;

pub(super) trait ComponentCommandsExt {
    fn insert_components(&mut self, components: Vec<Box<dyn Reflect>>) -> &mut Self;
}

impl ComponentCommandsExt for EntityCommands<'_, '_, '_> {
    fn insert_components(&mut self, components: Vec<Box<dyn Reflect>>) -> &mut Self {
        let command = InsertComponents::new(self.id(), components);
        self.commands().add(command);
        self
    }
}

#[derive(Constructor)]
struct InsertComponents {
    entity: Entity,
    components: Vec<Box<dyn Reflect>>,
}

impl Command for InsertComponents {
    fn write(self, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        for component in self.components {
            let type_name = component.type_name();
            let registration = registry
                .get_with_name(type_name)
                .unwrap_or_else(|| panic!("{type_name} should be registered"));

            let reflect_component = registration
                .data::<ReflectComponent>()
                .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));

            reflect_component.apply_or_insert(world, self.entity, &*component);
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
            .insert_components(vec![DummyComponent.clone_value()]);

        queue.apply(&mut world);

        assert!(world.entity(entity).contains::<DummyComponent>());
    }

    #[derive(Component, Default, Reflect)]
    #[reflect(Component)]
    struct DummyComponent;
}
