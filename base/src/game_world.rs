pub mod actor;
pub mod building;
pub mod city;
pub mod family;
pub mod hover;
pub mod object;
mod player_camera;

use std::fs;

use anyhow::{Context, Result};
use bevy::{
    prelude::*,
    scene::{ron, serde::SceneDeserializer},
};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use serde::de::DeserializeSeed;

use super::{core::GameState, game_paths::GamePaths, message::error_message};
use actor::ActorPlugin;
use building::BuildingPlugins;
use city::CityPlugin;
use family::FamilyPlugin;
use hover::HoverPlugin;
use object::ObjectPlugin;
use player_camera::PlayerCameraPlugin;

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ActorPlugin,
            BuildingPlugins,
            CityPlugin,
            HoverPlugin,
            FamilyPlugin,
            ObjectPlugin,
            PlayerCameraPlugin,
        ))
        .add_sub_state::<WorldState>()
        .enable_state_scoped_entities::<WorldState>()
        .add_event::<GameSave>()
        .add_event::<GameLoad>()
        .add_systems(Update, Self::start_game.run_if(client_just_connected))
        .add_systems(
            SpawnScene,
            Self::load
                .pipe(error_message)
                .run_if(on_event::<GameLoad>())
                .before(bevy::scene::scene_spawner_system),
        )
        .add_systems(
            PostUpdate,
            Self::save
                .pipe(error_message)
                .run_if(on_event::<GameSave>()),
        )
        .add_systems(OnExit(GameState::InGame), Self::cleanup);
    }
}

impl GameWorldPlugin {
    /// Saves world to disk with the name from [`GameWorld`] resource.
    fn save(
        world: &World,
        world_name: Res<WorldName>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&world_name.0);
        info!("saving world to {world_path:?}");

        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("unable to create {world_path:?}"))?;

        let mut scene = DynamicScene::default();
        let registry = registry.read();
        bevy_replicon::scene::replicate_into(&mut scene, world);
        let bytes = scene
            .serialize(&registry)
            .expect("game world should be serialized");

        fs::write(&world_path, bytes)
            .with_context(|| format!("unable to save game to {world_path:?}"))
    }

    /// Loads world from disk with the name from [`GameWorld`] resource.
    fn load(
        mut scene_spawner: ResMut<SceneSpawner>,
        mut scenes: ResMut<Assets<DynamicScene>>,
        mut game_state: ResMut<NextState<GameState>>,
        world_name: Res<WorldName>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&world_name.0);
        info!("loading world from {world_path:?}");

        let bytes =
            fs::read(&world_path).with_context(|| format!("unable to load {world_path:?}"))?;
        let mut deserializer = ron::Deserializer::from_bytes(&bytes)
            .with_context(|| format!("unable to parse {world_path:?}"))?;
        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.read(),
        };
        let mut scene = scene_deserializer
            .deserialize(&mut deserializer)
            .with_context(|| format!("unable to deserialize {world_path:?}"))?;

        // All saved entities should have `Replicated` component.
        for entity in &mut scene.entities {
            entity.components.push(Replicated.clone_value());
        }

        scene_spawner.spawn_dynamic(scenes.add(scene));
        game_state.set(GameState::InGame);

        Ok(())
    }

    fn start_game(mut commands: Commands, mut game_state: ResMut<NextState<GameState>>) {
        info!("joining replicated world");
        commands.insert_resource(WorldName::default());
        game_state.set(GameState::InGame);
    }

    fn cleanup(mut commands: Commands) {
        commands.remove_resource::<WorldName>();
    }
}

/// Event that indicates that game is about to be saved to the file name based on [`GameWorld`] resource.
#[derive(Default, Event)]
pub struct GameSave;

/// Event that indicates that game is about to be loaded from the file name based on [`GameWorld`] resource.
///
/// Sets game state to [`GameState::World`].
#[derive(Default, Event)]
pub struct GameLoad;

/// Contains metadata of the currently loaded world.
#[derive(Default, Resource)]
pub struct WorldName(pub String);

#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::InGame)]
pub enum WorldState {
    #[default]
    World,
    FamilyEditor,
    City,
    Family,
}

#[derive(PhysicsLayer)]
pub(super) enum Layer {
    Ground,
    Object,
    Wall,
}
