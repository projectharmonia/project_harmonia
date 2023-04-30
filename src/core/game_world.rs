use std::fs;

use anyhow::{Context, Result};
use bevy::{
    ecs::archetype::ArchetypeId,
    prelude::*,
    reflect::TypeRegistryArc,
    scene::{serde::SceneDeserializer, DynamicEntity},
};
use bevy_replicon::prelude::*;
use bevy_trait_query::imports::ComponentId;
use serde::de::DeserializeSeed;

use super::{error, game_paths::GamePaths, game_state::GameState};

pub(crate) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<WorldState>()
            .init_resource::<IgnoreSaving>()
            .add_event::<GameSave>()
            .add_event::<GameLoad>()
            .add_systems(
                (
                    Self::in_world_state_system.run_if(resource_added::<WorldName>()),
                    Self::no_world_state_system.run_if(resource_removed::<WorldName>()),
                )
                    .in_base_set(CoreSet::PostUpdate),
            )
            .add_systems((
                Self::loading_system
                    .pipe(error::report)
                    .run_if(on_event::<GameLoad>()),
                Self::saving_system
                    .pipe(error::report)
                    .run_if(on_event::<GameSave>()),
            ));
    }
}

impl GameWorldPlugin {
    fn no_world_state_system(mut world_state: ResMut<NextState<WorldState>>) {
        world_state.set(WorldState::NoWorld);
    }

    fn in_world_state_system(mut world_state: ResMut<NextState<WorldState>>) {
        world_state.set(WorldState::InWorld);
    }

    /// Saves world to disk with the name from [`GameWorld`] resource.
    pub(crate) fn saving_system(
        world: &World,
        world_name: Res<WorldName>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
        replication_rules: Res<ReplicationRules>,
        ignore_saving: Res<IgnoreSaving>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&world_name.0);

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
    pub(crate) fn loading_system(
        mut scene_spawner: ResMut<SceneSpawner>,
        mut scenes: ResMut<Assets<DynamicScene>>,
        mut game_state: ResMut<NextState<GameState>>,
        world_name: Res<WorldName>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&world_name.0);
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

        // All saved entities should have `Replication` component.
        for entity in &mut scene.entities {
            entity.components.push(Replication.clone_value());
        }

        scene_spawner.spawn_dynamic(scenes.add(scene));
        game_state.set(GameState::World);

        Ok(())
    }
}

/// Iterates over a world and serializes all components that implement [`Reflect`]
/// and not filtered with [`ReplicationRules`] or [`IgnoreSaving`].
fn save_to_scene(
    world: &World,
    registry: &TypeRegistryArc,
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
                entity: archetype_entity.entity(),
                components: Vec::new(),
            });
        }

        for component_id in archetype.components().filter(|&component_id| {
            replication_rules.is_replicated_component(archetype, component_id)
                && !ignore_saving.contains(&component_id)
        }) {
            // SAFE: `component_info` obtained from the world.
            let component_info = unsafe { world.components().get_info_unchecked(component_id) };
            let type_name = component_info.name();
            let type_id = component_info
                .type_id()
                .unwrap_or_else(|| panic!("{type_name} should have registered TypeId"));
            let registration = registry
                .get(type_id)
                .unwrap_or_else(|| panic!("{type_name} should be registered"));
            let reflect_component = registration
                .data::<ReflectComponent>()
                .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));

            for (index, archetype_entity) in archetype.entities().iter().enumerate() {
                let component = reflect_component
                    .reflect(world.entity(archetype_entity.entity()))
                    .unwrap_or_else(|| panic!("entity should have {type_name}"));

                scene.entities[entities_offset + index]
                    .components
                    .push(component.clone_value());
            }
        }
    }

    scene
}

/// Represents state of the [`WorldName`] resource.
///
/// To conveniently use with `in_set`.
#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub(super) enum WorldState {
    #[default]
    NoWorld,
    InWorld,
}

/// Event that indicates that game is about to be saved to the file name based on [`GameWorld`] resource.
#[derive(Default)]
pub(crate) struct GameSave;

/// Event that indicates that game is about to be loaded from the file name based on [`GameWorld`] resource.
///
/// Sets game state to [`GameState::World`].
#[derive(Default)]
pub(crate) struct GameLoad;

/// Contains name of the currently loaded world.
#[derive(Default, Resource)]
pub(crate) struct WorldName(pub(crate) String);

/// Contains component IDs that will be ignored on game world serialization.
#[derive(Default, Deref, DerefMut, Resource)]
pub(crate) struct IgnoreSaving(Vec<ComponentId>);

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
    use bevy::{asset::AssetPlugin, scene::ScenePlugin};

    use super::*;
    use crate::core::city::City;

    #[test]
    fn saving_and_loading() {
        const WORLD_NAME: &str = "Test world";
        let mut app = App::new();
        app.add_state::<GameState>()
            .add_plugins(
                ReplicationPlugins
                    .build()
                    .disable::<ClientPlugin>()
                    .disable::<ServerPlugin>(),
            )
            .register_type::<Camera>()
            .replicate::<Transform>()
            .replicate::<City>()
            .not_replicate_if_present::<Transform, City>()
            .init_resource::<GamePaths>()
            .insert_resource(WorldName(WORLD_NAME.to_string()))
            .add_plugin(TaskPoolPlugin::default())
            .add_plugin(TypeRegistrationPlugin)
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

        app.world.clear_entities();

        app.update();

        app.world.send_event_default::<GameLoad>();

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
