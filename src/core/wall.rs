pub(crate) mod creating_wall;

use std::f32::consts::PI;

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_renet::renet::RenetClient;
use itertools::{Itertools, MinMaxResult};
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    game_world::{GameEntity, GameWorld},
    network::network_event::{
        client_event::{ClientEvent, ClientEventAppExt},
        server_event::{SendMode, ServerEvent, ServerEventAppExt},
    },
};
use creating_wall::CreatingWallPlugin;

pub(super) struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CreatingWallPlugin)
            .register_type::<WallEdges>()
            .add_mapped_client_event::<WallCreate>()
            .add_server_event::<WallEventConfirmed>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                Self::init_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_system(Self::wall_creation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::mesh_update_system.run_if_resource_exists::<GameWorld>());
    }
}

impl WallPlugin {
    fn init_system(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        spawned_walls: Query<Entity, Added<WallEdges>>,
    ) {
        for entity in &spawned_walls {
            commands.entity(entity).insert((
                Name::new("Walls"),
                PbrBundle {
                    mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                    material: materials.add(StandardMaterial::default()),
                    ..Default::default()
                },
            ));
        }
    }

    fn wall_creation_system(
        mut commands: Commands,
        mut create_events: EventReader<ClientEvent<WallCreate>>,
        mut confirm_events: EventWriter<ServerEvent<WallEventConfirmed>>,
        children: Query<&Children>,
        mut walls: Query<&mut WallEdges, With<GameEntity>>,
    ) {
        for ClientEvent { client_id, event } in create_events.iter().copied() {
            confirm_events.send(ServerEvent {
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
                parent.spawn((WallEdges(vec![event.edge]), GameEntity));
            });
        }
    }

    fn mesh_update_system(
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(&WallEdges, &Handle<Mesh>), Changed<WallEdges>>,
    ) {
        for (edges, mesh_handle) in &walls {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("associated mesh handle should be valid");

            let mut positions = Vec::new();
            let mut indices = Vec::new();
            for &(a, b) in edges.iter() {
                let last_index: u32 = positions
                    .len()
                    .try_into()
                    .expect("vertex index should fit u32");

                let width = width_vec(a, b);
                let a_edges = minmax_angles(a, b, edges);
                let (left_a, right_a) = offset_points(a, b, a_edges, width);

                let b_edges = minmax_angles(b, a, edges);
                let (left_b, right_b) = offset_points(b, a, b_edges, -width);

                positions.push(Vec3::new(left_a.x, 0.0, left_a.y));
                positions.push(Vec3::new(right_a.x, 0.0, right_a.y));
                positions.push(Vec3::new(left_b.x, 0.0, left_b.y));
                positions.push(Vec3::new(right_b.x, 0.0, right_b.y));

                const HEIGHT: f32 = 2.0;
                positions.push(Vec3::new(left_a.x, HEIGHT, left_a.y));
                positions.push(Vec3::new(right_a.x, HEIGHT, right_a.y));
                positions.push(Vec3::new(left_b.x, HEIGHT, left_b.y));
                positions.push(Vec3::new(right_b.x, HEIGHT, right_b.y));

                // Top
                indices.push(last_index + 5);
                indices.push(last_index + 4);
                indices.push(last_index + 6);
                indices.push(last_index + 4);
                indices.push(last_index + 7);
                indices.push(last_index + 6);

                // Left
                indices.push(last_index + 3);
                indices.push(last_index + 4);
                indices.push(last_index);
                indices.push(last_index + 3);
                indices.push(last_index + 7);
                indices.push(last_index + 4);

                // Right
                indices.push(last_index + 1);
                indices.push(last_index + 5);
                indices.push(last_index + 2);
                indices.push(last_index + 5);
                indices.push(last_index + 6);
                indices.push(last_index + 2);

                match a_edges {
                    MinMaxResult::OneElement(_) => (),
                    MinMaxResult::NoElements => {
                        // Back
                        indices.push(last_index);
                        indices.push(last_index + 5);
                        indices.push(last_index + 1);
                        indices.push(last_index);
                        indices.push(last_index + 4);
                        indices.push(last_index + 5);
                    }
                    MinMaxResult::MinMax(_, _) => {
                        // Inside triangle to fill the gap between 3+ walls.
                        positions.push(Vec3::new(a.x, HEIGHT, a.y));
                        indices.push(last_index + 5);
                        indices.push(last_index + 8); // Point `b` added above.
                        indices.push(last_index + 4);
                    }
                }

                match b_edges {
                    MinMaxResult::OneElement(_) => (),
                    MinMaxResult::NoElements => {
                        // Front
                        indices.push(last_index + 2);
                        indices.push(last_index + 6);
                        indices.push(last_index + 3);
                        indices.push(last_index + 6);
                        indices.push(last_index + 7);
                        indices.push(last_index + 3);
                    }
                    MinMaxResult::MinMax(_, _) => {
                        // Inside triangle to fill the gap between 3+ walls.
                        positions.push(Vec3::new(b.x, HEIGHT, b.y));
                        indices.push(last_index + 6);
                        indices.push(last_index + 7);
                        indices.push(last_index + 8); // Point `b` added above.
                    }
                }
            }

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.set_indices(Some(Indices::U32(indices)));
        }
    }
}

/// Calculates the wall thickness vector that faces to the left relative to the wall vector.
fn width_vec(start: Vec2, end: Vec2) -> Vec2 {
    const WIDTH: f32 = 0.25;
    const HALF_WIDTH: f32 = WIDTH / 2.0;
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
            wall_intersection(start, end, a, b, -width),
            wall_intersection(start, end, b, a, width),
        ),
        MinMaxResult::MinMax((min_a, min_b), (max_a, max_b)) => (
            wall_intersection(start, end, max_a, max_b, -width),
            wall_intersection(start, end, min_b, min_a, width),
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
        .intersection(Line::with_offset(a, b, width_vec(a, b)))
        .unwrap_or_else(|| start - width)
}

#[derive(Clone, Copy, PartialEq, Debug)]
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
#[derive(Clone, Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(super) struct WallEdges(Vec<(Vec2, Vec2)>);

/// Client event that a wall has been created.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct WallCreate {
    lot_entity: Entity,
    edge: (Vec2, Vec2),
}

impl MapEntities for WallCreate {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.lot_entity = entity_map.get(self.lot_entity)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WallEventConfirmed;

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
        let (left_b, right_b) = offset_points(B, A, b_edges, width);

        assert_eq!(left_a, Vec2::new(0.0, 0.125));
        assert_eq!(right_a, Vec2::new(0.0, -0.125));
        assert_eq!(left_b, Vec2::new(1.0, 0.125));
        assert_eq!(right_b, Vec2::new(1.0, -0.125));
    }

    #[test]
    fn diagonal_walls() {
        const EDGES: &[(Vec2, Vec2)] = &[(Vec2::ZERO, Vec2::ONE), (Vec2::ZERO, Vec2::NEG_ONE)];
        const LEFT: [Vec2; 4] = [
            Vec2::new(-0.088388346, 0.088388346),
            Vec2::new(0.088388346, -0.088388346),
            Vec2::new(0.9116117, 1.0883883),
            Vec2::new(1.0883883, 0.9116117),
        ];

        for (&(a, b), expected) in EDGES.iter().zip(&[LEFT, LEFT.map(|vec| -vec)]) {
            let width = width_vec(a, b);
            let a_edges = minmax_angles(a, b, EDGES);
            let (left_a, right_a) = offset_points(a, b, a_edges, width);

            let b_edges = minmax_angles(b, a, EDGES);
            let (left_b, right_b) = offset_points(b, a, b_edges, width);

            for (actual, expected) in expected.iter().zip(&[left_a, right_a, left_b, right_b]) {
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
            Vec2::new(0.125, 0.125),
            Vec2::new(0.125, -0.125),
            Vec2::new(1.0, 0.125),
            Vec2::new(1.0, -0.125),
        ];
        const VERTICAL: [Vec2; 4] = [
            Vec2::new(-0.125, 0.125),
            Vec2::new(0.125, 0.125),
            Vec2::new(-0.125, 1.0),
            Vec2::new(0.125, 1.0),
        ];

        for (&(a, b), expected) in EDGES.iter().zip(&[
            HORIZONTAL,
            HORIZONTAL.map(|vec| -vec),
            VERTICAL,
            VERTICAL.map(|vec| -vec),
        ]) {
            let width = width_vec(a, b);
            let a_edges = minmax_angles(a, b, EDGES);
            let (left_a, right_a) = offset_points(a, b, a_edges, width);

            let b_edges = minmax_angles(b, a, EDGES);
            let (left_b, right_b) = offset_points(b, a, b_edges, width);

            for (actual, expected) in expected.iter().zip(&[left_a, right_a, left_b, right_b]) {
                assert_eq!(actual, expected);
            }
        }
    }
}
