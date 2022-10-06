use crate::Vec3;
use serde::{Deserialize, Serialize};

/// A ray is an infinite line starting at `origin`, going in `direction`.
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ray {
    /// The origin of the ray.
    pub origin: Vec3,
    /// The direction of the ray.
    pub direction: Vec3,
}

impl Ray {
    /// Returns true if this ray intersects the plane.
    pub fn intersects_plane(&self, plane_origin: Vec3, plane_normal: Vec3) -> bool {
        let denom = plane_normal.dot(self.direction);
        if denom.abs() > f32::EPSILON {
            let t = (plane_origin - self.origin).dot(plane_normal) / denom;
            if t >= f32::EPSILON {
                return true;
            }
        }
        false
    }
    
    /// Retrieve a point at the given distance along the ray.
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn intersects_plane() {
        let ray = Ray {
            origin: Vec3::ZERO,
            direction: Vec3::Z,
        };

        // Orthogonal
        assert!(ray.intersects_plane(Vec3::Z, Vec3::Z));
        assert!(ray.intersects_plane(Vec3::Z, Vec3::NEG_Z));
        assert!(!ray.intersects_plane(Vec3::NEG_Z, Vec3::Z));
        assert!(!ray.intersects_plane(Vec3::NEG_Z, Vec3::NEG_Z));

        // Diagonal
        assert!(ray.intersects_plane(Vec3::Z, Vec3::ONE));
        assert!(ray.intersects_plane(Vec3::Z, Vec3::NEG_ONE));
        assert!(!ray.intersects_plane(Vec3::NEG_Z, Vec3::ONE));
        assert!(!ray.intersects_plane(Vec3::NEG_Z, Vec3::NEG_ONE));
        
        // Parralel
        assert!(!ray.intersects_plane(Vec3::X, Vec3::X));
        assert!(!ray.intersects_plane(Vec3::X, Vec3::NEG_X));
        assert!(!ray.intersects_plane(Vec3::NEG_X, Vec3::X));
        assert!(!ray.intersects_plane(Vec3::NEG_X, Vec3::NEG_X));

        // Parralel with simulated rounding error
        assert!(!ray.intersects_plane(Vec3::X, Vec3::X + Vec3::Z * f32::EPSILON));
        assert!(!ray.intersects_plane(Vec3::NEG_X, Vec3::X + Vec3::NEG_Z * f32::EPSILON));
    }
}
