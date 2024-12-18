use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};

#[derive(Default)]
pub(super) struct DynamicMesh {
    pub(super) positions: Vec<[f32; 3]>,
    pub(super) uvs: Vec<[f32; 2]>,
    pub(super) normals: Vec<[f32; 3]>,
    pub(super) indices: Vec<u32>,
}

impl DynamicMesh {
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

    pub(super) fn vertices_count(&self) -> u32 {
        self.positions
            .len()
            .try_into()
            .expect("vertices should fit u32")
    }

    pub(super) fn clear(&mut self) {
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

    /// Returns a mesh with attributes required by [`DynamicMesh`].
    pub(super) fn create_empty() -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, Default::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<Vec3>::new())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<Vec2>::new())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
            .with_inserted_indices(Indices::U32(Vec::new()))
    }
}
