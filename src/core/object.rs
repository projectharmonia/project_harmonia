pub(crate) mod cursor_object;

use std::path::PathBuf;

use bevy::{
    app::PluginGroupBuilder,
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_mod_outline::{Outline, OutlineBundle};
use bevy_mod_raycast::RayCastMesh;
use bevy_renet::renet::RenetServer;
use bevy_scene_hook::SceneHook;
use derive_more::From;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use tap::TapFallible;

use super::{
    asset_metadata,
    city::City,
    game_world::{parent_sync::ParentSync, GameEntity, GameWorld},
    network::{
        entity_serde,
        network_event::{
            client_event::{ClientEvent, ClientEventAppExt},
            server_event::{SendMode, ServerEvent, ServerEventAppExt, ServerSendBuffer},
        },
    },
    picking::Pickable,
};
use cursor_object::CursorObjectPlugin;

pub(super) struct ObjectPlugins;

impl PluginGroup for ObjectPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(CursorObjectPlugin).add(ObjectPlugin);
    }
}

struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ObjectPath>()
            .add_client_event::<ObjectSpawn>()
            .add_mapped_client_event::<ObjectMove>()
            .add_mapped_client_event::<ObjectDespawn>()
            .add_server_event::<ObjectConfirmed>()
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::spawn_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::movement_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::despawn_system.run_if_resource_exists::<RenetServer>());
    }
}

impl ObjectPlugin {
    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        spawned_objects: Query<(Entity, &ObjectPath), Added<ObjectPath>>,
    ) {
        for (entity, object_path) in &spawned_objects {
            let scene_path = asset_metadata::scene_path(&object_path.0);
            let scene_handle: Handle<Scene> = asset_server.load(&scene_path);

            commands
                .entity(entity)
                .insert(scene_handle)
                .insert(Pickable)
                .insert(GlobalTransform::default())
                .insert(SceneHook::new(|entity, commands| {
                    if entity.contains::<Handle<Mesh>>() {
                        commands
                            .insert_bundle(OutlineBundle {
                                outline: Outline {
                                    visible: false,
                                    colour: Color::rgba(1.0, 1.0, 1.0, 0.3),
                                    width: 2.0,
                                },
                                ..Default::default()
                            })
                            .insert(RayCastMesh::<Pickable>::default());
                    }
                }))
                .insert_bundle(VisibilityBundle::default());
            debug!("spawned object {scene_path:?}");
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: EventReader<ClientEvent<ObjectSpawn>>,
        mut confirm_buffer: ResMut<ServerSendBuffer<ObjectConfirmed>>,
        visible_cities: Query<Entity, (With<City>, With<Visibility>)>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.iter().cloned() {
            commands.spawn_bundle(ObjectBundle::new(
                event.metadata_path,
                event.translation,
                event.rotation,
                visible_cities.single(),
            ));
            confirm_buffer.push(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: ObjectConfirmed,
            });
        }
    }

    fn movement_system(
        mut move_events: EventReader<ClientEvent<ObjectMove>>,
        mut confirm_buffer: ResMut<ServerSendBuffer<ObjectConfirmed>>,
        mut transforms: Query<&mut Transform>,
    ) {
        for ClientEvent { client_id, event } in move_events.iter().copied() {
            if let Ok(mut transform) = transforms
                .get_mut(event.entity)
                .tap_err(|e| error!("unable to apply movement from client {client_id}: {e}"))
            {
                transform.translation = event.translation;
                transform.rotation = event.rotation;
                confirm_buffer.push(ServerEvent {
                    mode: SendMode::Direct(client_id),
                    event: ObjectConfirmed,
                });
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<ClientEvent<ObjectDespawn>>,
        mut confirm_buffer: ResMut<ServerSendBuffer<ObjectConfirmed>>,
    ) {
        for ClientEvent { client_id, event } in despawn_events.iter().copied() {
            commands.entity(event.0).despawn_recursive();
            confirm_buffer.push(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: ObjectConfirmed,
            });
        }
    }
}

#[derive(Bundle)]
pub(crate) struct ObjectBundle {
    object_path: ObjectPath,
    transform: Transform,
    parent_sync: ParentSync,
    game_entity: GameEntity,
}

impl ObjectBundle {
    fn new(metadata_path: PathBuf, translation: Vec3, rotation: Quat, parent: Entity) -> Self {
        Self {
            object_path: ObjectPath(
                metadata_path
                    .into_os_string()
                    .into_string()
                    .expect("Path should be a UTF-8 string"),
            ),
            transform: Transform::default()
                .with_translation(translation)
                .with_rotation(rotation),
            parent_sync: ParentSync(parent),
            game_entity: GameEntity,
        }
    }
}

/// Contains path to an object metadata file.
// TODO Bevy 0.9: Use `PathBuf`: https://github.com/bevyengine/bevy/issues/6166
#[derive(Clone, Component, Debug, Default, From, Reflect)]
#[reflect(Component)]
pub(crate) struct ObjectPath(String);

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ObjectSpawn {
    metadata_path: PathBuf,
    translation: Vec3,
    rotation: Quat,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ObjectMove {
    entity: Entity,
    translation: Vec3,
    rotation: Quat,
}

impl MapEntities for ObjectMove {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.entity = entity_map.get(self.entity)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ObjectDespawn(#[serde(with = "entity_serde")] pub(super) Entity);

impl MapEntities for ObjectDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// An event from server which indicates action confirmation.
#[derive(Deserialize, Serialize, Debug, Default)]
struct ObjectConfirmed;
