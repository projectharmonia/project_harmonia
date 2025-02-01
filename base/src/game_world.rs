pub mod actor;
pub mod city;
pub mod commands_history;
pub mod family;
pub mod highlighting;
pub mod navigation;
pub mod object;
mod player_camera;
mod segment;

use std::fs;

use anyhow::{Context, Result};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    scene::{ron, serde::SceneDeserializer},
};
use bevy_replicon::prelude::*;
use serde::de::DeserializeSeed;

use super::{core::GameState, error_message::error_message, game_paths::GamePaths};
use actor::{Actor, ActorPlugin};
use city::CityPlugin;
use commands_history::CommandHistoryPlugin;
use family::FamilyPlugin;
use highlighting::HighlightingPlugin;
use navigation::NavigationPlugin;
use object::ObjectPlugin;
use player_camera::PlayerCameraPlugin;
use segment::SegmentPlugin;

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ActorPlugin,
            CityPlugin,
            SegmentPlugin,
            FamilyPlugin,
            HighlightingPlugin,
            NavigationPlugin,
            ObjectPlugin,
            PlayerCameraPlugin,
            CommandHistoryPlugin,
        ))
        .add_sub_state::<WorldState>()
        .enable_state_scoped_entities::<WorldState>()
        .add_observer(save.pipe(error_message))
        .add_observer(load.pipe(error_message))
        .add_systems(
            PreUpdate,
            start_game
                .after(ClientSet::Receive)
                .run_if(client_just_connected),
        )
        .add_systems(OnExit(GameState::InGame), cleanup);
    }
}

/// Saves world to disk with the name from [`WorldName`] resource.
fn save(
    _trigger: Trigger<GameSave>,
    world: &World,
    world_name: Res<WorldName>,
    game_paths: Res<GamePaths>,
    registry: Res<AppTypeRegistry>,
    actors: Query<Entity, With<Actor>>,
) -> Result<()> {
    let world_path = game_paths.world_path(&world_name.0);
    info!("saving world to {world_path:?}");

    fs::create_dir_all(&game_paths.worlds)
        .with_context(|| format!("unable to create {world_path:?}"))?;

    // Extract components that we don't replicate, but serialize.
    let mut scene = DynamicSceneBuilder::from_world(world)
        .deny_all()
        .allow_component::<Transform>()
        .extract_entities(actors.iter())
        .build();

    // Extract all replicated components that are reflected.
    let registry = registry.read();
    bevy_replicon::scene::replicate_into(&mut scene, world);
    let bytes = scene
        .serialize(&registry)
        .expect("game world should be serialized");

    fs::write(&world_path, bytes).with_context(|| format!("unable to save game to {world_path:?}"))
}

/// Loads world from disk with the name from [`WorldName`] resource.
fn load(
    _trigger: Trigger<GameLoad>,
    mut commands: Commands,
    mut scene_spawner: ResMut<SceneSpawner>,
    mut scenes: ResMut<Assets<DynamicScene>>,
    world_name: Res<WorldName>,
    game_paths: Res<GamePaths>,
    registry: Res<AppTypeRegistry>,
) -> Result<()> {
    let world_path = game_paths.world_path(&world_name.0);
    info!("loading world from {world_path:?}");

    let bytes = fs::read(&world_path).with_context(|| format!("unable to load {world_path:?}"))?;
    let mut deserializer = ron::Deserializer::from_bytes(&bytes)
        .with_context(|| format!("unable to parse {world_path:?}"))?;
    let scene_deserializer = SceneDeserializer {
        type_registry: &registry.read(),
    };
    let scene = scene_deserializer
        .deserialize(&mut deserializer)
        .with_context(|| format!("unable to deserialize {world_path:?}"))?;

    scene_spawner.spawn_dynamic(scenes.add(scene));
    commands.set_state(GameState::InGame);

    Ok(())
}

fn start_game(mut commands: Commands) {
    info!("joining replicated world");
    commands.insert_resource(WorldName::default());
    commands.set_state(GameState::InGame);
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<WorldName>();
}

/// Event that indicates that game is about to be saved to the file name based on [`WorldName`] resource.
#[derive(Default, Event)]
pub struct GameSave;

/// Event that indicates that game is about to be loaded from the file name based on [`WorldName`] resource.
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

#[derive(PhysicsLayer, Default)]
pub(super) enum Layer {
    #[default]
    None,
    Ground,
    Actor,
    Object,
    PlacingObject,
    Wall,
    PlacingWall,
    Road,
    PlacingRoad,
}
