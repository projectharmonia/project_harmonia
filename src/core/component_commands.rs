use bevy::{
    ecs::{
        reflect::ReflectBundle,
        system::{EntityCommand, EntityCommands},
    },
    prelude::*,
};

pub(super) trait ComponentCommandsExt {
    fn insert_reflect_bundle(&mut self, bundle: Box<dyn Reflect>) -> &mut Self;
}

impl ComponentCommandsExt for EntityCommands<'_, '_, '_> {
    fn insert_reflect_bundle(&mut self, bundle: Box<dyn Reflect>) -> &mut Self {
        self.add(InsertReflectBundle(bundle));
        self
    }
}

struct InsertReflectBundle(Box<dyn Reflect>);

impl EntityCommand for InsertReflectBundle {
    fn apply(self, entity: Entity, world: &mut World) {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let type_info = self.0.get_represented_type_info().unwrap();
        let type_path = type_info.type_path();
        let registration = registry
            .get(type_info.type_id())
            .unwrap_or_else(|| panic!("{type_path} should be registered"));
        let reflect_bundle = registration
            .data::<ReflectBundle>()
            .unwrap_or_else(|| panic!("{type_path} should have reflect(Bundle)"));

        reflect_bundle.insert(&mut world.entity_mut(entity), &*self.0);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::CommandQueue;

    use super::*;

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
