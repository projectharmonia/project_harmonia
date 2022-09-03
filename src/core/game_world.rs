mod ignore_rules;

use std::{borrow::Cow, fs};

use anyhow::{Context, Result};
use bevy::{
    ecs::archetype::ArchetypeId,
    prelude::*,
    reflect::TypeRegistry,
    scene::{serde::SceneDeserializer, DynamicEntity},
};
use iyes_loopless::prelude::*;
use ron::Deserializer;
use serde::de::DeserializeSeed;

use super::{errors, game_paths::GamePaths};
use ignore_rules::IgnoreRules;

#[derive(SystemLabel)]
pub(crate) enum GameWorldSystem {
    Saving,
    Loading,
}

pub(super) struct GameWorldPlugin;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GameEntity>()
            .register_type::<Cow<'static, str>>() // https://github.com/bevyengine/bevy/issues/5597
            .init_resource::<IgnoreRules>()
            .add_event::<GameSaved>()
            .add_event::<GameLoaded>()
            .add_system(
                Self::saving_system
                    .chain(errors::log_err_system)
                    .run_on_event::<GameSaved>()
                    .label(GameWorldSystem::Saving),
            )
            .add_system(
                Self::loading_system
                    .chain(errors::log_err_system)
                    .run_on_event::<GameLoaded>()
                    .label(GameWorldSystem::Loading),
            );
    }
}

impl GameWorldPlugin {
    /// Saves world to disk with the name from [`GameWorld`] resource.
    fn saving_system(
        world: &World,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
        registry: Res<TypeRegistry>,
        ignore_rules: Res<IgnoreRules>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.world_name);

        fs::create_dir_all(&game_paths.worlds)
            .with_context(|| format!("Unable to create {world_path:?}"))?;

        let scene = save_to_scene(world, &*registry, &*ignore_rules);
        let bytes = scene
            .serialize_ron(&registry)
            .expect("Unable to serlialize world");

        fs::write(&world_path, bytes)
            .with_context(|| format!("Unable to save game to {world_path:?}"))
    }

    /// Loads world from disk with the name from [`GameWorld`] resource.
    fn loading_system(
        mut scene_spawner: ResMut<SceneSpawner>,
        mut scenes: ResMut<Assets<DynamicScene>>,
        game_world: Res<GameWorld>,
        game_paths: Res<GamePaths>,
        registry: Res<TypeRegistry>,
    ) -> Result<()> {
        let world_path = game_paths.world_path(&game_world.world_name);

        let bytes =
            fs::read(&world_path).with_context(|| format!("Unable to load {world_path:?}"))?;
        let mut deserializer = Deserializer::from_bytes(&bytes)
            .with_context(|| format!("Unable to parse {world_path:?}"))?;
        let scene_deserializer = SceneDeserializer {
            type_registry: &registry.read(),
        };
        let scene = scene_deserializer
            .deserialize(&mut deserializer)
            .with_context(|| format!("Unable to deserialize {world_path:?}"))?;

        scene_spawner.spawn_dynamic(scenes.add(scene));

        Ok(())
    }
}

/// Iterates over a world and serializes all components that implement [`Reflect`]
/// and not filtered using [`IgnoreRules`]
fn save_to_scene(
    world: &World,
    registry: &TypeRegistry,
    ignore_rules: &IgnoreRules,
) -> DynamicScene {
    let mut scene = DynamicScene::default();
    let registry = registry.read();
    for archetype in world
        .archetypes()
        .iter()
        .filter(|archetype| archetype.id() != ArchetypeId::EMPTY)
        .filter(|archetype| archetype.id() != ArchetypeId::RESOURCE)
        .filter(|archetype| archetype.id() != ArchetypeId::INVALID)
        .filter(|archetype| !ignore_rules.ignored_archetype(archetype))
    {
        let entities_offset = scene.entities.len();
        for entity in archetype.entities() {
            scene.entities.push(DynamicEntity {
                entity: entity.id(),
                components: Vec::new(),
            });
        }

        for reflect_component in archetype
            .components()
            .filter(|&component_id| !ignore_rules.ignored_component(archetype, component_id))
            .filter_map(|component_id| {
                // SAFETY: `component_id` retrieved from the world.
                unsafe { world.components().get_info_unchecked(component_id) }.type_id()
            })
            .filter_map(|type_id| registry.get(type_id))
            .filter_map(|registration| registration.data::<ReflectComponent>())
        {
            for (index, entity) in archetype.entities().iter().enumerate() {
                let reflect = reflect_component
                    .reflect(world, *entity)
                    .expect("Unable to reflect component");

                scene.entities[entities_offset + index]
                    .components
                    .push(reflect.clone_value());
            }
        }
    }

    scene
}

/// All entities with this component will be removed after leaving [`InGame`] state.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct GameEntity;

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
    use crate::core::city::{City, CityBundle};

    #[test]
    fn saving_and_loading() {
        const WORLD_NAME: &str = "Test world";
        let mut app = App::new();
        app.register_type::<Camera>()
            .register_type::<City>()
            .init_resource::<GamePaths>()
            .insert_resource(GameWorld::new(WORLD_NAME.to_string()))
            .add_plugin(CorePlugin)
            .add_plugin(AssetPlugin)
            .add_plugin(ScenePlugin)
            .add_plugin(TransformPlugin)
            .add_plugin(GameWorldPlugin);

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
            .insert_bundle(CityBundle::default())
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
}
