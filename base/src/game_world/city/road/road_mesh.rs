use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::Collider;
use bevy::prelude::*;
use itertools::MinMaxResult;

use crate::{
    dynamic_mesh::DynamicMesh,
    game_world::segment::{PointKind, Segment, SegmentConnections},
};

/// Small offset to avoid Z-fighting with the ground.
const HEIGHT: f32 = 0.001;
const UV_ROTATION: f32 = FRAC_PI_2; // Because the texture is vertical.

pub(super) fn generate(
    mesh: &mut DynamicMesh,
    segment: Segment,
    connections: &SegmentConnections,
    half_width: f32,
) {
    mesh.clear();

    if segment.is_zero() {
        return;
    }

    let disp = segment.displacement();
    let width_disp = disp.perp().normalize() * half_width;

    let start_connections = connections.side_segments(PointKind::Start, disp);
    let (mut start_left, mut start_right) =
        segment.offset_points(width_disp, half_width, start_connections);

    let end_connections = connections.side_segments(PointKind::End, -disp);
    let (mut end_right, mut end_left) =
        segment
            .inverse()
            .offset_points(-width_disp, half_width, end_connections);

    // Use origin as center.
    start_left -= segment.start;
    start_right -= segment.start;
    end_left -= segment.start;
    end_right -= segment.start;

    // Remove segment rotation, it will be controlled by `Transform`.
    let segment_rotation = Mat2::from_angle(-disp.to_angle());
    start_left = segment_rotation * start_left;
    start_right = segment_rotation * start_right;
    end_left = segment_rotation * end_left;
    end_right = segment_rotation * end_right;

    let width = half_width * 2.0;

    generate_surface(mesh, start_left, start_right, end_left, end_right, width);

    if let MinMaxResult::MinMax(_, _) = start_connections {
        generate_start_connection(mesh);
    }

    if let MinMaxResult::MinMax(_, _) = end_connections {
        generate_end_connection(mesh, segment.len(), width);
    }
}

fn generate_surface(
    mesh: &mut DynamicMesh,
    start_left: Vec2,
    start_right: Vec2,
    end_left: Vec2,
    end_right: Vec2,
    width: f32,
) {
    // To avoid interfering with the ground.
    mesh.positions.push([start_left.x, HEIGHT, start_left.y]);
    mesh.positions.push([start_right.x, HEIGHT, start_right.y]);
    mesh.positions.push([end_right.x, HEIGHT, end_right.y]);
    mesh.positions.push([end_left.x, HEIGHT, end_left.y]);

    // Road UV on X axis should go from 0.0 to 1.0.
    // But on Y we use segment length divided by width to scale it properly.
    let uv_rotation = Mat2::from_angle(UV_ROTATION);
    mesh.uvs.push([0.0, (uv_rotation * start_left).y / width]);
    mesh.uvs.push([1.0, (uv_rotation * start_right).y / width]);
    mesh.uvs.push([1.0, (uv_rotation * end_right).y / width]);
    mesh.uvs.push([0.0, (uv_rotation * end_left).y / width]);

    mesh.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

    mesh.indices.push(0);
    mesh.indices.push(3);
    mesh.indices.push(1);
    mesh.indices.push(1);
    mesh.indices.push(3);
    mesh.indices.push(2);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_start_connection(mesh: &mut DynamicMesh) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([0.0, HEIGHT, 0.0]);
    mesh.uvs.push([0.5, 0.0]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(1);
    mesh.indices.push(vertices_start);
    mesh.indices.push(0);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_end_connection(mesh: &mut DynamicMesh, len: f32, width: f32) {
    let vertices_start = mesh.vertices_count();
    let uv_rotation = Mat2::from_angle(UV_ROTATION);

    mesh.positions.push([len, HEIGHT, 0.0]);
    mesh.uvs
        .push([0.5, (uv_rotation * Vec2::X * len).y / width]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(3);
    mesh.indices.push(vertices_start);
    mesh.indices.push(2);
}

pub(super) fn generate_collider(segment: Segment, half_width: f32) -> Collider {
    if segment.is_zero() {
        return Default::default();
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let len = segment.len();
    vertices.push(Vec3::Z * half_width);
    vertices.push(-Vec3::Z * half_width);
    vertices.push(Vec3::new(len, 0.0, half_width));
    vertices.push(Vec3::new(len, 0.0, -half_width));

    indices.push([1, 0, 2]);
    indices.push([0, 3, 2]);

    Collider::trimesh(vertices, indices)
}
