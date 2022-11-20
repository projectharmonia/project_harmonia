use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
    reflect::TypeRegistry,
};

pub(super) trait ComponentCommandsExt {
    fn remove_by_name(&mut self, type_name: &'static str);
    fn insert_reflect(&mut self, component: Box<dyn Reflect>);
}

impl ComponentCommandsExt for EntityCommands<'_, '_, '_> {
    fn remove_by_name(&mut self, type_name: &'static str) {
        let entity = self.id();
        self.commands().add(RemoveByName { entity, type_name });
    }

    fn insert_reflect(&mut self, component: Box<dyn Reflect>) {
        let entity = self.id();
        self.commands().add(InsertReflect { entity, component });
    }
}

struct RemoveByName {
    entity: Entity,
    type_name: &'static str,
}

impl Command for RemoveByName {
    fn write(self, world: &mut World) {
        let RemoveByName { entity, type_name } = self;
        let registry = world.resource::<TypeRegistry>().clone();
        let registry = registry.read();

        let registration = registry
            .get_with_name(type_name)
            .unwrap_or_else(|| panic!("{type_name} should be registered"));

        let reflect_component = registration
            .data::<ReflectComponent>()
            .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));

        reflect_component.remove(world, entity);
    }
}

struct InsertReflect {
    entity: Entity,
    component: Box<dyn Reflect>,
}

impl Command for InsertReflect {
    fn write(self, world: &mut World) {
        let type_name = self.component.type_name();
        let registry = world.resource::<TypeRegistry>().clone();
        let registry = registry.read();

        let registration = registry
            .get_with_name(type_name)
            .unwrap_or_else(|| panic!("{type_name} should be registered"));

        let reflect_component = registration
            .data::<ReflectComponent>()
            .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));

        reflect_component.apply_or_insert(world, self.entity, &*self.component);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::CommandQueue;

    use super::*;

    #[test]
    fn remove_by_name() {
        let mut world = World::new();
        let registry = TypeRegistry::default();
        registry.write().register::<TestComponent>();
        world.insert_resource(registry);

        let entity = world.spawn().insert(TestComponent).id();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .remove_by_name(TestComponent.type_name());
        queue.apply(&mut world);

        assert!(world.get::<TestComponent>(entity).is_none());
    }

    #[test]
    fn insert_reflect() {
        let mut world = World::new();
        let registry = TypeRegistry::default();
        registry.write().register::<TestComponent>();
        world.insert_resource(registry);

        let entity = world.spawn().id();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands
            .entity(entity)
            .insert_reflect(TestComponent.clone_value());
        queue.apply(&mut world);

        assert!(world.get::<TestComponent>(entity).is_some());
    }

    #[derive(Component, Default, Reflect)]
    #[reflect(Component)]
    struct TestComponent;
}
