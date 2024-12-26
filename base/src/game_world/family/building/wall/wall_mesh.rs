use avian3d::prelude::*;
use bevy::prelude::*;
use itertools::MinMaxResult;

use super::{triangulator::Triangulator, Aperture, Apertures};
use crate::{
    dynamic_mesh::DynamicMesh,
    game_world::segment::{PointKind, Segment, SegmentConnections},
};

const WIDTH: f32 = 0.15;
const HEIGHT: f32 = 2.8;
pub(crate) const HALF_WIDTH: f32 = WIDTH / 2.0;

pub(super) fn generate(
    mesh: &mut DynamicMesh,
    segment: Segment,
    connections: &SegmentConnections,
    apertures: &Apertures,
    triangulator: &mut Triangulator,
) {
    mesh.clear();

    if segment.is_zero() {
        return;
    }

    let disp = segment.displacement();
    let width_disp = disp.perp().normalize() * HALF_WIDTH;

    let start_connections = connections.side_segments(PointKind::Start, disp);
    let (mut start_left, mut start_right) =
        segment.offset_points(width_disp, HALF_WIDTH, start_connections);

    let end_connections = connections.side_segments(PointKind::End, -disp);
    let (mut end_right, mut end_left) =
        segment
            .inverse()
            .offset_points(-width_disp, HALF_WIDTH, end_connections);

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

    generate_top(mesh, start_left, start_right, end_left, end_right);

    generate_side(
        mesh,
        apertures,
        triangulator,
        start_right,
        end_right,
        -HALF_WIDTH,
    );

    generate_side(
        mesh,
        apertures,
        triangulator,
        start_left,
        end_left,
        HALF_WIDTH,
    );

    match start_connections {
        MinMaxResult::OneElement(_) => (),
        MinMaxResult::NoElements => generate_front(mesh, start_left, start_right, disp),
        MinMaxResult::MinMax(_, _) => generate_start_connection(mesh),
    }

    match end_connections {
        MinMaxResult::OneElement(_) => (),
        MinMaxResult::NoElements => generate_back(mesh, end_left, end_right, disp),
        MinMaxResult::MinMax(_, _) => generate_end_connection(mesh, segment.len()),
    }
}

fn generate_top(
    mesh: &mut DynamicMesh,
    start_left: Vec2,
    start_right: Vec2,
    end_left: Vec2,
    end_right: Vec2,
) {
    mesh.positions.push([start_left.x, HEIGHT, start_left.y]);
    mesh.positions.push([start_right.x, HEIGHT, start_right.y]);
    mesh.positions.push([end_right.x, HEIGHT, end_right.y]);
    mesh.positions.push([end_left.x, HEIGHT, end_left.y]);

    mesh.uvs.push(start_left.into());
    mesh.uvs.push(start_right.into());
    mesh.uvs.push(end_right.into());
    mesh.uvs.push(end_left.into());

    mesh.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

    mesh.indices.push(0);
    mesh.indices.push(3);
    mesh.indices.push(1);
    mesh.indices.push(1);
    mesh.indices.push(3);
    mesh.indices.push(2);
}

fn generate_side(
    mesh: &mut DynamicMesh,
    apertures: &Apertures,
    triangulator: &mut Triangulator,
    start_side: Vec2,
    end_side: Vec2,
    width_offset: f32,
) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([start_side.x, 0.0, start_side.y]);
    mesh.uvs.push(start_side.into());
    let normal = [0.0, 0.0, width_offset];
    mesh.normals.push(normal);

    for aperture in apertures.iter().filter(|aperture| !aperture.hole) {
        generate_apertures(mesh, aperture, normal);
    }

    mesh.positions.push([end_side.x, 0.0, end_side.y]);
    mesh.positions.push([end_side.x, HEIGHT, end_side.y]);
    mesh.positions.push([start_side.x, HEIGHT, start_side.y]);

    mesh.uvs.push(end_side.into());
    mesh.uvs.push([end_side.x, end_side.y + HEIGHT]);
    mesh.uvs.push([start_side.x, start_side.y + HEIGHT]);

    mesh.normals.extend_from_slice(&[normal; 3]);

    let mut last_index = mesh.vertices_count() - vertices_start;
    for aperture in apertures.iter().filter(|aperture| aperture.hole) {
        generate_apertures(mesh, aperture, normal);

        triangulator.add_hole(last_index);
        last_index += aperture.cutout.len() as u32;
    }

    for &index in triangulator.triangulate(
        &mesh.positions[vertices_start as usize..],
        width_offset.is_sign_negative(),
    ) {
        mesh.indices.push(vertices_start + index);
    }
}

fn generate_apertures(mesh: &mut DynamicMesh, aperture: &Aperture, normal: [f32; 3]) {
    for &position in &aperture.cutout {
        let mut translated = position.extend(normal[2]);
        translated.x += aperture.distance;

        mesh.positions.push(translated.into());

        let bottom_uv = translated.xz();
        mesh.uvs.push([bottom_uv.x, bottom_uv.y + position.y]);

        mesh.normals.push(normal)
    }
}

fn generate_front(mesh: &mut DynamicMesh, start_left: Vec2, start_right: Vec2, disp: Vec2) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([start_left.x, 0.0, start_left.y]);
    mesh.positions.push([start_left.x, HEIGHT, start_left.y]);
    mesh.positions.push([start_right.x, HEIGHT, start_right.y]);
    mesh.positions.push([start_right.x, 0.0, start_right.y]);

    mesh.uvs.push([0.0, 0.0]);
    mesh.uvs.push([0.0, HEIGHT]);
    mesh.uvs.push([WIDTH, HEIGHT]);
    mesh.uvs.push([WIDTH, 0.0]);

    mesh.normals
        .extend_from_slice(&[[-disp.x, 0.0, -disp.y]; 4]);

    mesh.indices.push(vertices_start);
    mesh.indices.push(vertices_start + 1);
    mesh.indices.push(vertices_start + 3);
    mesh.indices.push(vertices_start + 1);
    mesh.indices.push(vertices_start + 2);
    mesh.indices.push(vertices_start + 3);
}

fn generate_back(mesh: &mut DynamicMesh, end_left: Vec2, end_right: Vec2, disp: Vec2) {
    let vertices_start = mesh.vertices_count();

    // Back
    mesh.positions.push([end_left.x, 0.0, end_left.y]);
    mesh.positions.push([end_left.x, HEIGHT, end_left.y]);
    mesh.positions.push([end_right.x, HEIGHT, end_right.y]);
    mesh.positions.push([end_right.x, 0.0, end_right.y]);

    mesh.uvs.push([0.0, 0.0]);
    mesh.uvs.push([0.0, HEIGHT]);
    mesh.uvs.push([WIDTH, HEIGHT]);
    mesh.uvs.push([WIDTH, 0.0]);

    mesh.normals.extend_from_slice(&[[disp.x, 0.0, disp.y]; 4]);

    mesh.indices.push(vertices_start);
    mesh.indices.push(vertices_start + 3);
    mesh.indices.push(vertices_start + 1);
    mesh.indices.push(vertices_start + 1);
    mesh.indices.push(vertices_start + 3);
    mesh.indices.push(vertices_start + 2);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_start_connection(mesh: &mut DynamicMesh) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([0.0, HEIGHT, 0.0]);
    mesh.uvs.push([0.0, 0.0]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(1);
    mesh.indices.push(vertices_start);
    mesh.indices.push(0);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_end_connection(mesh: &mut DynamicMesh, len: f32) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([len, HEIGHT, 0.0]);
    mesh.uvs.push([len, 0.0]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(3);
    mesh.indices.push(vertices_start);
    mesh.indices.push(2);
}

/// Generates a simplified collider consists of cuboids.
///
/// Clippings split the collider into separate cuboids.
/// We generate a trimesh since navigation doesn't support compound shapes.
pub(super) fn generate_collider(segment: Segment, apertures: &Apertures) -> Collider {
    if segment.is_zero() {
        return Default::default();
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut start = 0.0;
    for aperture in apertures
        .iter()
        .filter(|aperture| !aperture.hole && !aperture.placing_object)
    {
        let first = aperture.cutout.first().expect("apertures can't be empty");
        let end = aperture.distance + first.x;

        generate_cuboid(&mut vertices, &mut indices, start, end);

        let last = aperture.cutout.last().unwrap();
        start = aperture.distance + last.x;
    }

    generate_cuboid(&mut vertices, &mut indices, start, segment.len());

    Collider::trimesh(vertices, indices)
}

fn generate_cuboid(vertices: &mut Vec<Vec3>, indices: &mut Vec<[u32; 3]>, start: f32, end: f32) {
    let last_index = vertices.len().try_into().expect("vertices should fit u32");

    vertices.push(Vec3::new(start, 0.0, HALF_WIDTH));
    vertices.push(Vec3::new(start, 0.0, -HALF_WIDTH));
    vertices.push(Vec3::new(end, 0.0, HALF_WIDTH));
    vertices.push(Vec3::new(end, 0.0, -HALF_WIDTH));

    vertices.push(Vec3::new(start, HEIGHT, HALF_WIDTH));
    vertices.push(Vec3::new(start, HEIGHT, -HALF_WIDTH));
    vertices.push(Vec3::new(end, HEIGHT, HALF_WIDTH));
    vertices.push(Vec3::new(end, HEIGHT, -HALF_WIDTH));

    // Top
    indices.push([last_index + 5, last_index + 4, last_index + 6]);
    indices.push([last_index + 4, last_index + 7, last_index + 6]);

    // Left
    indices.push([last_index + 3, last_index + 4, last_index]);
    indices.push([last_index + 3, last_index + 7, last_index + 4]);

    // Right
    indices.push([last_index + 1, last_index + 5, last_index + 2]);
    indices.push([last_index + 5, last_index + 6, last_index + 2]);

    // Back
    indices.push([last_index, last_index + 5, last_index + 1]);
    indices.push([last_index, last_index + 4, last_index + 5]);

    // Front
    indices.push([last_index + 2, last_index + 6, last_index + 3]);
    indices.push([last_index + 6, last_index + 7, last_index + 3]);
}
