use bevy_math::{Mat4, Vec3, Vec4};

/// A depth calculator for the draw order of [`PhaseItem`](crate::render_phase::PhaseItem)s.
pub struct ViewRangefinder3d {
    view_proj_row_2: Vec4,
    view_proj_row_3: Vec4,
}

impl ViewRangefinder3d {
    /// Creates a 3D rangefinder for a view-projection matrix.
    pub fn from_view_proj_matrix(view_proj_matrix: &Mat4) -> ViewRangefinder3d {
        ViewRangefinder3d {
            view_proj_row_2: view_proj_matrix.row(2),
            view_proj_row_3: view_proj_matrix.row(3),
        }
    }

    /// Calculates the depth for the given `translation`.
    #[inline]
    pub fn distance_translation(&self, translation: &Vec3) -> f32 {
        // NOTE: row 2 of the view-projection matrix dotted with the translation from the model matrix
        // gives the z component of translation of the mesh in clip-space
        self.view_proj_row_2.dot(translation.extend(1.0)) / self.view_proj_row_3.dot(translation.extend(1.0))
    }

    /// Calculates the depth for the given `transform`.
    #[inline]
    pub fn distance(&self, transform: &Mat4) -> f32 {
        // NOTE: row 2 of the view-projection matrix dotted with column 3 of the model matrix
        // gives the z component of translation of the mesh in clip-space
        self.view_proj_row_2.dot(transform.col(3)) / self.view_proj_row_3.dot(transform.col(3))
    }
}

#[cfg(test)]
mod tests {
    use super::ViewRangefinder3d;
    use bevy_math::{Mat4, Vec3};

    #[test]
    fn distance() {
        let view_proj_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -1.0));
        let rangefinder = ViewRangefinder3d::from_view_proj_matrix(&view_proj_matrix);
        assert_eq!(rangefinder.distance(&Mat4::IDENTITY), 1.0);
        assert_eq!(
            rangefinder.distance(&Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))),
            2.0
        );
    }
}
