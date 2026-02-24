use bevy_math::{Affine3A, Mat4, Vec3, Vec4};

/// A distance calculator for the draw order of [`PhaseItem`](crate::render_phase::PhaseItem)s.
pub struct ViewRangefinder3d {
    view_from_world_row_2: Vec4,
}

impl ViewRangefinder3d {
    /// Creates a 3D rangefinder for a view matrix.
    pub fn from_world_from_view(world_from_view: &Affine3A) -> ViewRangefinder3d {
        let view_from_world = world_from_view.inverse();

        ViewRangefinder3d {
            view_from_world_row_2: Mat4::from(view_from_world).row(2),
        }
    }

    /// Calculates the distance, or view-space `Z` value, for the given world-space `position`.
    #[inline]
    pub fn distance(&self, position: &Vec3) -> f32 {
        // NOTE: row 2 of the inverse view matrix dotted with the world-space position
        // gives the z component of the point in view-space
        self.view_from_world_row_2.dot(position.extend(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::ViewRangefinder3d;
    use bevy_math::{Affine3A, Vec3};

    #[test]
    fn distance() {
        let view_matrix = Affine3A::from_translation(Vec3::new(0.0, 0.0, -1.0));
        let rangefinder = ViewRangefinder3d::from_world_from_view(&view_matrix);
        assert_eq!(rangefinder.distance(&Vec3::new(0., 0., 0.)), 1.0);
        assert_eq!(rangefinder.distance(&Vec3::new(0., 0., 1.)), 2.0);
    }
}
