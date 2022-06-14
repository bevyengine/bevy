use bevy_math::{Mat4, Vec4};

use crate::view::ExtractedView;

pub struct ViewRangefinder3d {
    inverse_view_row_2: Vec4,
}

/// A helper for calculating the draw order of meshes.
impl ViewRangefinder3d {
    pub fn from_view(view: &ExtractedView) -> ViewRangefinder3d {
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        ViewRangefinder3d {
            inverse_view_row_2: inverse_view_matrix.row(2),
        }
    }

    /// Calculates the view-space Z value for a mesh's origin
    pub fn distance(&self, mesh_transform: &Mat4) -> f32 {
        // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
        // gives the z component of translation of the mesh in view-space
        self.inverse_view_row_2.dot(mesh_transform.col(3))
    }
}
