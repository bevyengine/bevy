use crate::{Vec3, Vec3A};

/// An axis-aligned bounding (AABB) box in 3D.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Aabb {
    /// The center of the box.
    pub center: Vec3A,
    /// The half-extents of the box along each axis.
    ///
    /// The box size is the double of this.
    pub half_extents: Vec3A,
}

impl Aabb {
    /// Create an AABB from its minimum and maximum corners.
    #[inline]
    pub fn from_min_max(minimum: Vec3, maximum: Vec3) -> Self {
        let minimum = Vec3A::from(minimum);
        let maximum = Vec3A::from(maximum);
        let center = 0.5 * (maximum + minimum);
        let half_extents = 0.5 * (maximum - minimum);
        Self {
            center,
            half_extents,
        }
    }

    /// Calculate the relative radius of the AABB with respect to a plane.
    #[inline]
    pub fn relative_radius(&self, p_normal: &Vec3A, axes: &[Vec3A]) -> f32 {
        // NOTE: dot products on Vec3A use SIMD and even with the overhead of conversion are net faster than Vec3
        let half_extents = self.half_extents;
        Vec3A::new(
            p_normal.dot(axes[0]),
            p_normal.dot(axes[1]),
            p_normal.dot(axes[2]),
        )
        .abs()
        .dot(half_extents)
    }

    /// Get the minimum corner of the AABB.
    #[inline]
    pub fn min(&self) -> Vec3A {
        self.center - self.half_extents
    }

    /// Get the maximum corner of the AABB.
    #[inline]
    pub fn max(&self) -> Vec3A {
        self.center + self.half_extents
    }
}
