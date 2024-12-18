use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::Collider;
use bevy::prelude::*;
use itertools::MinMaxResult;

use crate::{
    dynamic_mesh::DynamicMesh,
    game_world::segment::{PointKind, Segment, SplineConnections},
};

/// Small offset to avoid Z-fighting with the ground.
const HEIGHT: f32 = 0.001;

pub(super) fn generate(
    mesh: &mut DynamicMesh,
    segment: Segment,
    connections: &SplineConnections,
    half_width: f32,
) {
    mesh.clear();

    if segment.start == segment.end {
        return;
    }

    let disp = segment.displacement();
    let angle = -disp.to_angle();
    let width_disp = disp.perp().normalize() * half_width;
    let rotation_mat = Mat2::from_angle(angle + FRAC_PI_2); // PI/2 because the texture is vertical.

    let start_connections = connections.minmax_angles(disp, PointKind::Start);
    let (start_left, start_right) =
        segment.offset_points(width_disp, half_width, start_connections);

    let end_connections = connections.minmax_angles(-disp, PointKind::End);
    let (end_right, end_left) =
        segment
            .inverse()
            .offset_points(-width_disp, half_width, end_connections);

    let width = half_width * 2.0;

    generate_surface(
        mesh,
        segment,
        start_left,
        start_right,
        end_left,
        end_right,
        rotation_mat,
        width,
    );

    if let MinMaxResult::MinMax(_, _) = start_connections {
        generate_start_connection(mesh, segment);
    }

    if let MinMaxResult::MinMax(_, _) = end_connections {
        generate_end_connection(mesh, segment, rotation_mat, width);
    }
}

fn generate_surface(
    mesh: &mut DynamicMesh,
    segment: Segment,
    start_left: Vec2,
    start_right: Vec2,
    end_left: Vec2,
    end_right: Vec2,
    rotation_mat: Mat2,
    width: f32,
) {
    // To avoid interfering with the ground.
    mesh.positions.push([start_left.x, HEIGHT, start_left.y]);
    mesh.positions.push([start_right.x, HEIGHT, start_right.y]);
    mesh.positions.push([end_right.x, HEIGHT, end_right.y]);
    mesh.positions.push([end_left.x, HEIGHT, end_left.y]);

    // Road UV on X axis should go from 0.0 to 1.0.
    // But on Y we use segment length divided by width to scale it properly.
    mesh.uvs
        .push([0.0, (rotation_mat * (start_left - segment.start)).y / width]);
    mesh.uvs.push([
        1.0,
        (rotation_mat * (start_right - segment.start)).y / width,
    ]);
    mesh.uvs
        .push([1.0, (rotation_mat * (end_right - segment.start)).y / width]);
    mesh.uvs
        .push([0.0, (rotation_mat * (end_left - segment.start)).y / width]);

    mesh.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

    mesh.indices.push(0);
    mesh.indices.push(3);
    mesh.indices.push(1);
    mesh.indices.push(1);
    mesh.indices.push(3);
    mesh.indices.push(2);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_start_connection(mesh: &mut DynamicMesh, segment: Segment) {
    let vertices_start = mesh.vertices_count();

    mesh.positions
        .push([segment.start.x, HEIGHT, segment.start.y]);
    mesh.uvs.push([0.5, 0.0]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(1);
    mesh.indices.push(vertices_start);
    mesh.indices.push(0);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_end_connection(
    mesh: &mut DynamicMesh,
    segment: Segment,
    rotation_mat: Mat2,
    width: f32,
) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([segment.end.x, HEIGHT, segment.end.y]);
    mesh.uvs.push([
        0.5,
        (rotation_mat * (segment.end - segment.start)).y / width,
    ]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(3);
    mesh.indices.push(vertices_start);
    mesh.indices.push(2);
}

pub(super) fn generate_collider(segment: Segment, half_width: f32) -> Collider {
    if segment.start == segment.end {
        return Default::default();
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let disp = segment.displacement();
    let width_disp = disp.perp().normalize() * half_width;
    let left_start = segment.start + width_disp;
    let right_start = segment.start - width_disp;
    let left_end = segment.end + width_disp;
    let right_end = segment.end - width_disp;

    vertices.push(Vec3::new(left_start.x, 0.0, left_start.y));
    vertices.push(Vec3::new(right_start.x, 0.0, right_start.y));
    vertices.push(Vec3::new(right_end.x, 0.0, right_end.y));
    vertices.push(Vec3::new(left_end.x, 0.0, left_end.y));

    indices.push([1, 0, 2]);
    indices.push([0, 3, 2]);

    Collider::trimesh(vertices, indices)
}
