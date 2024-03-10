pub(crate) mod creating_wall;
pub(super) mod wall_mesh;

use std::mem;

use bevy::{
    ecs::entity::MapEntities,
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology, view::NoFrustumCulling},
};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use oxidized_navigation::NavMeshAffector;
use serde::{Deserialize, Serialize};

use super::{game_world::WorldName, Layer};
use creating_wall::{CreatingWall, CreatingWallPlugin};
use wall_mesh::WallMesh;

pub(super) struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CreatingWallPlugin)
            .register_type::<Wall>()
            .replicate::<Wall>()
            .add_mapped_client_event::<WallCreate>(ChannelKind::Unordered)
            .add_server_event::<WallCreateConfirmed>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::cleanup_connections,
                    Self::update_connections,
                    Self::update_meshes,
                    Self::create
                        .run_if(has_authority)
                        .before(ServerSet::StoreHierarchy),
                )
                    .chain()
                    .run_if(resource_exists::<WorldName>),
            );
    }
}

impl WallPlugin {
    fn init(
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        asset_server: Res<AssetServer>,
        walls: Query<(Entity, Has<CreatingWall>), Added<Wall>>,
    ) {
        for (entity, creating_wall) in &walls {
            let material = StandardMaterial {
                base_color_texture: Some(
                    asset_server.load("base/walls/brick/brick_base_color.png"),
                ),
                metallic_roughness_texture: Some(
                    asset_server.load("base/walls/brick/brick_roughnes_metalic.png"),
                ),
                normal_map_texture: Some(asset_server.load("base/walls/brick/brick_normal.png")),
                occlusion_texture: Some(asset_server.load("base/walls/brick/brick_occlusion.png")),
                perceptual_roughness: 0.0,
                reflectance: 0.0,
                ..Default::default()
            };
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<Vec3>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<Vec2>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_inserted_indices(Indices::U32(Vec::new()));

            let mut entity = commands.entity(entity);
            entity.insert((
                Name::new("Walls"),
                WallConnections::default(),
                Apertures::default(),
                Collider::default(),
                CollisionLayers::new(Layer::Wall, Layer::Object),
                NoFrustumCulling,
                PbrBundle {
                    material: materials.add(material),
                    mesh: meshes.add(mesh),
                    ..Default::default()
                },
            ));

            if !creating_wall {
                entity.insert(NavMeshAffector);
            }
        }
    }

    fn update_connections(
        mut walls: Query<(Entity, &Wall, &mut WallConnections)>,
        children: Query<&Children>,
        changed_walls: Query<(Entity, &Parent, &Wall), (Changed<Wall>, With<WallConnections>)>,
    ) {
        for (wall_entity, parent, &wall) in &changed_walls {
            // Take changed connections to avoid mutability issues.
            let (.., mut connections) = walls
                .get_mut(wall_entity)
                .expect("query is a subset of the changed query");
            let mut connections = mem::take(&mut *connections);

            // Cleanup old connections.
            for other_entity in connections.drain() {
                let (.., mut other_connections) = walls
                    .get_mut(other_entity)
                    .expect("connected wall should also have connections");
                if let Some((point_kind, index)) = other_connections.position(wall_entity) {
                    other_connections.remove(point_kind, index);
                }
            }

            // If wall have zero length, exclude it from connections.
            if wall.start != wall.end {
                // Scan all walls from this lot for possible connections.
                let mut iter = walls.iter_many_mut(children.get(**parent).unwrap());
                while let Some((other_entity, &other_wall, mut other_connections)) = iter
                    .fetch_next()
                    .filter(|&(entity, ..)| entity != wall_entity)
                {
                    if wall.start == other_wall.start {
                        connections.start.push(WallConnection {
                            wall_entity: other_entity,
                            point_kind: PointKind::Start,
                            wall: other_wall,
                        });
                        other_connections.start.push(WallConnection {
                            wall_entity,
                            point_kind: PointKind::Start,
                            wall,
                        });
                    } else if wall.start == other_wall.end {
                        connections.start.push(WallConnection {
                            wall_entity: other_entity,
                            point_kind: PointKind::End,
                            wall: other_wall,
                        });
                        other_connections.end.push(WallConnection {
                            wall_entity,
                            point_kind: PointKind::Start,
                            wall,
                        });
                    } else if wall.end == other_wall.end {
                        connections.end.push(WallConnection {
                            wall_entity: other_entity,
                            point_kind: PointKind::End,
                            wall: other_wall,
                        });
                        other_connections.end.push(WallConnection {
                            wall_entity,
                            point_kind: PointKind::End,
                            wall,
                        });
                    } else if wall.end == other_wall.start {
                        connections.end.push(WallConnection {
                            wall_entity: other_entity,
                            point_kind: PointKind::Start,
                            wall: other_wall,
                        });
                        other_connections.start.push(WallConnection {
                            wall_entity,
                            point_kind: PointKind::End,
                            wall,
                        });
                    }
                }
            }

            // Reinsert updated connections back.
            *walls.get_mut(wall_entity).unwrap().2 = connections;
        }
    }

    pub(super) fn update_meshes(
        mut meshes: ResMut<Assets<Mesh>>,
        mut changed_walls: Query<
            (
                &Handle<Mesh>,
                Ref<Wall>,
                &WallConnections,
                &mut Apertures,
                &mut Collider,
            ),
            Or<(Changed<WallConnections>, Changed<Apertures>)>,
        >,
    ) {
        for (mesh_handle, wall, connections, mut apertures, mut collider) in &mut changed_walls {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("wall handles should be valid");

            let mut wall_mesh = WallMesh::take(mesh);
            wall_mesh.generate(*wall, connections, &apertures);
            wall_mesh.apply(mesh);

            // Creating walls shouldn't affect navigation.
            if apertures.collision_outdated || wall.is_changed() || collider.is_added() {
                *collider = wall_mesh::generate_collider(*wall, &apertures);
                apertures.collision_outdated = false;
            }
        }
    }

    fn cleanup_connections(
        mut removed_walls: RemovedComponents<Wall>,
        mut walls: Query<&mut WallConnections>,
    ) {
        for entity in removed_walls.read() {
            for mut connections in &mut walls {
                if let Some((point_kind, index)) = connections.position(entity) {
                    connections.remove(point_kind, index);
                }
            }
        }
    }

    fn create(
        mut commands: Commands,
        mut create_events: EventReader<FromClient<WallCreate>>,
        mut confirm_events: EventWriter<ToClients<WallCreateConfirmed>>,
    ) {
        for FromClient { client_id, event } in create_events.read().copied() {
            // TODO: validate if wall can be spawned.
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: WallCreateConfirmed,
            });
            commands.entity(event.lot_entity).with_children(|parent| {
                parent.spawn(WallBundle::new(event.wall));
            });
        }
    }
}

#[derive(Bundle)]
struct WallBundle {
    wall: Wall,
    parent_sync: ParentSync,
    replication: Replication,
}

impl WallBundle {
    fn new(wall: Wall) -> Self {
        Self {
            wall,
            parent_sync: Default::default(),
            replication: Replication,
        }
    }
}

#[derive(Clone, Component, Copy, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(super) struct Wall {
    pub(super) start: Vec2,
    pub(super) end: Vec2,
}

impl Wall {
    /// Returns `true` if a point belongs to a wall.
    pub(super) fn contains(&self, point: Vec2) -> bool {
        let wall_disp = self.displacement();
        let point_disp = point - self.start;
        if wall_disp.perp_dot(point_disp).abs() > 0.1 {
            return false;
        }

        let dot = wall_disp.dot(point_disp);
        if dot < 0.0 {
            return false;
        }

        dot <= wall_disp.length_squared()
    }

    pub(super) fn closest_point(&self, point: Vec2) -> Vec2 {
        let wall_disp = self.displacement();
        let wall_dir = wall_disp.normalize();
        let point_dir = point - self.start;
        let dot = wall_dir.dot(point_dir);

        if dot <= 0.0 {
            self.start
        } else if dot >= wall_disp.length() {
            self.end
        } else {
            self.start + wall_dir * dot
        }
    }

    fn inverse(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
        }
    }

    pub(super) fn displacement(&self) -> Vec2 {
        self.end - self.start
    }
}

/// Dynamically updated component with precalculated connected entities for each wall point.
#[derive(Component, Default)]
pub(super) struct WallConnections {
    start: Vec<WallConnection>,
    end: Vec<WallConnection>,
}

impl WallConnections {
    fn drain(&mut self) -> impl Iterator<Item = Entity> + '_ {
        self.start
            .drain(..)
            .chain(self.end.drain(..))
            .map(|WallConnection { wall_entity, .. }| wall_entity)
    }

    /// Returns point kind and index to which it connected for an entity.
    ///
    /// Used for [`Self::remove`] later.
    /// It's two different functions to avoid triggering change detection if there is no such entity.
    fn position(&self, entity: Entity) -> Option<(PointKind, usize)> {
        if let Some(index) = self
            .start
            .iter()
            .position(|&WallConnection { wall_entity, .. }| wall_entity == entity)
        {
            Some((PointKind::Start, index))
        } else {
            self.end
                .iter()
                .position(|&WallConnection { wall_entity, .. }| wall_entity == entity)
                .map(|index| (PointKind::End, index))
        }
    }

    /// Removes connection by its index from specific point.
    fn remove(&mut self, point_kind: PointKind, index: usize) {
        match point_kind {
            PointKind::Start => self.start.remove(index),
            PointKind::End => self.end.remove(index),
        };
    }
}

struct WallConnection {
    wall_entity: Entity,
    point_kind: PointKind,
    wall: Wall,
}

#[derive(Clone, Copy, Debug)]
enum PointKind {
    Start,
    End,
}

/// Dynamically updated component with precalculated apertures for wall objects.
///
/// Apertures are sorted by distance to the wall starting point.
#[derive(Component, Default)]
pub(super) struct Apertures {
    apertures: Vec<Aperture>,
    pub(super) collision_outdated: bool,
}

impl Apertures {
    /// Returns iterator over all apertures.
    fn iter(&self) -> impl Iterator<Item = &Aperture> {
        self.apertures.iter()
    }

    /// Inserts a new aperture in sorted order.
    pub(super) fn insert(&mut self, aperture: Aperture) {
        let index = self
            .apertures
            .binary_search_by(|other| other.distance.total_cmp(&aperture.distance))
            .expect_err("apertures shouldn't have duplicates");

        if !aperture.placing_object && !aperture.hole {
            self.collision_outdated = true;
        }
        self.apertures.insert(index, aperture);
    }

    /// Returns index of an aperture on the corresponding object entity.
    pub(super) fn position(&self, entity: Entity) -> Option<usize> {
        self.iter()
            .position(|aperture| aperture.object_entity == entity)
    }

    /// Returns aperture by its index.
    pub(super) fn remove(&mut self, index: usize) -> Aperture {
        let aperture = self.apertures.remove(index);
        if !aperture.placing_object && !aperture.hole {
            self.collision_outdated = true;
        }
        aperture
    }
}

pub(super) struct Aperture {
    /// The entity that cut this aperture.
    pub(super) object_entity: Entity,

    /// Position of the aperture.
    pub(super) translation: Vec3,

    /// Distance to the beginning of the wall.
    ///
    /// Used for sorting in [`Apertures`].
    pub(super) distance: f32,

    /// Positions relative to the coordinate origin at which the aperture is cut in 2D space.
    ///
    /// Obtained from [`WallMount::Embed`](super::object::wall_mount::WallMount).
    pub(super) cutout: Vec<Vec2>,

    /// Indicates if the aperture is hole (like a window) or clipping (like a door or arch).
    pub(super) hole: bool,

    /// Indicates if the aperture caused by an object that has not yet been placed.
    pub(super) placing_object: bool,
}

/// Client event to request a wall creation.
#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct WallCreate {
    lot_entity: Entity,
    wall: Wall,
}

impl MapEntities for WallCreate {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.lot_entity = entity_mapper.map_entity(self.lot_entity);
    }
}

#[derive(Deserialize, Event, Serialize)]
struct WallCreateConfirmed;
