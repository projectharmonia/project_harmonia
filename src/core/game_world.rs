pub(crate) mod parent_sync;

use std::fs;

use anyhow::{Context, Result};
use bevy::{
    ecs::archetype::ArchetypeId,
    prelude::*,
    reflect::TypeRegistry,
    scene::{serde::SceneDeserializer, DynamicEntity},
};
use bevy_trait_query::imports::ComponentId;
use iyes_loopless::prelude::*;
use ron::Deserializer;
use serde::de::DeserializeSeed;

use super::{
    error_message,
    game_paths::GamePaths,
    game_state::GameState,
    network::replication::replication_rules::{Replication, ReplicationRules},
};
use parent_sync::ParentSyncPlugin;

#[derive(SystemLabel)]
pub(crate) enum GameWorldSystem {
    Saving,
    Loading,
}

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ParentSyncPlugin)
            .init_resource::<IgnoreSaving>()
            .add_event::<GameSave>()
            .add_event::<GameLoad>()
            .add_enter_system(GameState::MainMenu, Self::cleanup_system)
            .add_system(
                Self::saving_system
                    .pipe(error_message::err_message_system)
                    .run_on_event::<GameSave>()
                    .label(GameWorldSystem::Saving),
            )
            .add_system(
                Self::loading_system
                    .pipe(error_message::err_message_system)
                    .label(GameWorldSystem::Loading),
            );
    }
}

impl GameWorldPlugin {
    fn cleanup_system(mut commands: Commands) {
        commands.remove_resource::<GameWorld>();
    }

    /// Saves world to disk with the name from [`GameWorld`] resource.
    fn saving_system(
        world: &World,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
        replication_rules: Res<ReplicationRules>,
        ignore_saving: Res<IgnoreSaving>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.world_name);

        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("unable to create {world_path:?}"))?;

        let scene = save_to_scene(world, &registry, &replication_rules, &ignore_saving);
        let bytes = scene
            .serialize_ron(&registry)
            .expect("game world should be serialized");

        fs::write(&world_path, bytes)
            .with_context(|| format!("unable to save game to {world_path:?}"))
    }

    /// Loads world from disk with the name from [`GameWorld`] resource.
    fn loading_system(
        mut commands: Commands,
        mut load_events: ResMut<Events<GameLoad>>,
        mut scene_spawner: ResMut<SceneSpawner>,
        mut scenes: ResMut<Assets<DynamicScene>>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
    ) -> Result<()> {
        let Some(load_event) = load_events.drain().last() else {
            return Ok(());
        };

        let world_path = game_paths.world_path(&load_event.0);
        let bytes =
            fs::read(&world_path).with_context(|| format!("unable to load {world_path:?}"))?;
        let mut deserializer = Deserializer::from_bytes(&bytes)
            .with_context(|| format!("unable to parse {world_path:?}"))?;
        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.read(),
        };
        let mut scene = scene_deserializer
            .deserialize(&mut deserializer)
            .with_context(|| format!("unable to deserialize {world_path:?}"))?;

        // All saved entities should have `Replication` component.
        for entity in &mut scene.entities {
            entity.components.push(Replication.clone_value());
        }

        scene_spawner.spawn_dynamic(scenes.add(scene));

        commands.insert_resource(GameWorld::new(load_event.0));

        Ok(())
    }
}

/// Iterates over a world and serializes all components that implement [`Reflect`]
/// and not filtered with [`ReplicationRules`] or [`IgnoreSaving`].
fn save_to_scene(
    world: &World,
    registry: &TypeRegistry,
    replication_rules: &ReplicationRules,
    ignore_saving: &IgnoreSaving,
) -> DynamicScene {
    let mut scene = DynamicScene::default();
    let registry = registry.read();
    for archetype in world
        .archetypes()
        .iter()
        .filter(|archetype| archetype.id() != ArchetypeId::EMPTY)
        .filter(|archetype| archetype.id() != ArchetypeId::INVALID)
        .filter(|archetype| replication_rules.is_replicated_archetype(archetype))
    {
        let entities_offset = scene.entities.len();
        for archetype_entity in archetype.entities() {
            scene.entities.push(DynamicEntity {
                entity: archetype_entity.entity().index(),
                components: Vec::new(),
            });
        }

        for component_id in archetype.components().filter(|&component_id| {
            replication_rules.is_replicated_component(archetype, component_id)
                && !ignore_saving.contains(&component_id)
        }) {
            let component_info = unsafe { world.components().get_info_unchecked(component_id) };
            let type_name = component_info.name();
            let reflect_component = component_info
                .type_id()
                .and_then(|type_id| registry.get(type_id))
                .and_then(|registration| registration.data::<ReflectComponent>())
                .unwrap_or_else(|| panic!("non-ignored component {type_name} should be registered and have reflect(Component)"));

            for (index, archetype_entity) in archetype.entities().iter().enumerate() {
                let component = reflect_component
                    .reflect(world, archetype_entity.entity())
                    .unwrap_or_else(|| panic!("object should have {type_name}"));

                scene.entities[entities_offset + index]
                    .components
                    .push(component.clone_value());
            }
        }
    }

    scene
}

/// Event that indicates that game is about to be saved to the file name based on [`GameWorld`] resource.
#[derive(Default)]
pub(crate) struct GameSave;

/// Event that indicates that game is about to be loaded based on the specified name.
///
/// Creates [`GameWorld`] resource on loading.
pub(crate) struct GameLoad(pub(crate) String);

/// Contains meta-information about the currently loaded world.
#[derive(Default, Deref, Resource)]
pub(crate) struct GameWorld {
    pub(crate) world_name: String,
}

impl GameWorld {
    pub(crate) fn new(world_name: String) -> Self {
        Self { world_name }
    }
}

/// Contains component IDs that will be ignored on game world serialization.
#[derive(Default, Deref, DerefMut, Resource)]
struct IgnoreSaving(Vec<ComponentId>);

pub(super) trait AppIgnoreSavingExt {
    /// Ignore specified component for game world serialization.
    fn ignore_saving<T: Component>(&mut self) -> &mut Self;
}

impl AppIgnoreSavingExt for App {
    fn ignore_saving<T: Component>(&mut self) -> &mut Self {
        let component_id = self.world.init_component::<T>();
        self.world.resource_mut::<IgnoreSaving>().push(component_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetPlugin, core::CorePlugin, scene::ScenePlugin};

    use super::*;
    use crate::core::{
        city::City,
        network::replication::replication_rules::{
            AppReplicationExt, Replication, ReplicationRulesPlugin,
        },
    };

    #[test]
    fn saving_and_loading() {
        const WORLD_NAME: &str = "Test world";
        let mut app = App::new();
        app.add_loopless_state(GameState::World)
            .add_plugin(ReplicationRulesPlugin)
            .register_type::<Camera>()
            .replicate::<Transform>()
            .register_and_replicate::<City>()
            .not_replicate_if_present::<Transform, City>()
            .init_resource::<GamePaths>()
            .insert_resource(GameWorld::new(WORLD_NAME.to_string()))
            .add_plugin(CorePlugin::default())
            .add_plugin(AssetPlugin::default())
            .add_plugin(ScenePlugin)
            .add_plugin(TransformPlugin)
            .add_plugin(GameWorldPlugin);

        const TRANSFORM: Transform = Transform::IDENTITY;
        app.world.spawn(Transform::default()); // Non-reflected entity.
        app.world.spawn((TRANSFORM, Camera::default(), Replication)); // Reflected entity with ignored camera.
        app.world.spawn((Transform::default(), City, Replication)); // City entity with ignored transform.
        app.world.send_event_default::<GameSave>();

        app.update();

        app.insert_resource(NextState(GameState::MainMenu));
        app.world.clear_entities();

        app.update();

        assert!(
            app.world.get_resource::<GameWorld>().is_none(),
            "game world should be removed after entering main menu"
        );

        app.world
            .resource_mut::<Events<GameLoad>>()
            .send(GameLoad(WORLD_NAME.to_string()));

        app.update();
        app.update();

        assert_eq!(
            *app.world
                .query_filtered::<&Transform, With<Replication>>()
                .single(&app.world),
            TRANSFORM,
        );
        assert!(
            app.world
                .query_filtered::<(), (With<City>, With<Replication>, Without<Transform>)>()
                .get_single(&app.world)
                .is_ok(),
            "loaded city shouldn't contain transform"
        );
        assert!(
            app.world.query::<&Camera>().get_single(&app.world).is_err(),
            "camera component shouldn't be saved"
        );
    }
}
