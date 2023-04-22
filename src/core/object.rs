pub(super) mod mirror;
pub(crate) mod placing_object;

use std::path::PathBuf;

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_scene_hook::SceneHook;
use serde::{Deserialize, Serialize};

use super::{
    asset_metadata::{self, ObjectMetadata},
    city::{City, HALF_CITY_SIZE},
    collision_groups::LifescapeGroupsExt,
    component_commands::ComponentCommandsExt,
    cursor_hover::Hoverable,
    game_world::{parent_sync::ParentSync, WorldState},
    lot::LotVertices,
};
use mirror::MirrorPlugin;
use placing_object::PlacingObjectPlugin;

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(PlacingObjectPlugin)
            .add_plugin(MirrorPlugin)
            .replicate::<ObjectPath>()
            .add_client_event::<ObjectSpawn>()
            .add_mapped_client_event::<ObjectMove>()
            .add_mapped_client_event::<ObjectDespawn>()
            .add_server_event::<ObjectEventConfirmed>()
            .add_system(Self::init_system.in_set(OnUpdate(WorldState::InWorld)))
            .add_systems(
                (
                    Self::spawn_system,
                    Self::movement_system,
                    Self::despawn_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl ObjectPlugin {
    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        spawned_objects: Query<(Entity, &ObjectPath), Added<ObjectPath>>,
    ) {
        for (entity, object_path) in &spawned_objects {
            let metadata_handle = asset_server.load(&*object_path.0);
            let object_metadata = object_metadata
                .get(&metadata_handle)
                .unwrap_or_else(|| panic!("{object_path:?} should correspond to metadata"));

            let scene_path = asset_metadata::scene_path(&object_path.0);
            debug!("spawning object {scene_path:?}");

            let scene_handle: Handle<Scene> = asset_server.load(scene_path);
            commands
                .entity(entity)
                .insert((
                    scene_handle,
                    Name::new(object_metadata.general.name.clone()),
                    Hoverable,
                    AsyncSceneCollider::default(),
                    GlobalTransform::default(),
                    VisibilityBundle::default(),
                    SceneHook::new(|entity, commands| {
                        if entity.contains::<Handle<Mesh>>() {
                            commands.insert((
                                CollisionGroups::new(Group::OBJECT, Group::ALL),
                                OutlineBundle {
                                    outline: OutlineVolume {
                                        visible: false,
                                        colour: Color::rgba(1.0, 1.0, 1.0, 0.3),
                                        width: 2.0,
                                    },
                                    ..Default::default()
                                },
                            ));
                        }
                    }),
                ))
                .insert_reflect(
                    object_metadata
                        .components
                        .iter()
                        .map(|component| component.clone_value())
                        .collect::<Vec<_>>(),
                );
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: EventReader<FromClient<ObjectSpawn>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
        cities: Query<(Entity, &Transform), With<City>>,
        lots: Query<(Entity, &LotVertices)>,
    ) {
        for FromClient { client_id, event } in spawn_events.iter().cloned() {
            if event.position.y.abs() > HALF_CITY_SIZE {
                error!(
                    "received object spawn position {} with 'y' outside of city size",
                    event.position
                );
                continue;
            }

            let Some((city_entity, _)) = cities
                .iter()
                .map(|(entity, transform)| (entity, transform.translation.x - event.position.x))
                .find(|(_, x)| x.abs() < HALF_CITY_SIZE)
            else {
                error!("unable to find a city for object spawn position {}", event.position);
                continue;
            };

            // TODO: Add a check if user can spawn an object on the lot.
            let parent_entity = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(event.position))
                .map(|(lot_entity, _)| lot_entity)
                .unwrap_or(city_entity);

            commands.spawn(ObjectBundle::new(
                event.metadata_path,
                Vec3::new(event.position.x, 0.0, event.position.y),
                event.rotation,
                parent_entity,
            ));
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: ObjectEventConfirmed,
            });
        }
    }

    fn movement_system(
        mut move_events: EventReader<FromClient<ObjectMove>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
        mut transforms: Query<&mut Transform>,
    ) {
        for FromClient { client_id, event } in move_events.iter().copied() {
            match transforms.get_mut(event.entity) {
                Ok(mut transform) => {
                    transform.translation = event.translation;
                    transform.rotation = event.rotation;
                    confirm_events.send(ToClients {
                        mode: SendMode::Direct(client_id),
                        event: ObjectEventConfirmed,
                    });
                }
                Err(e) => error!("unable to apply object movement: {e}",),
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<FromClient<ObjectDespawn>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in despawn_events.iter().copied() {
            commands.entity(event.0).despawn_recursive();
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: ObjectEventConfirmed,
            });
        }
    }
}

#[derive(Bundle)]
struct ObjectBundle {
    object_path: ObjectPath,
    transform: Transform,
    parent_sync: ParentSync,
    replication: Replication,
}

impl ObjectBundle {
    fn new(metadata_path: PathBuf, translation: Vec3, rotation: Quat, parent: Entity) -> Self {
        Self {
            object_path: ObjectPath(metadata_path),
            transform: Transform::default()
                .with_translation(translation)
                .with_rotation(rotation),
            parent_sync: ParentSync(parent),
            replication: Replication,
        }
    }
}

/// Contains path to the object metadata file.
#[derive(Clone, Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct ObjectPath(PathBuf);

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ObjectSpawn {
    metadata_path: PathBuf,
    position: Vec2,
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
struct ObjectDespawn(Entity);

impl MapEntities for ObjectDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// An event from server which indicates action confirmation.
#[derive(Deserialize, Serialize, Debug, Default)]
struct ObjectEventConfirmed;
