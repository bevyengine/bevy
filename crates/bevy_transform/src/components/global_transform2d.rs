use std::ops::Mul;

use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::{Affine2, Affine3A, Mat2, Mat3, Mat4, Vec2, Vec3};
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect, ReflectFromReflect};
#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::components::Transform2d;

/// Describe the position of an entity relative to the reference frame.
///
/// * To place or move an entity, you should set its [`Transform2d`].
/// * [`GlobalTransform2d`] is fully managed by bevy, you cannot mutate it, use
///   [`Transform2d`] instead.
/// * To get the global transform of an entity, you should get its [`GlobalTransform2d`].
/// * For transform hierarchies to work correctly, you must have both a [`Transform2d`] and a [`GlobalTransform2d`].
///   * You may use the [`TransformBundle`](crate::TransformBundle) to guarantee this.
///
/// ## [`Transform2d`] and [`GlobalTransform2d`]
///
/// [`Transform2d`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a [`Parent`](bevy_hierarchy::Parent).
///
/// [`GlobalTransform2d`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform2d`] is updated from [`Transform2d`] by systems in the system set
/// [`TransformPropagate`](crate::TransformSystem::TransformPropagate).
///
/// This system runs during [`PostUpdate`](bevy_app::PostUpdate). If you
/// update the [`Transform2d`] of an entity in this schedule or after, you will notice a 1 frame lag
/// before the [`GlobalTransform2d`] is updated.
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, FromReflect)]
#[reflect(Component, Default, PartialEq, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
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
    /// An identity [`GlobalTransform2d`] that maps all points in space to themselves.
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
    /// This moves the `point` from the local space of this entity into global space.
    #[inline]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        let xy = point.truncate();

        self.affine
            .transform_point2(xy)
            .extend(point.z + self.z_translation)
    }

    /// Returns the 2d affine transformation matrix as a [`Mat4`].
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

    /// Returns the 2d affine transformation matrix as an [`Affine2`].
    #[inline]
    pub fn affine(&self) -> Affine2 {
        self.affine
    }

    /// Returns the translation on the Z axis.
    #[inline]
    pub fn z_translation(&self) -> f32 {
        self.z_translation
    }

    /// Returns the transformation as a [`Transform2d`].
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output
    /// will be invalid.
    #[inline]
    pub fn compute_transform(&self) -> Transform2d {
        let affine = self.affine;

        let det = affine.matrix2.determinant();

        let scale = Vec2::new(
            affine.matrix2.x_axis.length() * det.signum(),
            affine.matrix2.y_axis.length(),
        );

        let inv_scale = scale.recip();

        let rotation_matrix = Mat2::from_cols(
            affine.matrix2.x_axis * inv_scale.x,
            affine.matrix2.y_axis * inv_scale.y,
        );
        let rotation = Vec2::angle_between(Vec2::Y, rotation_matrix * Vec2::Y);

        Transform2d {
            translation: affine.translation,
            rotation,
            scale,
            z_translation: 0.,
        }
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
