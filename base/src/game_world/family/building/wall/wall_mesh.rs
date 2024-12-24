use std::f32::consts::FRAC_PI_2;

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
    let angle = -disp.to_angle();
    let width_disp = disp.perp().normalize() * HALF_WIDTH;
    let rotation_mat = Mat2::from_angle(angle);

    let start_connections = connections.side_segments(PointKind::Start, disp);
    let (start_left, start_right) =
        segment.offset_points(width_disp, HALF_WIDTH, start_connections);

    let end_connections = connections.side_segments(PointKind::End, -disp);
    let (end_right, end_left) =
        segment
            .inverse()
            .offset_points(-width_disp, HALF_WIDTH, end_connections);

    generate_top(
        mesh,
        segment,
        start_left,
        start_right,
        end_left,
        end_right,
        rotation_mat,
    );

    let inverse_winding = angle.abs() < FRAC_PI_2;
    let quat = Quat::from_axis_angle(Vec3::Y, angle);

    triangulator.set_inverse_winding(inverse_winding);
    generate_side(
        mesh,
        segment,
        apertures,
        triangulator,
        start_right,
        end_right,
        -width_disp,
        rotation_mat,
        quat,
    );

    triangulator.set_inverse_winding(!inverse_winding);
    generate_side(
        mesh,
        segment,
        apertures,
        triangulator,
        start_left,
        end_left,
        width_disp,
        rotation_mat,
        quat,
    );

    match start_connections {
        MinMaxResult::OneElement(_) => (),
        MinMaxResult::NoElements => generate_front(mesh, start_left, start_right, disp),
        MinMaxResult::MinMax(_, _) => generate_start_connection(mesh, segment),
    }

    match end_connections {
        MinMaxResult::OneElement(_) => (),
        MinMaxResult::NoElements => generate_back(mesh, end_left, end_right, disp),
        MinMaxResult::MinMax(_, _) => generate_end_connection(mesh, segment, rotation_mat),
    }
}

fn generate_top(
    mesh: &mut DynamicMesh,
    segment: Segment,
    start_left: Vec2,
    start_right: Vec2,
    end_left: Vec2,
    end_right: Vec2,
    rotation_mat: Mat2,
) {
    mesh.positions.push([start_left.x, HEIGHT, start_left.y]);
    mesh.positions.push([start_right.x, HEIGHT, start_right.y]);
    mesh.positions.push([end_right.x, HEIGHT, end_right.y]);
    mesh.positions.push([end_left.x, HEIGHT, end_left.y]);

    mesh.uvs
        .push((rotation_mat * (start_left - segment.start)).into());
    mesh.uvs
        .push((rotation_mat * (start_right - segment.start)).into());
    mesh.uvs
        .push((rotation_mat * (end_right - segment.start)).into());
    mesh.uvs
        .push((rotation_mat * (end_left - segment.start)).into());

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
    segment: Segment,
    apertures: &Apertures,
    triangulator: &mut Triangulator,
    start_side: Vec2,
    end_side: Vec2,
    width_disp: Vec2,
    rotation_mat: Mat2,
    quat: Quat,
) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([start_side.x, 0.0, start_side.y]);
    let start_uv = rotation_mat * (start_side - segment.start);
    mesh.uvs.push(start_uv.into());
    let normal = [width_disp.x, 0.0, width_disp.y];
    mesh.normals.push(normal);

    for aperture in apertures.iter().filter(|aperture| !aperture.hole) {
        generate_apertures(
            mesh,
            segment,
            aperture,
            normal,
            width_disp,
            rotation_mat,
            quat,
        );
    }

    mesh.positions.push([end_side.x, 0.0, end_side.y]);
    mesh.positions.push([end_side.x, HEIGHT, end_side.y]);
    mesh.positions.push([start_side.x, HEIGHT, start_side.y]);

    let end_uv = rotation_mat * (end_side - segment.start);
    mesh.uvs.push(end_uv.into());
    mesh.uvs.push([end_uv.x, end_uv.y + HEIGHT]);
    mesh.uvs.push([start_uv.x, start_uv.y + HEIGHT]);

    mesh.normals.extend_from_slice(&[normal; 3]);

    let mut last_index = mesh.vertices_count() - vertices_start;
    for aperture in apertures.iter().filter(|aperture| aperture.hole) {
        generate_apertures(
            mesh,
            segment,
            aperture,
            normal,
            width_disp,
            rotation_mat,
            quat,
        );

        triangulator.add_hole(last_index);
        last_index += aperture.cutout.len() as u32;
    }

    for &index in triangulator.triangulate(&mesh.positions[vertices_start as usize..]) {
        mesh.indices.push(vertices_start + index);
    }
}

fn generate_apertures(
    mesh: &mut DynamicMesh,
    segment: Segment,
    aperture: &Aperture,
    normal: [f32; 3],
    width_disp: Vec2,
    rotation_mat: Mat2,
    quat: Quat,
) {
    for &position in &aperture.cutout {
        let translated = quat * position.extend(0.0)
            + aperture.translation
            + Vec3::new(width_disp.x, 0.0, width_disp.y);

        mesh.positions.push(translated.into());

        let bottom_uv = rotation_mat * (translated.xz() - segment.start);
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
fn generate_start_connection(mesh: &mut DynamicMesh, segment: Segment) {
    let vertices_start = mesh.vertices_count();

    // Inside triangle to fill the gap between 3+ walls.
    mesh.positions
        .push([segment.start.x, HEIGHT, segment.start.y]);
    mesh.uvs.push([0.0, 0.0]);
    mesh.normals.push([0.0, 1.0, 0.0]);

    mesh.indices.push(1);
    mesh.indices.push(vertices_start);
    mesh.indices.push(0);
}

/// Inside triangle to fill the gap between 3+ walls.
fn generate_end_connection(mesh: &mut DynamicMesh, segment: Segment, rotation_mat: Mat2) {
    let vertices_start = mesh.vertices_count();

    mesh.positions.push([segment.end.x, HEIGHT, segment.end.y]);
    mesh.uvs
        .push((rotation_mat * (segment.end - segment.start)).into());
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
    let mut start = segment.start;
    let dir = segment.displacement().normalize();
    for aperture in apertures
        .iter()
        .filter(|aperture| !aperture.hole && !aperture.placing_object)
    {
        let first = aperture.cutout.first().expect("apertures can't be empty");
        let mut end = aperture.translation.xz();
        end += first.x * dir;

        generate_cuboid(&mut vertices, &mut indices, start, end);

        let last = aperture.cutout.last().unwrap();
        start = aperture.translation.xz();
        start += last.x * dir;
    }

    generate_cuboid(&mut vertices, &mut indices, start, segment.end);

    Collider::trimesh(vertices, indices)
}

fn generate_cuboid(vertices: &mut Vec<Vec3>, indices: &mut Vec<[u32; 3]>, start: Vec2, end: Vec2) {
    let last_index = vertices.len().try_into().expect("vertices should fit u32");

    let disp = end - start;
    let width_disp = disp.perp().normalize() * HALF_WIDTH;
    let left_start = start + width_disp;
    let right_start = start - width_disp;
    let left_end = end + width_disp;
    let right_end = end - width_disp;

    vertices.push(Vec3::new(left_start.x, 0.0, left_start.y));
    vertices.push(Vec3::new(right_start.x, 0.0, right_start.y));
    vertices.push(Vec3::new(right_end.x, 0.0, right_end.y));
    vertices.push(Vec3::new(left_end.x, 0.0, left_end.y));

    vertices.push(Vec3::new(left_start.x, HEIGHT, left_start.y));
    vertices.push(Vec3::new(right_start.x, HEIGHT, right_start.y));
    vertices.push(Vec3::new(right_end.x, HEIGHT, right_end.y));
    vertices.push(Vec3::new(left_end.x, HEIGHT, left_end.y));

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
