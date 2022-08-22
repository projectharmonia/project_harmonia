use std::{borrow::Cow, fs};

use anyhow::{Context, Result};
use bevy::{prelude::*, reflect::TypeRegistry, scene::serde::SceneDeserializer};
use iyes_loopless::prelude::*;
use iyes_scene_tools::SceneBuilder;
use ron::Deserializer;
use serde::de::DeserializeSeed;

use super::{city::City, cli::Cli, errors::log_err_system, game_paths::GamePaths};

#[derive(SystemLabel)]
pub(crate) enum GameWorldSystem {
    Loading,
}

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GameEntity>()
            .register_type::<Cow<'static, str>>() // https://github.com/bevyengine/bevy/issues/5597
            .add_event::<GameSaved>()
            .add_event::<GameLoaded>()
            .add_startup_system(Self::load_from_cli_system.chain(log_err_system))
            .add_system(
                Self::loading_system
                    .chain(log_err_system)
                    .run_on_event::<GameLoaded>()
                    .label(GameWorldSystem::Loading),
            );

        {
            // To avoid ambiguity: https://github.com/IyesGames/iyes_loopless/issues/15
            use iyes_loopless::condition::IntoConditionalExclusiveSystem;
            app.add_system(
                Self::logged_saving_system
                    .run_on_event::<GameSaved>()
                    .before_commands(),
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
        if let Some(world_name) = cli.world_name() {
            commands.insert_resource(GameWorld::new(world_name.clone()));
            load_events.send_default();
            // Should be called to avoid other systems reacting on the event twice
            // See https://github.com/IyesGames/iyes_loopless/issues/31
            load_events.update();
        }

        Ok(())
    }

    /// Saves world to disk with the name from [`GameWorld`] resource.
    fn saving_system(world: &mut World) -> Result<()> {
        let game_world = world.resource::<GameWorld>();
        let game_paths = world.resource::<GamePaths>();
        let world_path = game_paths.world_path(&game_world.world_name);
        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("Unable to create {:?}", game_paths.worlds))?;

        let mut builder = SceneBuilder::new(world);
        builder.add_from_query_filter::<With<GameEntity>>();
        builder.add_with_components::<(&City, &Name), ()>();
        builder
            .ignore_components::<(&Camera, &GlobalTransform, &Visibility, &ComputedVisibility)>();
        builder
            .export_to_file(&world_path)
            .map(|_| ())
            .with_context(|| format!("Unable to save world to {world_path:?}"))
    }

    /// Loads world from disk with the name from [`GameWorld`] resource.
    fn loading_system(
        mut scene_spawner: ResMut<SceneSpawner>,
        mut scenes: ResMut<Assets<DynamicScene>>,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
        type_registry: Res<TypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.world_name);

        let bytes =
            fs::read(&world_path).with_context(|| format!("Unable to load {world_path:?}"))?;
        let mut deserializer = Deserializer::from_bytes(&bytes)
            .with_context(|| format!("Unable to parse {world_path:?}"))?;
        let scene_deserializer = SceneDeserializer {
            type_registry: &type_registry.read(),
        };
        let scene = scene_deserializer
            .deserialize(&mut deserializer)
            .with_context(|| format!("Unable to deserialize {world_path:?}"))?;

        scene_spawner.spawn_dynamic(scenes.add(scene));

        Ok(())
    }

    /// Calls [`Self::loading_system`] with error logging.
    fn logged_saving_system(world: &mut World) {
        log_err_system(In(Self::saving_system(world)));
    }
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
    use bevy::{asset::AssetPlugin, core::CorePlugin, scene::ScenePlugin};

    use super::*;
    use crate::core::{
        city::{City, CityBundle},
        cli::GameCommand,
    };

    #[test]
    fn loading_from_cli() {
        const WORLD_NAME: &str = "World from CLI";
        let mut app = App::new();
        app.add_plugin(TestGameWorldPlugin);

        app.world.resource_mut::<Cli>().subcommand = Some(GameCommand::Play {
            world_name: WORLD_NAME.to_string(),
            city: None,
        });

        app.update();

        assert_eq!(app.world.resource::<Events<GameLoaded>>().len(), 1);
        assert_eq!(app.world.resource::<GameWorld>().world_name, WORLD_NAME);
    }

    #[test]
    fn saving_and_loading() {
        const WORLD_NAME: &str = "Test world";
        let mut app = App::new();
        app.register_type::<Camera>()
            .register_type::<City>()
            .insert_resource(GameWorld::new(WORLD_NAME.to_string()))
            .add_plugin(CorePlugin)
            .add_plugin(TransformPlugin)
            .add_plugin(TestGameWorldPlugin);

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
        let city = app
            .world
            .spawn()
            .insert_bundle(SpatialBundle::default())
            .insert_bundle(CityBundle::default())
            .push_children(&[game_world_entity])
            .id();

        let mut save_events = app.world.resource_mut::<Events<GameSaved>>();
        save_events.send_default();

        app.update();

        app.world.entity_mut(non_game_entity).despawn();
        app.world.entity_mut(city).despawn_recursive();

        let mut save_events = app.world.resource_mut::<Events<GameLoaded>>();
        save_events.send_default();

        app.update();
        app.update();

        assert_eq!(
            *app.world.query::<&Transform>().single(&app.world),
            TRANSFORM,
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
    }

    struct TestGameWorldPlugin;

    impl Plugin for TestGameWorldPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<Cli>()
                .init_resource::<GamePaths>()
                .add_plugin(AssetPlugin)
                .add_plugin(ScenePlugin)
                .add_plugin(GameWorldPlugin);
        }
    }
}
