use std::fs;

use anyhow::{Context, Result};
use bevy::{prelude::*, scene::serde::SceneDeserializer};
use bevy_replicon::prelude::*;
use serde::de::DeserializeSeed;

use super::{error_report, game_paths::GamePaths, game_state::GameState};

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GameSave>()
            .add_event::<GameLoad>()
            .add_systems(
                SpawnScene,
                Self::load
                    .pipe(error_report::report)
                    .run_if(on_event::<GameLoad>())
                    .before(bevy::scene::scene_spawner_system),
            )
            .add_systems(
                PostUpdate,
                Self::save
                    .pipe(error_report::report)
                    .run_if(on_event::<GameSave>()),
            );
    }
}

impl GameWorldPlugin {
    /// Saves world to disk with the name from [`GameWorld`] resource.
    fn save(
        world: &World,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.name);

        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("unable to create {world_path:?}"))?;

        let mut scene = DynamicScene::default();
        bevy_replicon::scene::replicate_into(&mut scene, world);
        let bytes = scene
            .serialize_ron(&registry)
            .expect("game world should be serialized");

        fs::write(&world_path, bytes)
            .with_context(|| format!("unable to save game to {world_path:?}"))
    }

    /// Loads world from disk with the name from [`GameWorld`] resource.
    fn load(
        mut scene_spawner: ResMut<SceneSpawner>,
        mut scenes: ResMut<Assets<DynamicScene>>,
        mut game_state: ResMut<NextState<GameState>>,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.name);
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
        game_state.set(GameState::World);

        Ok(())
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
pub struct GameWorld {
    pub name: String,
}
