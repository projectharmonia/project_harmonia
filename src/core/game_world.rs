mod ignore_rules;

use std::{any, borrow::Cow, fs};

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

use super::{
    cli::{Cli, GameCommand},
    errors::log_err_system,
    game_paths::GamePaths,
    game_state::GameState,
};
use ignore_rules::IgnoreRules;

#[derive(SystemLabel)]
pub(crate) enum GameWorldSystem {
    Saving,
}

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GameEntity>()
            .register_type::<Cow<'static, str>>() // https://github.com/bevyengine/bevy/issues/5597
            .add_event::<GameSaved>()
            .add_event::<GameLoaded>()
            .add_startup_system(Self::load_from_cli_system.chain(log_err_system))
            .add_system(Self::set_state_system.run_if_resource_added::<GameWorld>())
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>())
            .add_system(
                Self::saving_system
                    .chain(log_err_system)
                    .run_if_resource_exists::<GameWorld>()
                    .run_on_event::<GameSaved>()
                    .label(GameWorldSystem::Saving),
            );

        {
            // To avoid ambiguity: https://github.com/IyesGames/iyes_loopless/issues/15
            use iyes_loopless::condition::IntoConditionalExclusiveSystem;
            app.add_system(
                Self::logged_loading_system
                    .run_on_event::<GameLoaded>()
                    .at_start(),
            );
        }
    }
}

impl GameWorldPlugin {
    fn load_from_cli_system(
        mut commands: Commands,
        mut load_events: ResMut<Events<GameLoaded>>,
        cli: Res<Cli>,
    ) -> Result<()> {
        if let Some(GameCommand::Play { world_name, .. }) = &cli.subcommand {
            commands.insert_resource(GameWorld::new(world_name.clone()));
            load_events.send_default();
            // Should be called to avoid other systems reacting on the event twice
            // See https://github.com/IyesGames/iyes_loopless/issues/31
            load_events.update();
        }

        Ok(())
    }

    /// Sets state to [`GameState::World`].
    fn set_state_system(mut commands: Commands) {
        commands.insert_resource(NextState(GameState::World));
    }

    /// Removes all game world entities and sets state to [`GameState::MainMenu`].
    fn cleanup_system(mut commands: Commands, game_entities: Query<Entity, With<GameEntity>>) {
        for entity in &game_entities {
            commands.entity(entity).despawn_recursive();
        }
        commands.insert_resource(NextState(GameState::MainMenu));
    }

    /// Saves world to disk with the name from [`GameWorld`] resource.
    fn saving_system(
        world: &World,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.world_name);

        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("Unable to create {world_path:?}"))?;

        let bytes = rmp_serde::to_vec(&serialize_game_world(world).values().collect::<Vec<_>>())
            .expect("Unable to serlialize world");

        fs::write(&world_path, bytes)
            .with_context(|| format!("Unable to save game to {world_path:?}"))
    }

    /// Loads world from disk with the name from [`GameWorld`] resource.
    fn loading_system(world: &mut World) -> Result<()> {
        let game_world = world.resource::<GameWorld>();
        let game_paths = world.resource::<GamePaths>();
        let world_path = game_paths.world_path(&game_world.world_name);

        let bytes = fs::read(&world_path)
            .with_context(|| format!("Unable to load world from {world_path:?}"))?;

        let components = rmp_serde::from_slice::<Vec<Vec<Vec<u8>>>>(&bytes)
            .context("Unable to deserialize game world")?;

        deserialize_game_world(world, components);

        Ok(())
    }

    /// Calls [`Self::loading_system`] with log errors.
    fn logged_loading_system(world: &mut World) {
        log_err_system(In(Self::loading_system(world)));
    }
}

/// Iterates over a world and serializes all components that implement [`Reflect`]
/// and not filtered using [`IgnoreRules`]
fn serialize_game_world(world: &World) -> HashMap<Entity, Vec<Vec<u8>>> {
    let ignore_rules = IgnoreRules::global(world);
    let mut components = HashMap::new();
    let type_registry = world.resource::<TypeRegistry>().read();
    for archetype in world
        .archetypes()
        .iter()
        .filter(|archetype| !ignore_rules.ignored_archetype(archetype))
        .filter(|archetype| archetype.id() != ArchetypeId::EMPTY)
        .filter(|archetype| archetype.id() != ArchetypeId::RESOURCE)
        .filter(|archetype| archetype.id() != ArchetypeId::INVALID)
    {
        for reflect_component in archetype
            .components()
            .filter(|&component_id| !ignore_rules.ignored_component(archetype, component_id))
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
        .unwrap_or_else(|| panic!("Unable to extract {}", any::type_name::<TypeRegistry>()));
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

/// All entities with this component will be removed after leaving [`InGame`] state.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(super) struct GameEntity;

/// Event that indicates that game is about to be saved to the file name based on [`GameWorld`] resource.
#[derive(Default)]
pub(crate) struct GameSaved;

/// Event that indicates that game is about to be loaded from the file name based on [`GameWorld`] resource.
#[derive(Default)]
pub(crate) struct GameLoaded;

/// The name of the current world.
#[derive(Default, Deref)]
pub(crate) struct GameWorld {
    pub(crate) world_name: String,
}

impl GameWorld {
    pub(crate) fn new(world_name: String) -> Self {
        Self { world_name }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use bevy::core::CorePlugin;

    use super::*;
    use crate::core::city::City;

    #[test]
    fn loading_from_cli() {
        const WORLD_NAME: &str = "World from CLI";
        let mut app = App::new();
        app.init_resource::<GamePaths>()
            .add_plugin(TestGameWorldPlugin);

        app.world.resource_mut::<Cli>().subcommand = Some(GameCommand::Play {
            world_name: WORLD_NAME.to_string(),
        });

        app.update();

        assert_eq!(
            app.world.resource::<Events<GameLoaded>>().len(),
            1,
            "{} event should be fired",
            any::type_name::<GameLoaded>()
        );
        assert_eq!(
            app.world.resource::<GameWorld>().world_name,
            WORLD_NAME,
            "Loaded world name should match the one from CLI"
        );
    }

    #[test]
    fn world_cleanup() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_plugin(TestGameWorldPlugin::default());

        let child_entity = app.world.spawn().id();
        let ingame_entity = app
            .world
            .spawn()
            .insert(GameEntity)
            .push_children(&[child_entity])
            .id();

        app.update();

        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::World,
            "After adding {} state should become {}",
            any::type_name::<GameWorld>(),
            GameState::World,
        );

        app.world.remove_resource::<GameWorld>();

        app.update();

        assert!(
            app.world.get_entity(ingame_entity).is_none(),
            "Ingame entity should be despawned after leaving ingame state"
        );
        assert!(
            app.world.get_entity(child_entity).is_none(),
            "Children of ingame entity should be despawned with its parent"
        );
        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::MainMenu,
            "After removing {} state should become {}",
            any::type_name::<GameWorld>(),
            GameState::MainMenu,
        );
    }

    #[test]
    fn saving_and_loading() -> Result<()> {
        const WORLD_NAME: &str = "Test world";
        let mut app = App::new();
        app.register_type::<Camera>()
            .register_type::<City>()
            .init_resource::<GamePaths>()
            .insert_resource(GameWorld::new(WORLD_NAME.to_string()))
            .add_plugin(CorePlugin)
            .add_plugin(TransformPlugin)
            .add_plugin(TestGameWorldPlugin::default());

        let game_paths = app.world.resource::<GamePaths>();
        let world_path = game_paths.world_path(WORLD_NAME);
        assert!(
            !world_path.exists(),
            "File {world_path:?} shouldn't exists after the plugin initialization"
        );

        const TRANSFORM: Transform = Transform::identity();
        let non_game_entity = app.world.spawn().insert(Transform::identity()).id();
        let game_world_entity = app
            .world
            .spawn()
            .insert_bundle(SpatialBundle {
                transform: TRANSFORM,
                ..Default::default()
            })
            .insert(Camera::default())
            .insert(GameEntity)
            .id();
        let non_game_city = app
            .world
            .spawn()
            .insert_bundle(SpatialBundle::default())
            .insert(City)
            .id();
        let city = app
            .world
            .spawn()
            .insert_bundle(SpatialBundle::default())
            .insert(City)
            .insert(GameEntity)
            .push_children(&[game_world_entity])
            .id();

        let mut save_events = app.world.resource_mut::<Events<GameSaved>>();
        save_events.send_default();

        app.update();

        app.world.entity_mut(non_game_entity).despawn();
        app.world.entity_mut(non_game_city).despawn();
        app.world.entity_mut(city).despawn_recursive();

        let mut save_events = app.world.resource_mut::<Events<GameLoaded>>();
        save_events.send_default();

        app.update();

        assert_eq!(
            *app.world.query::<&Transform>().single(&app.world),
            TRANSFORM,
            "Loaded transform should be equal to the saved"
        );
        assert!(
            app.world
                .query_filtered::<(), (With<City>, Without<Transform>)>()
                .get_single(&app.world)
                .is_ok(),
            "Loaded city shouldn't contain transform"
        );
        assert!(
            app.world.query::<&Camera>().get_single(&app.world).is_err(),
            "Camera component shouldn't be saved"
        );

        fs::remove_file(&world_path)
            .with_context(|| format!("Unable to remove {world_path:?} after test"))
    }

    #[derive(Default)]
    struct TestGameWorldPlugin;

    impl Plugin for TestGameWorldPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<Cli>().add_plugin(GameWorldPlugin);
        }
    }
}
