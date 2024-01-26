pub(crate) mod creating_wall;

use std::f32::consts::PI;

use bevy::{
    ecs::query::Has,
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
};
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use itertools::{Itertools, MinMaxResult};
use oxidized_navigation::NavMeshAffector;
use serde::{Deserialize, Serialize};

use super::{collision_groups::HarmoniaGroupsExt, game_world::WorldName};
use creating_wall::{CreatingWall, CreatingWallPlugin};

pub(super) struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CreatingWallPlugin)
            .register_type::<WallObject>()
            .register_type::<(Vec2, Vec2)>()
            .register_type::<Vec<(Vec2, Vec2)>>()
            .register_type::<WallEdges>()
            .replicate::<WallEdges>()
            .add_mapped_client_event::<WallCreate>(EventType::Unordered)
            .add_server_event::<WallEventConfirmed>(EventType::Unordered)
            .add_systems(
                PreUpdate,
                Self::mesh_update_system
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(PostUpdate, Self::init_system)
            .add_systems(Update, Self::wall_creation_system.run_if(has_authority()));
    }
}

impl WallPlugin {
    fn init_system(
        mut commands: Commands,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        spawned_walls: Query<(Entity, Has<CreatingWall>), Added<WallEdges>>,
    ) {
        for (entity, creating_wall) in &spawned_walls {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<Vec3>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_indices(Some(Indices::U32(Vec::new())));

            let mut entity = commands.entity(entity);
            entity.insert((
                Name::new("Walls"),
                CollisionGroups::new(Group::WALL, Group::ALL),
                NoFrustumCulling,
                PbrBundle {
                    material: materials.add(StandardMaterial::default()),
                    mesh: meshes.add(mesh),
                    ..Default::default()
                },
            ));

            // Creating walls shouldn't affect navigation.
            if !creating_wall {
                entity.insert(NavMeshAffector);
            }
        }
    }

    fn wall_creation_system(
        mut commands: Commands,
        mut create_events: EventReader<FromClient<WallCreate>>,
        mut confirm_events: EventWriter<ToClients<WallEventConfirmed>>,
        children: Query<&Children>,
        mut walls: Query<&mut WallEdges, Without<CreatingWall>>,
    ) {
        for FromClient { client_id, event } in create_events.read().copied() {
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: WallEventConfirmed,
            });
            if let Ok(children) = children.get(event.lot_entity) {
                if let Some(mut edges) = walls.iter_many_mut(children.iter()).fetch_next() {
                    edges.push(event.edge);
                    return;
                }
            }

            // No wall entity found, create a new one
            commands.entity(event.lot_entity).with_children(|parent| {
                parent.spawn(WallBundle::new(vec![event.edge]));
            });
        }
    }

    fn mesh_update_system(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(Entity, &WallEdges, &Handle<Mesh>), Changed<WallEdges>>,
    ) {
        for (entity, edges, mesh_handle) in &walls {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("wall handles should be valid");

            // Remove attributes to avoid mutability issues.
            let Some(VertexAttributeValues::Float32x3(mut positions)) =
                mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("all walls should be initialized with position attribute");
            };
            let Some(VertexAttributeValues::Float32x3(mut normals)) =
                mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
            else {
                panic!("all walls should be initialized with normal attribute");
            };
            let Some(Indices::U32(indices)) = mesh.indices_mut() else {
                panic!("all walls should have U32 indices");
            };

            positions.clear();
            normals.clear();
            indices.clear();

            for &(a, b) in edges.iter() {
                let last_index: u32 = positions
                    .len()
                    .try_into()
                    .expect("vertex index should fit u32");

                let width = width_vec(a, b);
                let a_edges = minmax_angles(a, b, edges);
                let (left_a, right_a) = offset_points(a, b, a_edges, width);

                let b_edges = minmax_angles(b, a, edges);
                let (right_b, left_b) = offset_points(b, a, b_edges, -width);

                // Top
                const HEIGHT: f32 = 2.8;
                positions.push([left_a.x, HEIGHT, left_a.y]);
                positions.push([right_a.x, HEIGHT, right_a.y]);
                positions.push([right_b.x, HEIGHT, right_b.y]);
                positions.push([left_b.x, HEIGHT, left_b.y]);
                normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
                indices.push(last_index);
                indices.push(last_index + 3);
                indices.push(last_index + 1);
                indices.push(last_index + 1);
                indices.push(last_index + 3);
                indices.push(last_index + 2);

                // Right
                positions.push([right_a.x, 0.0, right_a.y]);
                positions.push([right_b.x, 0.0, right_b.y]);
                positions.push([right_b.x, HEIGHT, right_b.y]);
                positions.push([right_a.x, HEIGHT, right_a.y]);
                normals.extend_from_slice(&[[-width.x, 0.0, -width.y]; 4]);
                indices.push(last_index + 4);
                indices.push(last_index + 7);
                indices.push(last_index + 5);
                indices.push(last_index + 5);
                indices.push(last_index + 7);
                indices.push(last_index + 6);

                // Left
                positions.push([left_a.x, 0.0, left_a.y]);
                positions.push([left_b.x, 0.0, left_b.y]);
                positions.push([left_b.x, HEIGHT, left_b.y]);
                positions.push([left_a.x, HEIGHT, left_a.y]);
                normals.extend_from_slice(&[[width.x, 0.0, width.y]; 4]);
                indices.push(last_index + 8);
                indices.push(last_index + 9);
                indices.push(last_index + 11);
                indices.push(last_index + 9);
                indices.push(last_index + 10);
                indices.push(last_index + 11);

                match a_edges {
                    MinMaxResult::OneElement(_) => (),
                    MinMaxResult::NoElements => {
                        let normal = a - b;

                        // Front
                        positions.push([left_a.x, 0.0, left_a.y]);
                        positions.push([left_a.x, HEIGHT, left_a.y]);
                        positions.push([right_a.x, HEIGHT, right_a.y]);
                        positions.push([right_a.x, 0.0, right_a.y]);
                        normals.extend_from_slice(&[[normal.x, 0.0, normal.y]; 4]);
                        indices.push(last_index + 12);
                        indices.push(last_index + 13);
                        indices.push(last_index + 15);
                        indices.push(last_index + 13);
                        indices.push(last_index + 14);
                        indices.push(last_index + 15);
                    }
                    MinMaxResult::MinMax(_, _) => {
                        let a_index: u32 = positions
                            .len()
                            .try_into()
                            .expect("vertex a index should fit u32");

                        // Inside triangle to fill the gap between 3+ walls.
                        positions.push([a.x, HEIGHT, a.y]);
                        normals.push([0.0, 1.0, 0.0]);
                        indices.push(last_index + 1);
                        indices.push(a_index);
                        indices.push(last_index);
                    }
                }

                match b_edges {
                    MinMaxResult::OneElement(_) => (),
                    MinMaxResult::NoElements => {
                        let normal = b - a;
                        let back_index: u32 = positions
                            .len()
                            .try_into()
                            .expect("vertex back index should fit u32");

                        // Back
                        positions.push([left_b.x, 0.0, left_b.y]);
                        positions.push([left_b.x, HEIGHT, left_b.y]);
                        positions.push([right_b.x, HEIGHT, right_b.y]);
                        positions.push([right_b.x, 0.0, right_b.y]);
                        normals.extend_from_slice(&[[normal.x, 0.0, normal.y]; 4]);
                        indices.push(back_index);
                        indices.push(back_index + 3);
                        indices.push(back_index + 1);
                        indices.push(back_index + 1);
                        indices.push(back_index + 3);
                        indices.push(back_index + 2);
                    }
                    MinMaxResult::MinMax(_, _) => {
                        let b_index: u32 = positions
                            .len()
                            .try_into()
                            .expect("vertex b index should fit u32");

                        // Inside triangle to fill the gap between 3+ walls.
                        positions.push([b.x, HEIGHT, b.y]);
                        normals.push([0.0, 1.0, 0.0]);
                        indices.push(last_index + 3);
                        indices.push(b_index);
                        indices.push(last_index + 2);
                    }
                }
            }

            // Reinsert removed attributes back.
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

            let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh)
                .expect("wall mesh should be in compatible format");
            commands.entity(entity).insert(collider);
        }
    }
}

const WIDTH: f32 = 0.15;
pub(super) const HALF_WIDTH: f32 = WIDTH / 2.0;

/// Calculates the wall thickness vector that faces to the left relative to the wall vector.
fn width_vec(start: Vec2, end: Vec2) -> Vec2 {
    (end - start).perp().normalize() * HALF_WIDTH
}

/// Calculates the left and right wall points for the `start` point of the wall,
/// considering intersections with other walls.
fn offset_points(
    start: Vec2,
    end: Vec2,
    edges: MinMaxResult<(Vec2, Vec2)>,
    width: Vec2,
) -> (Vec2, Vec2) {
    match edges {
        MinMaxResult::NoElements => (start + width, start - width),
        MinMaxResult::OneElement((a, b)) => (
            wall_intersection(start, end, a, b, width),
            wall_intersection(start, end, b, a, -width),
        ),
        MinMaxResult::MinMax((min_a, min_b), (max_a, max_b)) => (
            wall_intersection(start, end, max_a, max_b, width),
            wall_intersection(start, end, min_b, min_a, -width),
        ),
    }
}

/// Returns the points with the maximum and minimum angle relative
/// to the wall vector that come out of point `start`.
fn minmax_angles(start: Vec2, end: Vec2, edges: &[(Vec2, Vec2)]) -> MinMaxResult<(Vec2, Vec2)> {
    let dir = end - start;
    edges
        .iter()
        .filter_map(|&(a, b)| {
            if a == start && b != end {
                Some((a, b))
            } else if b == start && a != end {
                Some((b, a))
            } else {
                None
            }
        })
        .minmax_by_key(|&(a, b)| {
            let angle = (b - a).angle_between(dir);
            if angle < 0.0 {
                angle + 2.0 * PI
            } else {
                angle
            }
        })
}

/// Returns the intersection of the part of the wall that is `width` away
/// at the line constructed from `start` and `end` points with another part of the wall.
///
/// If the walls do not intersect, then returns a point that is a `width` away from the `start` point.
fn wall_intersection(start: Vec2, end: Vec2, a: Vec2, b: Vec2, width: Vec2) -> Vec2 {
    Line::with_offset(start, end, width)
        .intersection(Line::with_offset(a, b, -width_vec(a, b)))
        .unwrap_or_else(|| start + width)
}

#[derive(Clone, Copy, PartialEq)]
struct Line {
    a: f32,
    b: f32,
    c: f32,
}

impl Line {
    #[must_use]
    fn new(p1: Vec2, p2: Vec2) -> Self {
        let a = p2.y - p1.y;
        let b = p1.x - p2.x;
        let c = a * p1.x + b * p1.y;
        Self { a, b, c }
    }

    #[must_use]
    fn with_offset(p1: Vec2, p2: Vec2, offset: Vec2) -> Self {
        Self::new(p1 + offset, p2 + offset)
    }

    fn intersection(self, rhs: Self) -> Option<Vec2> {
        let det = self.a * rhs.b - rhs.a * self.b;
        if det == 0.0 {
            None
        } else {
            Some(Vec2 {
                x: (rhs.b * self.c - self.b * rhs.c) / det,
                y: (self.a * rhs.c - rhs.a * self.c) / det,
            })
        }
    }
}

/// Stores a handle for the lot line material.
#[derive(Resource)]
struct WallMaterial(Handle<StandardMaterial>);

impl FromWorld for WallMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let handle = materials.add(StandardMaterial::default());
        Self(handle)
    }
}

#[derive(Bundle)]
struct WallBundle {
    edges: WallEdges,
    parent_sync: ParentSync,
    replication: Replication,
}

impl WallBundle {
    fn new(edges: Vec<(Vec2, Vec2)>) -> Self {
        Self {
            edges: WallEdges(edges),
            parent_sync: Default::default(),
            replication: Replication,
        }
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(super) struct WallEdges(Vec<(Vec2, Vec2)>);

/// Client event to request a wall creation.
#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct WallCreate {
    lot_entity: Entity,
    edge: (Vec2, Vec2),
}

impl MapNetworkEntities for WallCreate {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.lot_entity = mapper.map(self.lot_entity);
    }
}

#[derive(Event, Serialize, Deserialize)]
struct WallEventConfirmed;

/// A component that marks that entity can be placed only on walls.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub(super) struct WallObject;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersecting_lines() {
        let line_a = Line::new(Vec2::X, Vec2::ONE);
        let line_b = Line::new(Vec2::ZERO, Vec2::X * 2.0);

        assert_eq!(line_a.intersection(line_b), Some(Vec2::X));
    }

    #[test]
    fn parallel_lines() {
        let line_a = Line::new(Vec2::X, Vec2::ONE);
        let line_b = Line::new(Vec2::X * 2.0, Vec2::ONE * 2.0);

        assert_eq!(line_a.intersection(line_b), None);
    }

    #[test]
    fn single_wall() {
        const A: Vec2 = Vec2::ZERO;
        const B: Vec2 = Vec2::X;
        const EDGES: &[(Vec2, Vec2)] = &[(A, B)];

        let width = width_vec(A, B);
        let a_edges = minmax_angles(A, B, EDGES);
        let (left_a, right_a) = offset_points(A, B, a_edges, width);

        let b_edges = minmax_angles(B, A, EDGES);
        let (right_b, left_b) = offset_points(B, A, b_edges, -width);

        assert_eq!(left_a, Vec2::new(0.0, 0.075));
        assert_eq!(right_a, Vec2::new(0.0, -0.075));
        assert_eq!(left_b, Vec2::new(1.0, 0.075));
        assert_eq!(right_b, Vec2::new(1.0, -0.075));
    }

    #[test]
    fn opposite_walls() {
        const EDGES: &[(Vec2, Vec2)] = &[(Vec2::ZERO, Vec2::X), (Vec2::NEG_X, Vec2::ZERO)];
        const LEFT: [Vec2; 4] = [
            Vec2::new(0.0, 0.075),
            Vec2::new(0.0, -0.075),
            Vec2::new(1.0, 0.075),
            Vec2::new(1.0, -0.075),
        ];
        let mut right = LEFT.map(|vertex| -vertex);
        right.reverse();

        for (&(a, b), expected) in EDGES.iter().zip(&[LEFT, right]) {
            let width = width_vec(a, b);
            let a_edges = minmax_angles(a, b, EDGES);
            let (left_a, right_a) = offset_points(a, b, a_edges, width);

            let b_edges = minmax_angles(b, a, EDGES);
            let (right_b, left_b) = offset_points(b, a, b_edges, -width);

            for (actual, expected) in expected.iter().zip(&[left_a, right_a, left_b, right_b]) {
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn diagonal_walls() {
        const EDGES: &[(Vec2, Vec2)] = &[(Vec2::ZERO, Vec2::ONE), (Vec2::ZERO, Vec2::NEG_ONE)];
        const LEFT: [Vec2; 4] = [
            Vec2::new(-0.05303301, 0.05303301),
            Vec2::new(0.05303301, -0.05303301),
            Vec2::new(0.946967, 1.053033),
            Vec2::new(1.053033, 0.946967),
        ];

        for (&(a, b), expected) in EDGES.iter().zip(&[LEFT, LEFT.map(|vertex| -vertex)]) {
            let width = width_vec(a, b);
            let a_edges = minmax_angles(a, b, EDGES);
            let (left_a, right_a) = offset_points(a, b, a_edges, width);

            let b_edges = minmax_angles(b, a, EDGES);
            let (right_b, left_b) = offset_points(b, a, b_edges, -width);

            for (expected, actual) in expected.iter().zip(&[left_a, right_a, left_b, right_b]) {
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn crossed_walls() {
        const EDGES: &[(Vec2, Vec2)] = &[
            (Vec2::ZERO, Vec2::X),
            (Vec2::ZERO, Vec2::NEG_X),
            (Vec2::ZERO, Vec2::Y),
            (Vec2::ZERO, Vec2::NEG_Y),
        ];
        const HORIZONTAL: [Vec2; 4] = [
            Vec2::new(0.075, 0.075),
            Vec2::new(0.075, -0.075),
            Vec2::new(1.0, 0.075),
            Vec2::new(1.0, -0.075),
        ];
        const VERTICAL: [Vec2; 4] = [
            Vec2::new(-0.075, 0.075),
            Vec2::new(0.075, 0.075),
            Vec2::new(-0.075, 1.0),
            Vec2::new(0.075, 1.0),
        ];

        for (&(a, b), expected) in EDGES.iter().zip(&[
            HORIZONTAL,
            HORIZONTAL.map(|vertex| -vertex),
            VERTICAL,
            VERTICAL.map(|vertex| -vertex),
        ]) {
            let width = width_vec(a, b);
            let a_edges = minmax_angles(a, b, EDGES);
            let (left_a, right_a) = offset_points(a, b, a_edges, width);

            let b_edges = minmax_angles(b, a, EDGES);
            let (right_b, left_b) = offset_points(b, a, b_edges, -width);

            for (expected, actual) in expected.iter().zip(&[left_a, right_a, left_b, right_b]) {
                assert_eq!(actual, expected);
            }
        }
    }
}
