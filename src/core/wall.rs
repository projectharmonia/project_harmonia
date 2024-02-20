pub(crate) mod spawning_wall;
pub(super) mod wall_mesh;

use std::mem;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology, view::NoFrustumCulling},
};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use oxidized_navigation::NavMeshAffector;
use serde::{Deserialize, Serialize};

use super::{cursor_hover::CursorHoverable, game_world::WorldName, Layer};
use spawning_wall::{SpawningWall, SpawningWallPlugin};
use wall_mesh::WallMesh;

pub(super) struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SpawningWallPlugin)
            .register_type::<Wall>()
            .replicate::<Wall>()
            .add_mapped_client_event::<WallSpawn>(EventType::Unordered)
            .add_systems(
                PreUpdate,
                (
                    Self::init_system,
                    Self::spawn_system.run_if(resource_exists::<RenetServer>()),
                )
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::cleanup_system,
                    Self::connections_update_system,
                    Self::mesh_update_system,
                )
                    .chain()
                    .run_if(resource_exists::<WorldName>()),
            );
    }
}

impl WallPlugin {
    fn init_system(
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        asset_server: Res<AssetServer>,
        spawned_walls: Query<Entity, Added<Wall>>,
    ) {
        for entity in &spawned_walls {
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
            let mesh = Mesh::new(PrimitiveTopology::TriangleList)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<Vec3>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<Vec2>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_indices(Some(Indices::U32(Vec::new())));

            commands.entity(entity).insert((
                Name::new("Walls"),
                WallConnections::default(),
                Apertures::default(),
                Collider::default(),
                CollisionLayers::new([Layer::Wall], [Layer::Object]),
                CursorHoverable,
                NavMeshAffector,
                NoFrustumCulling,
                PbrBundle {
                    material: materials.add(material),
                    mesh: meshes.add(mesh),
                    ..Default::default()
                },
            ));
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut entity_map: ResMut<ClientEntityMap>,
        mut spawn_events: EventReader<FromClient<WallSpawn>>,
    ) {
        for FromClient { client_id, event } in spawn_events.read().copied() {
            commands.entity(event.lot_entity).with_children(|parent| {
                // TODO: validate if wall can be spawned.
                let server_entity = parent.spawn(WallBundle::new(event.wall)).id();
                entity_map.insert(
                    client_id,
                    ClientMapping {
                        client_entity: event.wall_entity,
                        server_entity,
                    },
                );
            });
        }
    }

    fn connections_update_system(
        mut walls: Query<(Entity, &Wall, &mut WallConnections)>,
        children: Query<&Children>,
        changed_walls: Query<(Entity, &Parent, &Wall), (Changed<Wall>, With<WallConnections>)>,
    ) {
        for (wall_entity, parent, &wall) in &changed_walls {
            // Take changed connections to avoid mutability issues.
            let mut connections =
                mem::take(&mut *walls.component_mut::<WallConnections>(wall_entity));

            // Cleanup old connections.
            for other_entity in connections.drain() {
                let mut other_connections = walls.component_mut::<WallConnections>(other_entity);
                if let Some((point, index)) = other_connections.position(wall_entity) {
                    other_connections.remove(point, index);
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
            *walls.component_mut::<WallConnections>(wall_entity) = connections;
        }
    }

    pub(super) fn mesh_update_system(
        mut meshes: ResMut<Assets<Mesh>>,
        mut changed_walls: Query<
            (
                &Handle<Mesh>,
                &Wall,
                &WallConnections,
                &Apertures,
                &mut Collider,
                Has<SpawningWall>,
            ),
            Or<(Changed<WallConnections>, Changed<Apertures>)>,
        >,
    ) {
        for (mesh_handle, &wall, connections, apertures, mut collider, spawning_wall) in
            &mut changed_walls
        {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("wall handles should be valid");

            let mut wall_mesh = WallMesh::take(mesh);
            wall_mesh.generate(wall, connections, apertures);
            wall_mesh.apply(mesh);

            // Spawning walls shouldn't affect navigation.
            if !spawning_wall {
                *collider = Collider::trimesh_from_mesh(mesh)
                    .expect("wall mesh should be in compatible format");
            }
        }
    }

    fn cleanup_system(
        mut removed_walls: RemovedComponents<Wall>,
        mut walls: Query<&mut WallConnections>,
    ) {
        for entity in removed_walls.read() {
            for mut connections in &mut walls {
                if let Some((point, index)) = connections.position(entity) {
                    connections.remove(point, index);
                }
            }
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
        let wall_dir = self.dir();
        let point_dir = point - self.start;
        if wall_dir.perp_dot(point_dir).abs() > 0.1 {
            return false;
        }

        let dot = wall_dir.dot(point_dir);
        if dot < 0.0 {
            return false;
        }

        dot <= wall_dir.length_squared()
    }

    fn inverse(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
        }
    }

    pub(super) fn dir(&self) -> Vec2 {
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

    /// Returns position and point kind to which it connected for an entity.
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
    fn remove(&mut self, kind: PointKind, index: usize) {
        match kind {
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

#[derive(Component, Default, Deref, DerefMut)]
pub(super) struct Apertures(Vec<Aperture>);

impl Apertures {
    pub(super) fn update_translation(&mut self, entity: Entity, translation: Vec3) {
        let aperture = self
            .iter_mut()
            .find(|aperture| aperture.object_entity == entity)
            .expect("object entity for update should exist");

        aperture.translation = translation;
    }

    pub(super) fn remove_existing(&mut self, entity: Entity) {
        let index = self
            .iter()
            .position(|aperture| aperture.object_entity == entity)
            .expect("object entity for removal should exist");

        self.remove(index);
    }
}

pub(super) struct Aperture {
    pub(super) object_entity: Entity,
    pub(super) translation: Vec3,
    pub(super) positions: Vec<Vec2>,
}

/// Client event to request a wall creation.
#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct WallSpawn {
    lot_entity: Entity,
    wall_entity: Entity,
    wall: Wall,
}

impl MapNetworkEntities for WallSpawn {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.lot_entity = mapper.map(self.lot_entity);
    }
}
