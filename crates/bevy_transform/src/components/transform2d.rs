use std::ops::Mul;

use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::{Affine2, Mat2, Mat3, Vec2, Vec3};
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect, ReflectFromReflect};
#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Describe the position of an entity. If the entity has a parent, the position is relative
/// to its parent position.
///
/// * To place or move an entity, you should set its [`Transform2d`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform2d`].
/// * To be displayed, an entity must have both a [`Transform2d`] and a [`GlobalTransform2d`].
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
/// update the [`Transform2d`] of an entity during this set or after, you will notice a 1 frame lag
/// before the [`GlobalTransform2d`] is updated.
///
/// [`GlobalTransform2d`]: super::GlobalTransform2d
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, FromReflect)]
#[reflect(Component, Default, PartialEq, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Transform2d {
    /// The translation along the `X` and `Y` axes.
    pub translation: Vec2,
    /// The rotation in radians. Positive values rotate anti-clockwise.
    pub rotation: f32,
    /// The scale along the `X` and `Y` axes.
    pub scale: Vec2,
    /// The translation along the `Z` axis.
    ///
    /// You might be surprised that 2D entities have a translation along the Z axis,
    /// but this third dimension is used when rendering to decide what should appear in front or behind.
    /// A higher translation on the Z axis puts the entity closer to the camera, and thus in front of entities with a lower Z translation.
    ///
    /// Keep in mind that this is relative to the [`Parent`](bevy_hierarchy::Parent)'s `z_translation`.
    /// The other fields on [`Transform2d`] don't affect this because they are strictly 2D.
    pub z_translation: f32,
}

impl Default for Transform2d {
    fn default() -> Self {
        Transform2d::IDENTITY
    }
}

impl Transform2d {
    /// Creates a new identity [`Transform2d`], with no translation, rotation, and a scale of 1 on all axes.
    ///
    /// Translation is `Vec2::ZERO`, rotation is `0.`, scale is `Vec2::ONE` and `z_translation` is `0.`.
    pub const IDENTITY: Self = Transform2d {
        translation: Vec2::ZERO,
        rotation: 0.,
        scale: Vec2::ONE,
        z_translation: 0.,
    };

    /// Creates a new [`Transform2d`] at the position `(x, y)`.
    ///
    /// Rotation will be `0.`, scale will be `Vec2::ONE` and `z_translation` will be `0.`.
    #[inline]
    pub const fn from_xy(x: f32, y: f32) -> Self {
        Transform2d::from_translation(Vec2::new(x, y))
    }

    /// Creates a new [`Transform2d`] at the position `(x, y, z)`. The `z` component
    /// is used for z-ordering elements: higher `z`-value will be in front of lower
    /// `z`-value.
    #[inline]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec2::new(x, y)).with_z_translation(z)
    }

    /// Creates a new [`Transform2d`] with `translation`.
    ///
    /// Rotation will be `0.`, scale will be `Vec2::ONE` and `z_translation` will be `0.`.
    #[inline]
    pub const fn from_translation(translation: Vec2) -> Self {
        Transform2d {
            translation,
            ..Self::IDENTITY
        }
    }

    /// Creates a new [`Transform2d`] with `translation`.
    ///
    /// Rotation will be `0.` and scale will be `Vec2::ONE`
    #[inline]
    pub const fn from_translation_3d(Vec3 { x, y, z }: Vec3) -> Self {
        Transform2d {
            translation: Vec2 { x, y },
            z_translation: z,
            ..Self::IDENTITY
        }
    }

    /// Creates a new [`Transform2d`] with `rotation`.
    ///
    /// Translation will be `Vec2::ZERO`, scale will be `Vec2::ONE` and `z_translation` will be `0.`.
    #[inline]
    pub const fn from_rotation(rotation: f32) -> Self {
        Transform2d {
            rotation,
            ..Self::IDENTITY
        }
    }

    /// Creates a new [`Transform2d`] with `scale`.
    ///
    /// Translation will be `Vec2::ZERO`, rotation will be `0.` and `z_translation` will be `0.`
    #[inline] // Hmm const
    pub const fn from_scale(scale: Vec2) -> Self {
        Transform2d {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Returns this [`Transform2d`] with a new translation.
    #[must_use]
    #[inline]
    pub const fn with_translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Returns this [`Transform2d`] with a new rotation.
    #[must_use]
    #[inline]
    pub const fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Returns this [`Transform2d`] with a new scale.
    #[must_use]
    #[inline]
    pub const fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Returns this [`Transform2d`] with a new Z translation.
    #[must_use]
    #[inline]
    pub const fn with_z_translation(mut self, z_translation: f32) -> Self {
        self.z_translation = z_translation;
        self
    }

    /// Returns this [`Transform2d`] rotated so the local `direction` points in the given `target_direction`.
    ///
    /// # Example
    /// ```
    /// # use bevy_transform::prelude::*;
    /// # use bevy_math::prelude::*;
    /// // Create a transform rotated so that up(Vec2::Y) points to the right.
    /// let transform = Transform2d::IDENTITY.pointed_to(Vec2::Y, Vec2::X);
    ///
    /// approx::assert_abs_diff_eq!(transform.up(), Vec2::X);
    /// ```
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn pointed_to(mut self, direction: Vec2, target_direction: Vec2) -> Self {
        self.point_to(direction, target_direction);
        self
    }

    /// Returns this [`Transform2d`] rotated so the local `direction` points at the given `target_position`.
    ///
    /// # Example
    /// ```
    /// # use bevy_transform::prelude::*;
    /// # use bevy_math::prelude::*;
    /// // Create a transform that is translated to Vec2::ONE and then rotated so that up(Vec2::Y) points to the origin.
    /// let transform = Transform2d::from_translation(Vec2::ONE)
    ///     .pointed_at(Vec2::Y, Vec2::ZERO);
    ///
    /// approx::assert_abs_diff_eq!(transform.up(), Vec2::NEG_ONE.normalize());
    /// ```
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn pointed_at(mut self, direction: Vec2, target_position: Vec2) -> Self {
        self.point_at(direction, target_position);
        self
    }

    /// Rotates this [`Transform2d`] so the local `direction` points in the given `target_direction`.
    ///
    /// # Example
    /// ```
    /// # use bevy_transform::prelude::*;
    /// # use bevy_math::prelude::*;
    /// let mut transform = Transform2d::IDENTITY;
    ///
    /// // Rotate the transform so that up(Vec2::Y) points to the right.
    /// transform.point_to(Vec2::Y, Vec2::X);
    ///
    /// approx::assert_abs_diff_eq!(transform.up(), Vec2::X);
    /// ```
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn point_to(&mut self, direction: Vec2, target_direction: Vec2) {
        self.rotation = Vec2::angle_between(direction, target_direction);
    }

    /// Rotates this [`Transform2d`] so the local `direction` points at the given `target_position`.
    ///
    /// # Example
    /// ```
    /// # use bevy_transform::prelude::*;
    /// # use bevy_math::prelude::*;
    /// let mut transform = Transform2d::from_translation(Vec2::ONE);
    ///
    /// // Rotate the transform so that `up`/local_y points to the origin.
    /// transform.point_at(Vec2::Y, Vec2::ZERO);
    ///
    /// approx::assert_abs_diff_eq!(transform.up(), Vec2::NEG_ONE.normalize());
    /// ```
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn point_at(&mut self, direction: Vec2, target_position: Vec2) {
        self.point_to(direction, target_position - self.translation);
    }

    /// Get the unit vector in the local `X` direction.
    #[inline]
    pub fn local_x(&self) -> Vec2 {
        let (sin, cos) = self.rotation.sin_cos();
        (cos, sin).into()
    }

    #[inline]
    /// Equivalent to [`-local_x()`][Self::local_x()]
    pub fn left(&self) -> Vec2 {
        -self.local_x()
    }

    #[inline]
    /// Equivalent to [`local_x()`][Self::local_x()]
    pub fn right(&self) -> Vec2 {
        self.local_x()
    }

    /// Get the unit vector in the local `Y` direction.
    #[inline]
    pub fn local_y(&self) -> Vec2 {
        let (sin, cos) = self.rotation.sin_cos();
        (-sin, cos).into()
    }

    /// Equivalent to [`local_y()`][Self::local_y]
    #[inline]
    pub fn up(&self) -> Vec2 {
        self.local_y()
    }

    /// Equivalent to [`-local_y()`][Self::local_y]
    #[inline]
    pub fn down(&self) -> Vec2 {
        -self.local_y()
    }

    /// Returns the rotation matrix from this transforms rotation.
    #[inline]
    pub fn rotation_matrix(&self) -> Mat2 {
        Mat2::from_angle(self.rotation)
    }

    /// Computes the affine transformation matrix of this transform.
    #[inline]
    pub fn compute_matrix(&self) -> Mat3 {
        Mat3::from_scale_angle_translation(self.scale, self.rotation, self.translation)
    }

    /// Computes the affine transform of this transform.
    #[inline]
    pub fn compute_affine(&self) -> Affine2 {
        Affine2::from_scale_angle_translation(self.scale, self.rotation, self.translation)
    }

    /// Translates this [`Transform2d`] around a `point` in space.
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn translate_around(&mut self, point: Vec2, angle: f32) {
        self.translation = point + Mat2::from_angle(angle) * (self.translation - point);
    }

    /// Rotates this [`Transform2d`] around a `point` in space.
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn rotate_around(&mut self, point: Vec2, angle: f32) {
        self.translate_around(point, angle);
        self.rotation += angle;
    }

    /// Transforms the given `point`, applying scale, rotation and translation.
    /// `z_translation` is ignored.
    ///
    /// If this [`Transform2d`] has a parent, this will transform a `point` that is
    /// relative to the parent's [`Transform2d`] into one relative to this [`Transform2d`].
    ///
    /// If this [`Transform2d`] does not have a parent, this will transform a `point`
    /// that is in global space into one relative to this [`Transform2d`].
    ///
    /// If you want to transform a `point` in global space to the local space of this [`Transform2d`],
    /// consider using [`GlobalTransform2d::transform_point()`](super::GlobalTransform2d::transform_point) instead.
    #[inline]
    pub fn transform_point(&self, mut point: Vec2) -> Vec2 {
        point *= self.scale;
        point = self.rotation_matrix() * point;
        point += self.translation;
        point
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`Transform2d`]
    #[inline]
    #[must_use]
    pub fn mul_transform(&self, transform: Transform2d) -> Self {
        let translation = self.transform_point(transform.translation);
        let rotation = self.rotation + transform.rotation;
        let scale = self.scale * transform.scale;
        let z_translation = self.z_translation + transform.z_translation;
        Transform2d {
            translation,
            rotation,
            scale,
            z_translation,
        }
    }
}

impl Mul<Transform2d> for Transform2d {
    type Output = Transform2d;

    fn mul(self, transform: Transform2d) -> Self::Output {
        self.mul_transform(transform)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    #[test]
    fn local_vectors() {
        let mut transform = Transform2d::from_rotation(TAU / 2.44);
        assert_eq!(transform.local_y(), transform.rotation_matrix() * Vec2::Y);
        assert_eq!(transform.local_x(), transform.rotation_matrix() * Vec2::X);
        transform.rotation = TAU / -0.56;
        assert_eq!(transform.local_y(), transform.rotation_matrix() * Vec2::Y);
        assert_eq!(transform.local_x(), transform.rotation_matrix() * Vec2::X);
    }
}
