pub(super) mod door;
pub mod placing_object;
pub(super) mod wall_mount;

use bevy::{
    asset::AssetPath,
    ecs::{entity::MapEntities, reflect::ReflectCommandExt},
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
    game_world::GameWorld,
    highlighting::OutlineHighlightingExt,
    lot::LotVertices,
    Layer,
};
use door::DoorPlugin;
use placing_object::{PlacingObject, PlacingObjectPlugin};
use wall_mount::WallMountPlugin;

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DoorPlugin, PlacingObjectPlugin, WallMountPlugin))
            .register_type::<ObjectPath>()
            .replicate::<ObjectPath>()
            .add_client_event::<ObjectBuy>(ChannelKind::Unordered)
            .add_mapped_client_event::<ObjectMove>(ChannelKind::Ordered)
            .add_mapped_client_event::<ObjectSell>(ChannelKind::Unordered)
            .add_server_event::<ObjectEventConfirmed>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .run_if(resource_exists::<GameWorld>)
                    .after(ClientSet::Receive),
            )
            .add_systems(
                SpawnScene,
                Self::init_children
                    .run_if(resource_exists::<GameWorld>)
                    .after(scene::scene_spawner_system),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::buy.before(ServerSet::StoreHierarchy),
                    Self::apply_movement,
                    Self::sell,
                )
                    .run_if(has_authority),
            );
    }
}

impl ObjectPlugin {
    fn init(
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

            let scene_path = metadata::gltf_asset(&object_path.0, "Scene0");
            debug!("spawning object {scene_path:?}");

            let scene_handle: Handle<Scene> = asset_server.load(scene_path);
            let mut entity = commands.entity(entity);
            entity.insert((
                scene_handle,
                Name::new(metadata.general.name.clone()),
                CursorHoverable,
                RigidBody::Kinematic,
                OutlineBundle::highlighting(),
                SpatialBundle::default(),
                CollisionLayers::new(Layer::Object, [Layer::Object, Layer::Wall]),
            ));

            for component in &metadata.components {
                entity.insert_reflect(component.clone_value());
            }
            if placing_object {
                for component in &metadata.place_components {
                    entity.insert_reflect(component.clone_value());
                }
            } else {
                for component in &metadata.spawn_components {
                    entity.insert_reflect(component.clone_value());
                }
            }
        }
    }

    fn init_children(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        objects: Query<Entity, With<ObjectPath>>,
        chidlren: Query<&Children>,
        child_meshes: Query<(&Transform, &Handle<Mesh>)>,
    ) {
        for object_entity in objects.iter_many(ready_events.read().map(|event| event.parent)) {
            let mut merged_mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_inserted_indices(Indices::U32(Vec::new()));

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

    fn buy(
        mut commands: Commands,
        mut buy_events: EventReader<FromClient<ObjectBuy>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
        cities: Query<(Entity, &Transform), With<City>>,
        lots: Query<(Entity, &LotVertices)>,
    ) {
        for FromClient { client_id, event } in buy_events.read().cloned() {
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
                .find(|(_, vertices)| vertices.contains_point(event.position.xz()))
                .map(|(lot_entity, _)| lot_entity)
                .unwrap_or(city_entity);

            commands.entity(parent_entity).with_children(|parent| {
                parent.spawn(ObjectBundle::new(
                    event.metadata_path,
                    event.position,
                    event.rotation,
                ));
            });
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: ObjectEventConfirmed,
            });
        }
    }

    fn apply_movement(
        mut move_events: EventReader<FromClient<ObjectMove>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
        mut objects: Query<(&mut Position, &mut Rotation)>,
    ) {
        for FromClient { client_id, event } in move_events.read().copied() {
            match objects.get_mut(event.entity) {
                Ok((mut position, mut rotation)) => {
                    **position = event.position;
                    **rotation = event.rotation;
                    confirm_events.send(ToClients {
                        mode: SendMode::Direct(client_id),
                        event: ObjectEventConfirmed,
                    });
                }
                Err(e) => error!("unable to apply object movement: {e}",),
            }
        }
    }

    fn sell(
        mut commands: Commands,
        mut sell_events: EventReader<FromClient<ObjectSell>>,
        mut confirm_events: EventWriter<ToClients<ObjectEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in sell_events.read().copied() {
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
    position: Position,
    rotation: Rotation,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl ObjectBundle {
    fn new(metadata_path: AssetPath<'static>, translation: Vec3, rotation: Quat) -> Self {
        Self {
            object_path: ObjectPath(metadata_path),
            position: Position(translation),
            rotation: Rotation(rotation),
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

/// Contains path to the object metadata file.
#[derive(Clone, Component, Debug, Default, Event, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(crate) struct ObjectPath(AssetPath<'static>);

#[derive(Clone, Deserialize, Event, Serialize)]
struct ObjectBuy {
    metadata_path: AssetPath<'static>,
    position: Vec3,
    rotation: Quat,
}

#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct ObjectMove {
    entity: Entity,
    position: Vec3,
    rotation: Quat,
}

impl MapEntities for ObjectMove {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.entity = entity_mapper.map_entity(self.entity);
    }
}

#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct ObjectSell(Entity);

impl MapEntities for ObjectSell {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

/// An event from server which indicates action confirmation.
#[derive(Deserialize, Event, Serialize, Default)]
struct ObjectEventConfirmed;
