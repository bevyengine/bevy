use bevy_math::{Mat4, Vec3, Vec4};

/// A distance calculator for the draw order of [`PhaseItem`](crate::render_phase::PhaseItem)s.
pub struct ViewRangefinder3d {
    view_from_world_row_2: Vec4,
}

impl ViewRangefinder3d {
    /// Creates a 3D rangefinder for a view matrix.
    pub fn from_world_from_view(world_from_view: &Mat4) -> ViewRangefinder3d {
        let view_from_world = world_from_view.inverse();

        ViewRangefinder3d {
            view_from_world_row_2: view_from_world.row(2),
        }
    }

    /// Calculates the distance, or view-space `Z` value, for the given `translation`.
    #[inline]
    pub fn distance_translation(&self, translation: &Vec3) -> f32 {
        // NOTE: row 2 of the inverse view matrix dotted with the translation from the model matrix
        // gives the z component of translation of the mesh in view-space
        self.view_from_world_row_2.dot(translation.extend(1.0))
    }

    /// Calculates the distance, or view-space `Z` value, for the given `transform`.
    #[inline]
    pub fn distance(&self, transform: &Mat4) -> f32 {
        // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
        // gives the z component of translation of the mesh in view-space
        self.view_from_world_row_2.dot(transform.col(3))
    }
}

#[cfg(test)]
mod tests {
    use super::ViewRangefinder3d;
    use bevy_math::{Mat4, Vec3};

    #[test]
    fn distance() {
        let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -1.0));
        let rangefinder = ViewRangefinder3d::from_world_from_view(&view_matrix);
        assert_eq!(rangefinder.distance(&Mat4::IDENTITY), 1.0);
        assert_eq!(
            rangefinder.distance(&Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))),
            2.0
        );
    }
}
