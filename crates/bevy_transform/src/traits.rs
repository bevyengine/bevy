use bevy_math::{Affine3A, Mat4, Vec3};

use crate::prelude::{GlobalTransform, Transform};

/// A trait for point transformation methods.
pub trait TransformPoint {
    /// Transform a point.
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3;
}

impl TransformPoint for Transform {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into())
    }
}

impl TransformPoint for GlobalTransform {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into())
    }
}

impl TransformPoint for Mat4 {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point3(point.into())
    }
}

impl TransformPoint for Affine3A {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point3(point.into())
    }
}
