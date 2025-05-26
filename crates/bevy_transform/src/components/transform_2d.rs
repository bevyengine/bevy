use super::{GlobalTransform, TransformTreeChanged};
use bevy_math::{Affine2, Affine3A, Dir2, Isometry2d, Mat3, Quat, Rot2, Vec2, Vec3};
use core::ops::Mul;

#[cfg(feature = "bevy-support")]
use bevy_ecs::component::Component;

#[cfg(feature = "bevy_reflect")]
use {bevy_ecs::reflect::ReflectComponent, bevy_reflect::prelude::*};

/// TODO
#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy-support",
    derive(Component),
    require(GlobalTransform, TransformTreeChanged)
)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, PartialEq, Debug, Clone)
)]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize)
)]
pub struct Transform2d {
    /// TODO
    pub translation: Vec2,
    /// TODO
    pub rotation: Rot2,
    /// TODO
    pub scale: f32,
}

impl Transform2d {
    /// TODO
    pub const IDENTITY: Self = Transform2d {
        translation: Vec2::ZERO,
        rotation: Rot2::IDENTITY,
        scale: 1.0,
    };

    /// TODO
    #[inline]
    pub const fn from_xy(x: f32, y: f32) -> Self {
        Self::from_translation(Vec2::new(x, y))
    }

    /// TODO
    #[inline]
    pub const fn from_translation(translation: Vec2) -> Self {
        Transform2d {
            translation,
            ..Self::IDENTITY
        }
    }

    /// TODO
    #[inline]
    pub const fn from_rotation(rotation: Rot2) -> Self {
        Transform2d {
            rotation,
            ..Self::IDENTITY
        }
    }

    /// TODO
    #[inline]
    pub const fn from_scale(scale: f32) -> Self {
        Transform2d {
            scale,
            ..Self::IDENTITY
        }
    }

    /// TODO
    #[inline]
    pub fn from_isometry(iso: Isometry2d) -> Self {
        Transform2d {
            translation: iso.translation,
            rotation: iso.rotation,
            ..Self::IDENTITY
        }
    }

    /// Returns this [`Transform2d`] with a new translation.
    #[inline]
    #[must_use]
    pub const fn with_translation(mut self, translation: Vec2) -> Self {
        self.translation = translation;
        self
    }

    /// Returns this [`Transform2d`] with a new rotation.
    #[inline]
    #[must_use]
    pub const fn with_rotation(mut self, rotation: Rot2) -> Self {
        self.rotation = rotation;
        self
    }

    /// Returns this [`Transform2d`] with a new scale.
    #[inline]
    #[must_use]
    pub const fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Returns the 3d affine transformation matrix from this transforms translation,
    /// rotation, and scale.
    #[inline]
    pub fn compute_matrix(&self) -> Mat3 {
        Mat3::from_scale_angle_translation(
            Vec2::splat(self.scale),
            self.rotation.as_radians(),
            self.translation,
        )
    }

    /// Returns the 3d affine transformation matrix from this transforms translation,
    /// rotation, and scale.
    #[inline]
    pub fn compute_affine(&self) -> Affine2 {
        Affine2::from_scale_angle_translation(
            Vec2::splat(self.scale),
            self.rotation.as_radians(),
            self.translation,
        )
    }

    /// TODO
    #[inline]
    pub fn compute_affine3(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            Quat::from_rotation_z(self.rotation.as_radians()),
            Vec3::new(self.translation.x, self.translation.y, 0.0),
        )
    }

    /// Get the unit vector in the local `X` direction.
    #[inline]
    pub fn local_x(&self) -> Dir2 {
        Dir2::new_unchecked(self.rotation * Vec2::X)
    }

    /// Equivalent to [`-local_x()`][Transform2d::local_x()]
    #[inline]
    pub fn left(&self) -> Dir2 {
        -self.local_x()
    }

    /// Equivalent to [`local_x()`][Transform2d::local_x()]
    #[inline]
    pub fn right(&self) -> Dir2 {
        self.local_x()
    }

    /// Get the unit vector in the local `Y` direction.
    #[inline]
    pub fn local_y(&self) -> Dir2 {
        // Quat * unit vector is length 1
        Dir2::new_unchecked(self.rotation * Vec2::Y)
    }

    /// Equivalent to [`local_y()`][Transform2d::local_y]
    #[inline]
    pub fn up(&self) -> Dir2 {
        self.local_y()
    }

    /// Equivalent to [`-local_y()`][Transform2d::local_y]
    #[inline]
    pub fn down(&self) -> Dir2 {
        -self.local_y()
    }

    /// TODO
    #[inline]
    pub fn rotate(&mut self, rotation: Rot2) {
        self.rotation = rotation * self.rotation;
    }

    /// Translates this [`Transform2d`] around a `point` in space.
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn translate_around(&mut self, point: Vec2, rotation: Rot2) {
        self.translation = point + rotation * (self.translation - point);
    }

    /// Rotates this [`Transform2d`] around a `point` in space.
    ///
    /// If this [`Transform2d`] has a parent, the `point` is relative to the [`Transform2d`] of the parent.
    #[inline]
    pub fn rotate_around(&mut self, point: Vec2, rotation: Rot2) {
        self.translate_around(point, rotation);
        self.rotate(rotation);
    }

    /// TODO
    #[inline]
    pub fn look_at(&mut self, target: Vec2) {
        todo!()
    }

    /// TODO
    #[inline]
    pub fn look_to(&mut self, direction: impl TryInto<Dir2>) {
        todo!()
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`Transform2d`]
    #[inline]
    #[must_use]
    pub fn mul_transform(&self, transform: Transform2d) -> Self {
        let translation = self.transform_point(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Transform2d {
            translation,
            rotation,
            scale,
        }
    }

    /// TODO
    #[inline]
    pub fn transform_point(&self, mut point: Vec2) -> Vec2 {
        point = self.scale * point;
        point = self.rotation * point;
        point += self.translation;
        point
    }

    /// TODO
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.translation.is_finite() && self.rotation.is_finite() && self.scale.is_finite()
    }

    /// TODO
    #[inline]
    pub fn to_isometry(&self) -> Isometry2d {
        Isometry2d::new(self.translation, self.rotation)
    }
}

impl Default for Transform2d {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul<Transform2d> for Transform2d {
    type Output = Transform2d;

    fn mul(self, transform: Transform2d) -> Self::Output {
        self.mul_transform(transform)
    }
}

impl Mul<GlobalTransform> for Transform2d {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, global_transform: GlobalTransform) -> Self::Output {
        GlobalTransform::from(self) * global_transform
    }
}

impl Mul<Vec2> for Transform2d {
    type Output = Vec2;

    fn mul(self, value: Vec2) -> Self::Output {
        self.transform_point(value)
    }
}
