pub(crate) mod placing_object;
pub(super) mod wall_mount;

use bevy::{
    asset::AssetPath,
    ecs::reflect::ReflectCommandExt,
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::{InheritOutlineBundle, OutlineBundle};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    asset::metadata::{self, object_metadata::ObjectMetadata},
    city::{City, HALF_CITY_SIZE},
    cursor_hover::CursorHoverable,
    game_world::WorldName,
    highlighting::OutlineHighlightingExt,
    lot::LotVertices,
    Layer,
};
use placing_object::{PlacingObject, PlacingObjectPlugin};
use wall_mount::WallMountPlugin;

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PlacingObjectPlugin, WallMountPlugin))
            // TODO 0.13: Remove.
            .register_type::<AssetPath>()
            .register_type::<ObjectPath>()
            .replicate::<ObjectPath>()
            .add_client_event::<ObjectSpawn>(EventType::Unordered)
            .add_mapped_client_event::<ObjectMove>(EventType::Ordered)
            .add_mapped_client_event::<ObjectDespawn>(EventType::Unordered)
            .add_server_event::<ObjectEventConfirmed>(EventType::Unordered)
            .add_systems(
                PreUpdate,
                (
                    Self::init_system.run_if(resource_exists::<WorldName>()),
                    (
                        Self::spawn_system,
                        Self::movement_system,
                        Self::despawn_system,
                    )
                        .run_if(has_authority()),
                )
                    .after(ClientSet::Receive),
            )
            .add_systems(
                SpawnScene,
                Self::scene_init_system
                    .run_if(resource_exists::<WorldName>())
                    .after(scene::scene_spawner_system),
            );
    }
}

impl ObjectPlugin {
    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        spawned_objects: Query<(Entity, &ObjectPath, Has<PlacingObject>), Added<ObjectPath>>,
    ) {
        for (entity, object_path, placing_object) in &spawned_objects {
            let metadata_handle = asset_server.load(&object_path.0);
            let metadata = object_metadata
                .get(&metadata_handle)
                .unwrap_or_else(|| panic!("{object_path:?} should correspond to metadata"));

            let scene_path = metadata::scene_path(&asset_server, metadata_handle);
            debug!("spawning object {scene_path:?}");

            let scene_handle: Handle<Scene> = asset_server.load(scene_path);
            let mut entity = commands.entity(entity);
            entity.insert((
                scene_handle,
                Name::new(metadata.general.name.clone()),
                CursorHoverable,
                OutlineBundle::highlighting(),
                GlobalTransform::default(),
                VisibilityBundle::default(),
                CollisionLayers::from_bits(
                    Layer::Object.to_bits(),
                    Layer::Object.to_bits() | Layer::Wall.to_bits(),
                ),
            ));

            for component in &metadata.components {
                if placing_object && component.insert_on_placing()
                    || !placing_object && component.insert_on_spawning()
                {
                    entity.insert_reflect(component.clone_value());
                }
            }
        }
    }

    fn scene_init_system(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        objects: Query<Entity, With<ObjectPath>>,
        chidlren: Query<&Children>,
        child_meshes: Query<(&Transform, &Handle<Mesh>)>,
    ) {
        for object_entity in objects.iter_many(ready_events.read().map(|event| event.parent)) {
            let mut merged_mesh = Mesh::new(PrimitiveTopology::TriangleList)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_indices(Some(Indices::U32(Vec::new())));

            for child_entity in chidlren.iter_descendants(object_entity) {
                commands
                    .entity(child_entity)
                    .insert(InheritOutlineBundle::default());

                if let Ok((&transform, mesh_handle)) = child_meshes.get(child_entity) {
                    let mut mesh = meshes
                        .get(mesh_handle)
                        .cloned()
                        .expect("scene mesh should always be valid");
                    mesh.transform_by(transform);
                    merged_mesh.merge(mesh);
                }
            }

            let collider = Collider::convex_hull_from_mesh(&merged_mesh)
                .expect("object mesh should be in compatible format");

            commands.entity(object_entity).insert(collider);
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: EventReader<FromClient<ObjectSpawn>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
        cities: Query<(Entity, &Transform), With<City>>,
        lots: Query<(Entity, &LotVertices)>,
    ) {
        for FromClient { client_id, event } in spawn_events.read().cloned() {
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
                error!(
                    "unable to find a city for object spawn position {}",
                    event.position
                );
                continue;
            };

            // TODO: Add a check if user can spawn an object on the lot.
            let parent_entity = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(event.position))
                .map(|(lot_entity, _)| lot_entity)
                .unwrap_or(city_entity);

            commands.entity(parent_entity).with_children(|parent| {
                parent.spawn(ObjectBundle::new(
                    event.metadata_path,
                    Vec3::new(event.position.x, 0.0, event.position.y),
                    event.rotation,
                ));
            });
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
        for FromClient { client_id, event } in move_events.read().copied() {
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
        for FromClient { client_id, event } in despawn_events.read().copied() {
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
    fn new(metadata_path: AssetPath<'static>, translation: Vec3, rotation: Quat) -> Self {
        Self {
            object_path: ObjectPath(metadata_path),
            transform: Transform::default()
                .with_translation(translation)
                .with_rotation(rotation),
            parent_sync: Default::default(),
            replication: Replication,
        }
    }
}

/// Contains path to the object metadata file.
#[derive(Clone, Component, Debug, Default, Event, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(crate) struct ObjectPath(AssetPath<'static>);

#[reflect_trait]
pub(crate) trait ObjectComponent: Reflect {
    /// Returns `true` if component should be inserted on spawning.
    fn insert_on_spawning(&self) -> bool;

    /// Returns `true` if component should be inserted on placing.
    ///
    /// Can be used to avoid triggering systems that rely on this component when placing.
    fn insert_on_placing(&self) -> bool;
}

#[derive(Clone, Deserialize, Event, Serialize)]
struct ObjectSpawn {
    metadata_path: AssetPath<'static>,
    position: Vec2,
    rotation: Quat,
}

#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct ObjectMove {
    entity: Entity,
    translation: Vec3,
    rotation: Quat,
}

impl MapNetworkEntities for ObjectMove {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.entity = mapper.map(self.entity);
    }
}

#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct ObjectDespawn(Entity);

impl MapNetworkEntities for ObjectDespawn {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.0 = mapper.map(self.0);
    }
}

/// An event from server which indicates action confirmation.
#[derive(Deserialize, Event, Serialize, Default)]
struct ObjectEventConfirmed;
