use bevy_math::{Affine2, Affine3A, Isometry2d, Isometry3d, Mat3, Mat3A, Mat4, Vec2, Vec3};

use crate::prelude::{GlobalTransform, Transform2d, Transform3d};

/// A trait for point transformation methods.
pub trait TransformPoint<T = Vec3> {
    /// Transform a point.
    fn transform_point(&self, point: impl Into<T>) -> T;
}

impl TransformPoint<Vec3> for Transform3d {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into())
    }
}

impl TransformPoint<Vec3> for GlobalTransform {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into())
    }
}

impl TransformPoint<Vec3> for Mat4 {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point3(point.into())
    }
}

impl TransformPoint<Vec3> for Affine3A {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point3(point.into())
    }
}

impl TransformPoint<Vec3> for Isometry3d {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into()).into()
    }
}

impl TransformPoint<Vec2> for Transform2d {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec2>) -> Vec2 {
        self.transform_point(point.into())
    }
}

impl TransformPoint<Vec2> for Mat3 {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec2>) -> Vec2 {
        self.transform_point2(point.into())
    }
}

impl TransformPoint<Vec2> for Mat3A {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec2>) -> Vec2 {
        self.transform_point2(point.into())
    }
}

impl TransformPoint<Vec2> for Affine2 {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec2>) -> Vec2 {
        self.transform_point2(point.into())
    }
}

impl TransformPoint<Vec2> for Isometry2d {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec2>) -> Vec2 {
        self.transform_point(point.into())
    }
}
