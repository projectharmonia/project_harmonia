use bevy::{
    prelude::*,
    render::mesh::{Indices, VertexAttributeValues},
};

use crate::game_world::building::spline::{PointKind, SplineConnections, SplineSegment};

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
        let width_disp = disp.perp().normalize() * half_width;

        let start_connections = connections.minmax_angles(disp, PointKind::Start);
        let (start_left, start_right) =
            segment.offset_points(width_disp, half_width, start_connections);

        let end_connections = connections.minmax_angles(-disp, PointKind::End);
        let (end_right, end_left) =
            segment
                .inverse()
                .offset_points(-width_disp, half_width, end_connections);

        const HEIGHT: f32 = 0.001; // To avoid interfering with the ground.
        self.positions.push([start_left.x, HEIGHT, start_left.y]);
        self.positions.push([start_right.x, HEIGHT, start_right.y]);
        self.positions.push([end_right.x, HEIGHT, end_right.y]);
        self.positions.push([end_left.x, HEIGHT, end_left.y]);

        let repeats = disp.length() / (half_width * 2.0);
        self.uvs.push([0.0, 0.0]);
        self.uvs.push([1.0, 0.0]);
        self.uvs.push([1.0, repeats]);
        self.uvs.push([0.0, repeats]);

        self.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

        self.indices.push(0);
        self.indices.push(3);
        self.indices.push(1);
        self.indices.push(1);
        self.indices.push(3);
        self.indices.push(2);
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
