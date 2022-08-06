use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    fs,
};

use anyhow::{Context, Result};
use bevy::{
    ecs::archetype::ArchetypeId,
    prelude::*,
    reflect::{
        serde::{ReflectDeserializer, ReflectSerializer},
        TypeRegistry,
    },
    utils::HashMap,
};
use iyes_loopless::prelude::*;
use serde::de::DeserializeSeed;

use super::{errors::log_err_system, game_paths::GamePaths, game_state::GameState};

#[derive(SystemLabel)]
enum GameWorldSystem {
    Saving,
}

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<InGameOnly>()
            .register_type::<Cow<'static, str>>() // https://github.com/bevyengine/bevy/issues/5597
            .add_event::<GameSaved>()
            .add_event::<GameLoaded>()
            .add_exit_system(
                GameState::InGame,
                Self::world_saving_system
                    .chain(log_err_system)
                    .run_on_event::<GameSaved>()
                    .label(GameWorldSystem::Saving),
            )
            .add_exit_system(
                GameState::InGame,
                Self::cleanup_world_system.after(GameWorldSystem::Saving),
            )
            .add_system(
                Self::world_saving_system
                    .chain(log_err_system)
                    .run_in_state(GameState::InGame)
                    .run_on_event::<GameSaved>(),
            );

        {
            // To avoid ambiguity: https://github.com/IyesGames/iyes_loopless/issues/15
            use iyes_loopless::condition::IntoConditionalExclusiveSystem;
            app.add_system(
                (|world: &mut World| log_err_system(In(Self::world_loading_system(world))))
                    .run_on_event::<GameLoaded>()
                    .at_start(),
            );
        }
    }
}

impl GameWorldPlugin {
    fn cleanup_world_system(
        mut commands: Commands,
        ingame_entities: Query<Entity, With<InGameOnly>>,
    ) {
        for entity in &ingame_entities {
            commands.entity(entity).despawn_recursive();
        }
        commands.remove_resource::<WorldName>();
    }

    fn world_saving_system(
        world: &World,
        world_name: Res<WorldName>,
        game_paths: Res<GamePaths>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&world_name.0);

        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("Unable to create {world_path:?}"))?;

        let bytes = rmp_serde::to_vec(&serialize_game_world(world).values().collect::<Vec<_>>())
            .expect("Unable to serlialize world");

        fs::write(&world_path, bytes)
            .with_context(|| format!("Unable to save game to {world_path:?}"))
    }

    fn world_loading_system(world: &mut World) -> Result<()> {
        let world_name = world.resource::<WorldName>();
        let game_paths = world.resource::<GamePaths>();
        let world_path = game_paths.world_path(&world_name.0);

        let bytes = fs::read(&world_path)
            .with_context(|| format!("Unable to load world from {world_path:?}"))?;

        let components = rmp_serde::from_slice::<Vec<Vec<Vec<u8>>>>(&bytes)
            .expect("Unable to deserialize game world");

        deserialize_game_world(world, components);

        Ok(())
    }
}

/// Iterates over a world and serializes all components that implement [`Reflect`]
/// on entities that have [`InGameOnly`] component.
fn serialize_game_world(world: &World) -> HashMap<Entity, Vec<Vec<u8>>> {
    let mut components = HashMap::new();
    let type_registry = world.resource::<TypeRegistry>().read();
    for archetype in world.archetypes().iter() {
        if matches!(
            archetype.id(),
            ArchetypeId::EMPTY | ArchetypeId::RESOURCE | ArchetypeId::INVALID
        ) {
            continue;
        }

        if archetype
            .components()
            .filter_map(|component_id| {
                // SAFETY: `component_id` retrieved from the world.
                unsafe { world.components().get_info_unchecked(component_id) }.type_id()
            })
            .all(|type_id| type_id != TypeId::of::<InGameOnly>())
        {
            // Not an ingame entity
            continue;
        }

        for reflect_component in archetype
            .components()
            .filter_map(|component_id| {
                // SAFETY: `component_id` retrieved from the world.
                unsafe { world.components().get_info_unchecked(component_id) }.type_id()
            })
            .filter_map(|type_id| type_registry.get(type_id))
            .filter_map(|registration| registration.data::<ReflectComponent>())
        {
            for entity in archetype.entities() {
                let reflect = reflect_component
                    .reflect(world, *entity)
                    .expect("Unable to reflect component");

                let serializer = ReflectSerializer::new(reflect, &type_registry);
                let type_name = reflect.type_name();
                let bytes = rmp_serde::to_vec(&serializer)
                    .unwrap_or_else(|error| panic!("Unable to serialize {type_name}: {error}"));
                let entry: &mut Vec<Vec<u8>> = components.entry(*entity).or_default();
                entry.push(bytes);
            }
        }
    }

    components
}

fn deserialize_game_world(world: &mut World, components: Vec<Vec<Vec<u8>>>) {
    // Temorary take resources to avoid borrowing issues
    let type_registry = world
        .remove_resource::<TypeRegistry>()
        .unwrap_or_else(|| panic!("Unable to extract {}", type_name::<TypeRegistry>()));
    let read_registry = type_registry.read();

    for entity_components in components {
        let entity = world.spawn().id();
        for component in entity_components {
            let reflect_deserializer = ReflectDeserializer::new(&read_registry);
            let mut deserializer = rmp_serde::Deserializer::from_read_ref(&component);

            let reflect = reflect_deserializer
                .deserialize(&mut deserializer)
                .expect("Unable to deserialize component");

            let type_name = reflect.type_name();
            let registration = read_registry
                .get_with_name(type_name)
                .unwrap_or_else(|| panic!("Unable to get registration for {type_name}"));

            let reflect_component = registration
                .data::<ReflectComponent>()
                .unwrap_or_else(|| panic!("Unable to reflect component for {type_name}"));

            reflect_component.insert(world, entity, &*reflect);
        }
    }

    drop(read_registry);
    world.insert_resource(type_registry);
}

/// All entities with this component will be removed after leaving [`InGame`] state
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(super) struct InGameOnly;

/// Event that indicates that game is about to be saved to the file name based on [`WorldName`].
#[derive(Default)]
pub(crate) struct GameSaved;

/// Event that indicates that game is about to be loaded from the file name based on [`WorldName`].
#[derive(Default)]
pub(crate) struct GameLoaded;

/// The name of the current world.
#[derive(Default, Deref)]
pub(crate) struct WorldName(pub(crate) String);

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use bevy::core::CorePlugin;

    use super::*;
    use crate::core::game_paths::GamePaths;

    #[test]
    fn world_cleanup() {
        let mut app = App::new();
        app.add_plugin(TestGameWorldPlugin);

        let child_entity = app.world.spawn().id();
        let ingame_entity = app
            .world
            .spawn()
            .insert(InGameOnly)
            .push_children(&[child_entity])
            .id();

        app.update();

        app.world.insert_resource(NextState(GameState::Menu));

        app.update();

        assert!(
            app.world.get_entity(ingame_entity).is_none(),
            "Ingame entity should be despawned after leaving ingame state"
        );
        assert!(
            app.world.get_entity(child_entity).is_none(),
            "Children of ingame entity should be despawned with its parent"
        );
        assert!(
            app.world.get_resource::<WorldName>().is_none(),
            "Would name resource should be removed"
        );
    }

    #[test]
    fn saving_and_loading() -> Result<()> {
        const WORLD_NAME: &str = "Test world";
        let mut app = App::new();
        app.init_resource::<GamePaths>()
            .insert_resource(WorldName(WORLD_NAME.to_string()))
            .add_plugin(CorePlugin)
            .add_plugin(TransformPlugin)
            .add_plugin(TestGameWorldPlugin);

        let game_paths = app.world.resource::<GamePaths>();
        let world_path = game_paths.world_path(WORLD_NAME);
        assert!(
            !world_path.exists(),
            "File {world_path:?} shouldn't exists after the plugin initialization"
        );

        const TRANSFORM: Transform = Transform::identity();
        let ingame_entity = app.world.spawn().insert(TRANSFORM).insert(InGameOnly).id();
        let other_entity = app.world.spawn().insert(Transform::identity()).id();

        let mut save_events = app.world.resource_mut::<Events<GameSaved>>();
        save_events.send_default();

        app.update();

        app.world.entity_mut(ingame_entity).despawn();
        app.world.entity_mut(other_entity).despawn();

        let mut save_events = app.world.resource_mut::<Events<GameLoaded>>();
        save_events.send_default();

        app.update();

        assert_eq!(
            *app.world.query::<&Transform>().single(&app.world),
            TRANSFORM,
            "Loaded transform should be equal to the saved"
        );

        fs::remove_file(&world_path)
            .with_context(|| format!("Unable to remove {world_path:?} after test"))
    }

    struct TestGameWorldPlugin;

    impl Plugin for TestGameWorldPlugin {
        fn build(&self, app: &mut App) {
            app.add_loopless_state(GameState::InGame)
                .add_plugin(GameWorldPlugin);
        }
    }
}
