use std::f32::consts::FRAC_PI_2;

use bevy::{
    prelude::*,
    render::mesh::{Indices, VertexAttributeValues},
};
use itertools::MinMaxResult;

use crate::{
    game_world::building::spline::{PointKind, SplineConnections, SplineSegment},
    math::segment::Segment,
};

/// Small offset to avoid Z-fighting with the ground.
const HEIGHT: f32 = 0.001;

#[derive(Default)]
pub(super) struct RoadMesh {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl RoadMesh {
    pub(super) fn take(mesh: &mut Mesh) -> Self {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("all road meshes should have positions");
        };
        let Some(VertexAttributeValues::Float32x2(uvs)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("all road meshes should have UVs");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("all road meshes should have normals");
        };
        let Some(Indices::U32(indices)) = mesh.remove_indices() else {
            panic!("all road meshes should have U32 indices");
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
        segment: SplineSegment,
        connections: &SplineConnections,
        half_width: f32,
    ) {
        self.clear();

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

        self.generate_surface(
            *segment,
            start_left,
            start_right,
            end_left,
            end_right,
            rotation_mat,
            width,
        );

        if let MinMaxResult::MinMax(_, _) = start_connections {
            self.generate_start_connection(*segment);
        }

        if let MinMaxResult::MinMax(_, _) = end_connections {
            self.generate_end_connection(*segment, rotation_mat, width);
        }
    }

    fn generate_surface(
        &mut self,
        segment: Segment,
        start_left: Vec2,
        start_right: Vec2,
        end_left: Vec2,
        end_right: Vec2,
        rotation_mat: Mat2,
        width: f32,
    ) {
        // To avoid interfering with the ground.
        self.positions.push([start_left.x, HEIGHT, start_left.y]);
        self.positions.push([start_right.x, HEIGHT, start_right.y]);
        self.positions.push([end_right.x, HEIGHT, end_right.y]);
        self.positions.push([end_left.x, HEIGHT, end_left.y]);

        // Road UV on X axis should go from 0.0 to 1.0.
        // But on Y we use segment length divided by width to scale it properly.
        self.uvs
            .push([0.0, (rotation_mat * (start_left - segment.start)).y / width]);
        self.uvs.push([
            1.0,
            (rotation_mat * (start_right - segment.start)).y / width,
        ]);
        self.uvs
            .push([1.0, (rotation_mat * (end_right - segment.start)).y / width]);
        self.uvs
            .push([0.0, (rotation_mat * (end_left - segment.start)).y / width]);

        self.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

        self.indices.push(0);
        self.indices.push(3);
        self.indices.push(1);
        self.indices.push(1);
        self.indices.push(3);
        self.indices.push(2);
    }

    /// Inside triangle to fill the gap between 3+ walls.
    fn generate_start_connection(&mut self, segment: Segment) {
        let vertices_start = self.vertices_count();

        self.positions
            .push([segment.start.x, HEIGHT, segment.start.y]);
        self.uvs.push([0.5, 0.0]);
        self.normals.push([0.0, 1.0, 0.0]);

        self.indices.push(1);
        self.indices.push(vertices_start);
        self.indices.push(0);
    }

    /// Inside triangle to fill the gap between 3+ walls.
    fn generate_end_connection(&mut self, segment: Segment, rotation_mat: Mat2, width: f32) {
        let vertices_start = self.vertices_count();

        self.positions.push([segment.end.x, HEIGHT, segment.end.y]);
        self.uvs.push([
            0.5,
            (rotation_mat * (segment.end - segment.start)).y / width,
        ]);
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
