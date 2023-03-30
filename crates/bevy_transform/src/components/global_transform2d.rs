use std::ops::Mul;

use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::{Affine2, Affine3A, Mat3, Mat4, Vec2, Vec3};
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect};

use crate::prelude::Transform2d;

#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, FromReflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component, Default, PartialEq)]
pub struct GlobalTransform2d {
    affine: Affine2,
    z_translation: f32,
}

impl Default for GlobalTransform2d {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl GlobalTransform2d {
    pub const IDENTITY: Self = GlobalTransform2d {
        affine: Affine2::IDENTITY,
        z_translation: 0.0,
    };

    #[doc(hidden)]
    #[inline]
    pub fn from_translation(translation: Vec2) -> Self {
        GlobalTransform2d {
            affine: Affine2::from_translation(translation),
            z_translation: 0.,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_translation_3d(translation: Vec3) -> Self {
        GlobalTransform2d {
            affine: Affine2::from_translation(translation.truncate()),
            z_translation: translation.z,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_rotation(rotation: f32) -> Self {
        GlobalTransform2d {
            affine: Affine2::from_angle_translation(rotation, Vec2::ZERO),
            z_translation: 0.0,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_scale(scale: Vec2) -> Self {
        GlobalTransform2d {
            affine: Affine2::from_scale(scale),
            z_translation: 0.0,
        }
    }

    /// Get the translation as a [`Vec3`].
    #[inline]
    pub fn translation(&self) -> Vec3 {
        self.affine.translation.extend(self.z_translation)
    }

    /// Transforms the given `point`, applying shear, scale, rotation and translation.
    ///
    /// This moves `point` into the local space of this [`GlobalTransform`].
    #[inline]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        let xy = point.truncate();

        self.affine
            .transform_point2(xy)
            .extend(point.z + self.z_translation)
    }

    /// Returns the 3d affine transformation matrix as a [`Mat4`].
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        let mat3 = Mat3::from_cols_array_2d(&[
            self.affine.matrix2.x_axis.extend(0.).to_array(),
            self.affine.matrix2.y_axis.extend(0.).to_array(),
            [0., 0., 1.],
        ]);

        Mat4::from(Affine3A::from_mat3_translation(
            mat3,
            self.affine.translation.extend(self.z_translation),
        ))
    }
}

impl From<Transform2d> for GlobalTransform2d {
    fn from(transform: Transform2d) -> Self {
        Self {
            affine: transform.compute_affine(),
            z_translation: transform.z_translation,
        }
    }
}

impl Mul<GlobalTransform2d> for GlobalTransform2d {
    type Output = GlobalTransform2d;

    #[inline]
    fn mul(self, other: GlobalTransform2d) -> Self::Output {
        GlobalTransform2d {
            affine: self.affine * other.affine,
            z_translation: self.z_translation + other.z_translation,
        }
    }
}
