pub(crate) mod placing_object;

use std::path::PathBuf;

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use bevy_scene_hook::SceneHook;
use iyes_loopless::prelude::*;
use placing_object::PlacingObjectPlugin;
use serde::{Deserialize, Serialize};
use tap::{TapFallible, TapOptional};

use super::{
    action::{self, Action},
    asset_metadata::{self, ObjectMetadata},
    city::{City, CityMode, CityPlugin},
    collision_groups::DollisGroups,
    cursor_hover::{CursorHover, Hoverable},
    family::FamilyMode,
    game_state::GameState,
    game_world::{parent_sync::ParentSync, AppIgnoreSavingExt, GameWorld},
    lot::LotVertices,
    network::{
        network_event::{
            client_event::{ClientEvent, ClientEventAppExt},
            server_event::{SendMode, ServerEvent, ServerEventAppExt},
        },
        replication::replication_rules::{AppReplicationExt, Replication},
    },
};

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(PlacingObjectPlugin)
            .register_and_replicate::<ObjectPath>()
            .register_and_replicate::<PickedPlayer>()
            .ignore_saving::<PickedPlayer>()
            .add_mapped_client_event::<ObjectPick>()
            .add_client_event::<ObjectPickCancel>()
            .add_client_event::<ObjectSpawn>()
            .add_client_event::<ObjectMove>()
            .add_client_event::<ObjectDespawn>()
            .add_server_event::<ObjectSpawnConfirmed>()
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system(
                Self::picking_system
                    .run_if(action::just_pressed(Action::Confirm))
                    .run_in_state(GameState::Family)
                    .run_in_state(FamilyMode::Building),
            )
            .add_system(
                Self::picking_system
                    .run_if(action::just_pressed(Action::Confirm))
                    .run_in_state(GameState::City)
                    .run_in_state(CityMode::Objects),
            )
            .add_system(Self::pick_confirmation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::pick_cancellation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::spawn_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::movement_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::despawn_system.run_unless_resource_exists::<RenetClient>());
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
                .unwrap_or_else(|| panic!("object metadata {:?} is invalid", object_path.0));

            let scene_path = asset_metadata::scene_path(&object_path.0);
            let scene_handle: Handle<Scene> = asset_server.load(&scene_path);

            commands.entity(entity).insert((
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
            ));
            debug!("spawned object {scene_path:?}");
        }
    }

    fn picking_system(
        mut pick_events: EventWriter<ObjectPick>,
        hovered_objects: Query<Entity, (With<ObjectPath>, With<CursorHover>)>,
    ) {
        for entity in hovered_objects.iter() {
            pick_events.send(ObjectPick(entity));
        }
    }

    fn pick_confirmation_system(
        mut commands: Commands,
        mut pick_events: EventReader<ClientEvent<ObjectPick>>,
        unpicked_objects: Query<(), (With<ObjectPath>, Without<PickedPlayer>)>,
    ) {
        for ClientEvent { client_id, event } in pick_events.iter().copied() {
            if unpicked_objects
                .get(event.0)
                .tap_err(|e| error!("entity {:?} can't be picked: {e}", event.0))
                .is_ok()
            {
                commands.entity(event.0).insert(PickedPlayer(client_id));
            }
        }
    }

    fn pick_cancellation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<ClientEvent<ObjectPickCancel>>,
        picked_objects: Query<(Entity, &PickedPlayer)>,
    ) {
        for client_id in cancel_events.iter().map(|event| event.client_id) {
            if let Some(entity) = picked_objects
                .iter()
                .find(|(_, picked)| picked.0 == client_id)
                .map(|(entity, _)| entity)
                .tap_none(|| error!("unable to find picked entity for client {client_id}"))
            {
                commands.entity(entity).remove::<PickedPlayer>();
            }
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: EventReader<ClientEvent<ObjectSpawn>>,
        mut confirm_events: EventWriter<ServerEvent<ObjectSpawnConfirmed>>,
        cities: Query<(Entity, &Transform), With<City>>,
        lots: Query<(Entity, &LotVertices)>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.iter().cloned() {
            const HALF_CITY_SIZE: f32 = CityPlugin::CITY_SIZE / 2.0;
            if event.position.y.abs() > HALF_CITY_SIZE {
                error!(
                    "received object spawn position {} with 'y' outside of city size",
                    event.position
                );
                continue;
            }

            let Some(city_entity) = cities
                .iter()
                .map(|(entity, transform)| (entity, transform.translation.x - event.position.x))
                .find(|(_, x)| x.abs() < HALF_CITY_SIZE)
                .map(|(entity, _)| entity)
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
            confirm_events.send(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: ObjectSpawnConfirmed,
            });
        }
    }

    fn movement_system(
        mut commands: Commands,
        mut move_events: EventReader<ClientEvent<ObjectMove>>,
        mut transforms: Query<(Entity, &mut Transform, &PickedPlayer)>,
    ) {
        for ClientEvent { client_id, event } in move_events.iter().copied() {
            if let Some((entity, mut transform)) = transforms
                .iter_mut()
                .find(|(.., picked)| picked.0 == client_id)
                .map(|(entity, transform, _)| (entity, transform))
                .tap_none(|| error!("no picked object to apply movement from client {client_id}"))
            {
                transform.translation = event.translation;
                transform.rotation = event.rotation;
                commands.entity(entity).remove::<PickedPlayer>();
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<ClientEvent<ObjectDespawn>>,
        mut picked_objects: Query<(Entity, &PickedPlayer)>,
    ) {
        for client_id in despawn_events.iter().map(|event| event.client_id) {
            if let Some(entity) = picked_objects
                .iter_mut()
                .find(|(_, picked)| picked.0 == client_id)
                .map(|(entity, _)| entity)
                .tap_none(|| error!("no picked object to apply despawn from client {client_id}"))
            {
                commands.entity(entity).despawn_recursive();
            }
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
pub(crate) struct ObjectPath(pub(crate) PathBuf);

/// Client id that picked the object.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct PickedPlayer(u64);

/// A client event for picking an entity.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ObjectPick(Entity);

impl MapEntities for ObjectPick {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// An event to cancel the currenly picked object for sender.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct ObjectPickCancel;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ObjectSpawn {
    metadata_path: PathBuf,
    position: Vec2,
    rotation: Quat,
}

/// An event to apply translation and rotation to the currently picked object.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ObjectMove {
    translation: Vec3,
    rotation: Quat,
}

/// An event to despawn the currently picked object.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ObjectDespawn;

/// An event from server which indicates spawn confirmation.
#[derive(Deserialize, Serialize, Debug, Default)]
struct ObjectSpawnConfirmed;
