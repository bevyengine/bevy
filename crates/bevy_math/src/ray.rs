use crate::Vec3;

/// A ray is an infinite line starting at `origin`, going in `direction`.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Ray {
    /// The origin of the ray.
    pub origin: Vec3,
    /// A normalized vector representing the direction of the ray.
    pub direction: Vec3,
}

impl Ray {
    /// Returns the distance to the plane if the ray intersects it.
    #[inline]
    pub fn intersect_plane(&self, plane_origin: Vec3, plane_normal: Vec3) -> Option<f32> {
        let denominator = plane_normal.dot(self.direction);
        if denominator.abs() > f32::EPSILON {
            let distance = (plane_origin - self.origin).dot(plane_normal) / denominator;
            if distance > f32::EPSILON {
                return Some(distance);
            }
        }
        None
    }

    /// Retrieve a point at the given distance along the ray.
    #[inline]
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_plane() {
        let ray = Ray {
            origin: Vec3::ZERO,
            direction: Vec3::Z,
        };

        // Orthogonal, and test that an inverse plane_normal has the same result
        assert_eq!(Some(1.), ray.intersect_plane(Vec3::Z, Vec3::Z));
        assert_eq!(Some(1.), ray.intersect_plane(Vec3::Z, Vec3::NEG_Z));
        assert_eq!(None, ray.intersect_plane(Vec3::NEG_Z, Vec3::Z));
        assert_eq!(None, ray.intersect_plane(Vec3::NEG_Z, Vec3::NEG_Z));

        // Diagonal
        assert_eq!(Some(1.), ray.intersect_plane(Vec3::Z, Vec3::ONE));
        assert_eq!(None, ray.intersect_plane(Vec3::NEG_Z, Vec3::ONE));

        // Parallel
        assert_eq!(None, ray.intersect_plane(Vec3::X, Vec3::X));

        // Parallel with simulated rounding error
        assert_eq!(
            None,
            ray.intersect_plane(Vec3::X, Vec3::X + Vec3::Z * f32::EPSILON)
        );
    }
}
