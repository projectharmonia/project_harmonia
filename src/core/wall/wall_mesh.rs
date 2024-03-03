use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    prelude::*,
    render::mesh::{Indices, VertexAttributeValues},
};
use itertools::{Itertools, MinMaxResult};

use super::{Aperture, Apertures, PointKind, Wall, WallConnection, WallConnections};
use crate::core::line::Line;

const WIDTH: f32 = 0.15;
const HEIGHT: f32 = 2.8;
pub(crate) const HALF_WIDTH: f32 = WIDTH / 2.0;

#[derive(Default)]
pub(super) struct WallMesh {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl WallMesh {
    pub(super) fn take(mesh: &mut Mesh) -> Self {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("all wall meshes should have positions");
        };
        let Some(VertexAttributeValues::Float32x2(uvs)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("all wall meshes should have UVs");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("all wall meshes should have normals");
        };
        let Some(Indices::U32(indices)) = mesh.remove_indices() else {
            panic!("all wall meshes should have U32 indices");
        };

        Self {
            positions,
            uvs,
            normals,
            indices,
        }
    }

    pub(super) fn generate(
        &mut self,
        wall: Wall,
        connections: &WallConnections,
        apertures: &Apertures,
    ) {
        self.clear();

        if wall.start == wall.end {
            return;
        }

        let disp = wall.displacement();
        let angle = -disp.to_angle();
        let width = wall_width(disp);
        let rotation_mat = Mat2::from_angle(angle);

        let start_walls = minmax_angles(disp, PointKind::Start, &connections.start);
        let (start_left, start_right) = offset_points(wall, start_walls, width);

        let end_walls = minmax_angles(-disp, PointKind::End, &connections.end);
        let (end_right, end_left) = offset_points(wall.inverse(), end_walls, -width);

        self.generate_top(
            wall,
            start_left,
            start_right,
            end_left,
            end_right,
            rotation_mat,
        );

        let inverse_winding = angle.abs() < FRAC_PI_2;
        let quat = Quat::from_axis_angle(Vec3::Y, angle);

        let vertices_start = self.positions.len();
        let indices_start = self.indices.len();

        self.generate_side(
            wall,
            apertures,
            start_right,
            end_right,
            -width,
            rotation_mat,
            quat,
            inverse_winding,
        );

        self.mirror_side(vertices_start, indices_start, width);

        match start_walls {
            MinMaxResult::OneElement(_) => (),
            MinMaxResult::NoElements => self.generate_front(start_left, start_right, disp),
            MinMaxResult::MinMax(_, _) => self.generate_start_connection(wall),
        }

        match end_walls {
            MinMaxResult::OneElement(_) => (),
            MinMaxResult::NoElements => self.generate_back(end_left, end_right, disp),
            MinMaxResult::MinMax(_, _) => self.generate_end_connection(wall, rotation_mat),
        }
    }

    fn generate_top(
        &mut self,
        wall: Wall,
        start_left: Vec2,
        start_right: Vec2,
        end_left: Vec2,
        end_right: Vec2,
        rotation_mat: Mat2,
    ) {
        self.positions.push([start_left.x, HEIGHT, start_left.y]);
        self.positions.push([start_right.x, HEIGHT, start_right.y]);
        self.positions.push([end_right.x, HEIGHT, end_right.y]);
        self.positions.push([end_left.x, HEIGHT, end_left.y]);

        self.uvs
            .push((rotation_mat * (start_left - wall.start)).into());
        self.uvs
            .push((rotation_mat * (start_right - wall.start)).into());
        self.uvs
            .push((rotation_mat * (end_right - wall.start)).into());
        self.uvs
            .push((rotation_mat * (end_left - wall.start)).into());

        self.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

        self.indices.push(0);
        self.indices.push(3);
        self.indices.push(1);
        self.indices.push(1);
        self.indices.push(3);
        self.indices.push(2);
    }

    fn generate_side(
        &mut self,
        wall: Wall,
        apertures: &Apertures,
        start_side: Vec2,
        end_side: Vec2,
        width: Vec2,
        rotation_mat: Mat2,
        quat: Quat,
        inverse_winding: bool,
    ) {
        let vertices_start = self.vertices_count();

        self.positions.push([start_side.x, 0.0, start_side.y]);
        let start_uv = rotation_mat * (start_side - wall.start);
        self.uvs.push(start_uv.into());
        let normal = [width.x, 0.0, width.y];
        self.normals.push(normal);

        for aperture in apertures.iter_clippings() {
            self.generate_apertures(wall, aperture, normal, width, rotation_mat, quat);
        }

        self.positions.push([end_side.x, 0.0, end_side.y]);
        self.positions.push([end_side.x, HEIGHT, end_side.y]);
        self.positions.push([start_side.x, HEIGHT, start_side.y]);

        let end_uv = rotation_mat * (end_side - wall.start);
        self.uvs.push(end_uv.into());
        self.uvs.push([end_uv.x, end_uv.y + HEIGHT]);
        self.uvs.push([start_uv.x, start_uv.y + HEIGHT]);

        self.normals.extend_from_slice(&[normal; 3]);

        let mut hole_indices = Vec::new();
        let mut last_index = self.positions.len() - vertices_start as usize;
        for aperture in apertures.iter_holes() {
            self.generate_apertures(wall, aperture, normal, width, rotation_mat, quat);

            hole_indices.push(last_index);
            last_index += aperture.positions.len();
        }

        let vertices: Vec<_> = self.positions[vertices_start as usize..]
            .iter()
            .flat_map(|&[x, y, _]| [x, y])
            .collect();

        let mut indices = earcutr::earcut(&vertices, &hole_indices, 2)
            .expect("vertices should be triangulatable");

        if inverse_winding {
            for triangle in indices.chunks_exact_mut(3) {
                triangle.swap(0, 2);
            }
        }

        for index in indices {
            self.indices.push(vertices_start + index as u32);
        }
    }

    fn mirror_side(&mut self, vertices_start: usize, indices_start: usize, width: Vec2) {
        let pos_end = self.positions.len();
        let vertices_count = pos_end - vertices_start;
        let indices_end = self.indices.len();
        let indices_count = indices_end - indices_start;

        // Copy original side data.
        self.positions.resize(pos_end + vertices_count, [0.0; 3]);
        self.positions.copy_within(vertices_start..pos_end, pos_end);
        self.indices.resize(indices_end + indices_count, 0);
        self.indices
            .copy_within(indices_start..indices_end, indices_end);
        self.uvs.resize(pos_end + vertices_count, [0.0; 2]);
        self.uvs.copy_within(vertices_start..pos_end, pos_end);
        self.normals.resize(pos_end + vertices_count, [0.0; 3]);
        self.normals.copy_within(vertices_start..pos_end, pos_end);

        // Offset the side by the specified width.
        for position in &mut self.positions[pos_end..] {
            position[0] += 2.0 * width.x;
            position[2] += 2.0 * width.y;
        }

        // Mirror normals.
        for normal in &mut self.normals[pos_end..] {
            for value in normal {
                *value = -*value;
            }
        }

        for triangle in self.indices[indices_end..].chunks_exact_mut(3) {
            // Inverse winding.
            triangle.swap(0, 2);

            // Shift all indices to the next side.
            for index in triangle {
                *index += vertices_count as u32;
            }
        }
    }

    fn generate_apertures(
        &mut self,
        wall: Wall,
        aperture: &Aperture,
        normal: [f32; 3],
        width: Vec2,
        rotation_mat: Mat2,
        quat: Quat,
    ) {
        for &position in &aperture.positions {
            let translated = quat * position.extend(0.0)
                + aperture.translation
                + Vec3::new(width.x, 0.0, width.y);

            self.positions.push(translated.into());

            let bottom_uv = rotation_mat * (translated.xz() - wall.start);
            self.uvs.push([bottom_uv.x, bottom_uv.y + position.y]);

            self.normals.push(normal)
        }
    }

    fn generate_front(&mut self, start_left: Vec2, start_right: Vec2, disp: Vec2) {
        let vertices_start = self.vertices_count();

        self.positions.push([start_left.x, 0.0, start_left.y]);
        self.positions.push([start_left.x, HEIGHT, start_left.y]);
        self.positions.push([start_right.x, HEIGHT, start_right.y]);
        self.positions.push([start_right.x, 0.0, start_right.y]);

        self.uvs.push([0.0, 0.0]);
        self.uvs.push([0.0, HEIGHT]);
        self.uvs.push([WIDTH, HEIGHT]);
        self.uvs.push([WIDTH, 0.0]);

        self.normals
            .extend_from_slice(&[[-disp.x, 0.0, -disp.y]; 4]);

        self.indices.push(vertices_start);
        self.indices.push(vertices_start + 1);
        self.indices.push(vertices_start + 3);
        self.indices.push(vertices_start + 1);
        self.indices.push(vertices_start + 2);
        self.indices.push(vertices_start + 3);
    }

    fn generate_back(&mut self, end_left: Vec2, end_right: Vec2, disp: Vec2) {
        let vertices_start = self.vertices_count();

        // Back
        self.positions.push([end_left.x, 0.0, end_left.y]);
        self.positions.push([end_left.x, HEIGHT, end_left.y]);
        self.positions.push([end_right.x, HEIGHT, end_right.y]);
        self.positions.push([end_right.x, 0.0, end_right.y]);

        self.uvs.push([0.0, 0.0]);
        self.uvs.push([0.0, HEIGHT]);
        self.uvs.push([WIDTH, HEIGHT]);
        self.uvs.push([WIDTH, 0.0]);

        self.normals.extend_from_slice(&[[disp.x, 0.0, disp.y]; 4]);

        self.indices.push(vertices_start);
        self.indices.push(vertices_start + 3);
        self.indices.push(vertices_start + 1);
        self.indices.push(vertices_start + 1);
        self.indices.push(vertices_start + 3);
        self.indices.push(vertices_start + 2);
    }

    /// Inside triangle to fill the gap between 3+ walls.
    fn generate_start_connection(&mut self, wall: Wall) {
        let vertices_start = self.vertices_count();

        // Inside triangle to fill the gap between 3+ walls.
        self.positions.push([wall.start.x, HEIGHT, wall.start.y]);
        self.uvs.push([0.0, 0.0]);
        self.normals.push([0.0, 1.0, 0.0]);

        self.indices.push(1);
        self.indices.push(vertices_start);
        self.indices.push(0);
    }

    /// Inside triangle to fill the gap between 3+ walls.
    fn generate_end_connection(&mut self, wall: Wall, rotation_mat: Mat2) {
        let vertices_start = self.vertices_count();

        self.positions.push([wall.end.x, HEIGHT, wall.end.y]);
        self.uvs
            .push((rotation_mat * (wall.end - wall.start)).into());
        self.normals.push([0.0, 1.0, 0.0]);

        self.indices.push(3);
        self.indices.push(vertices_start);
        self.indices.push(2);
    }

    fn vertices_count(&self) -> u32 {
        self.positions
            .len()
            .try_into()
            .expect("vertices should fit u32")
    }

    fn clear(&mut self) {
        self.positions.clear();
        self.uvs.clear();
        self.normals.clear();
        self.indices.clear();
    }

    pub(super) fn apply(self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_indices(Indices::U32(self.indices))
    }
}

/// Calculates the wall thickness vector that faces to the left relative to the wall vector.
fn wall_width(disp: Vec2) -> Vec2 {
    disp.perp().normalize() * HALF_WIDTH
}

/// Calculates the left and right wall points for the `start` point of the wall,
/// considering intersections with other walls.
fn offset_points(wall: Wall, min_max_walls: MinMaxResult<Wall>, width: Vec2) -> (Vec2, Vec2) {
    match min_max_walls {
        MinMaxResult::NoElements => (wall.start + width, wall.start - width),
        MinMaxResult::OneElement(other_wall) => {
            let other_width = wall_width(other_wall.displacement());
            (
                wall_intersection(wall, width, other_wall, -other_width),
                wall_intersection(wall, -width, other_wall.inverse(), other_width),
            )
        }
        MinMaxResult::MinMax(min_wall, max_wall) => (
            wall_intersection(wall, width, max_wall, -wall_width(max_wall.displacement())),
            wall_intersection(
                wall,
                -width,
                min_wall.inverse(),
                wall_width(min_wall.displacement()),
            ),
        ),
    }
}

/// Returns the points with the maximum and minimum angle relative
/// to the displacement vector.
fn minmax_angles(
    disp: Vec2,
    point_kind: PointKind,
    point_connections: &[WallConnection],
) -> MinMaxResult<Wall> {
    point_connections
        .iter()
        .map(|connection| {
            // Rotate points based on connection type.
            match (point_kind, connection.point_kind) {
                (PointKind::Start, PointKind::End) => connection.wall.inverse(),
                (PointKind::End, PointKind::Start) => connection.wall,
                (PointKind::Start, PointKind::Start) => connection.wall,
                (PointKind::End, PointKind::End) => connection.wall.inverse(),
            }
        })
        .minmax_by_key(|wall| {
            let angle = wall.displacement().angle_between(disp);
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
fn wall_intersection(wall: Wall, width: Vec2, other_wall: Wall, other_width: Vec2) -> Vec2 {
    let other_line = Line::with_offset(other_wall.start, other_wall.end, other_width);

    Line::with_offset(wall.start, wall.end, width)
        .intersection(other_line)
        .unwrap_or_else(|| wall.start + width)
}
