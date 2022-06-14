use bevy_math::{Mat4, Vec4};

use crate::view::ExtractedView;

/// A distance calculator for the draw order of [PhaseItems](crate::render_phase::PhaseItem).
pub struct ViewRangefinder3d {
    inverse_view_row_2: Vec4,
}

impl ViewRangefinder3d {
    /// Creates a rangefinder for a view
    pub fn from_view(view: &ExtractedView) -> ViewRangefinder3d {
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        ViewRangefinder3d {
            inverse_view_row_2: inverse_view_matrix.row(2),
        }
    }

    /// Calculates the distance, or view-space Z value, for a transform
    pub fn distance(&self, transform: &Mat4) -> f32 {
        // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
        // gives the z component of translation of the mesh in view-space
        self.inverse_view_row_2.dot(transform.col(3))
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::{Mat4, Vec3};
    use bevy_transform::prelude::Transform;

    use crate::view::ExtractedView;

    use super::ViewRangefinder3d;

    #[test]
    fn distance() {
        let view = ExtractedView {
            projection: Mat4::IDENTITY,
            transform: Transform::identity()
                .with_translation(Vec3::new(0.0, 0.0, -1.0))
                .into(),
            width: 0,
            height: 0,
        };
        let rangefinder = ViewRangefinder3d::from_view(&view);
        assert_eq!(rangefinder.distance(&Mat4::IDENTITY), 1.0);
        assert_eq!(
            rangefinder.distance(&Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))),
            2.0
        );
    }
}
