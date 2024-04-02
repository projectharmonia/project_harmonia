use earcut_rs::Earcut;

/// A small wrapper around [`Earcut`] to reuse allocated memory.
#[derive(Default)]
pub(crate) struct Triangulator {
    earcut: Earcut<f32>,
    hole_indices: Vec<u32>,
    indices: Vec<u32>,
    inverse_winding: bool,
}

impl Triangulator {
    pub(super) fn set_inverse_winding(&mut self, inverse_winding: bool) {
        self.inverse_winding = inverse_winding;
    }

    pub(super) fn triangulate(&mut self, positions: &[[f32; 3]]) -> &[u32] {
        self.earcut.earcut(
            positions.iter().flat_map(|&[x, y, _]| [x, y]),
            &self.hole_indices,
            &mut self.indices,
        );

        self.hole_indices.clear();

        if self.inverse_winding {
            for triangle in self.indices.chunks_exact_mut(3) {
                triangle.swap(0, 2);
            }
        }

        &self.indices
    }

    pub(super) fn add_hole(&mut self, index: u32) {
        self.hole_indices.push(index);
    }
}
