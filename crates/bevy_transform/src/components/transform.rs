use super::GlobalTransform;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::primitives::Direction3d;
use bevy_math::{Affine3A, Mat3, Mat4, Quat, Vec3};
use bevy_reflect::prelude::*;
use bevy_reflect::Reflect;
use std::ops::Mul;

/// Describe the position of an entity. If the entity has a parent, the position is relative
/// to its parent position.
///
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform`].
/// * To be displayed, an entity must have both a [`Transform`] and a [`GlobalTransform`].
///   * You may use the [`TransformBundle`](crate::TransformBundle) to guarantee this.
///
/// ## [`Transform`] and [`GlobalTransform`]
///
/// [`Transform`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a [`Parent`](bevy_hierarchy::Parent).
///
/// [`GlobalTransform`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform`] is updated from [`Transform`] by systems in the system set
/// [`TransformPropagate`](crate::TransformSystem::TransformPropagate).
///
/// This system runs during [`PostUpdate`](bevy_app::PostUpdate). If you
/// update the [`Transform`] of an entity during this set or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
///
/// # Examples
///
/// - [`transform`]
/// - [`global_vs_local_translation`]
///
/// [`global_vs_local_translation`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/global_vs_local_translation.rs
/// [`transform`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/transform.rs
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component, Default, PartialEq)]
pub struct Transform {
    /// Position of the entity. In 2d, the last value of the `Vec3` is used for z-ordering.
    ///
    /// See the [`translations`] example for usage.
    ///
    /// [`translations`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/translation.rs
    pub translation: Vec3,
    /// Rotation of the entity.
    ///
    /// See the [`3d_rotation`] example for usage.
    ///
    /// [`3d_rotation`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/3d_rotation.rs
    pub rotation: Quat,
    /// Scale of the entity.
    ///
    /// See the [`scale`] example for usage.
    ///
    /// [`scale`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/scale.rs
    pub scale: Vec3,
}

impl Transform {
    /// An identity [`Transform`] with no translation, rotation, and a scale of 1 on all axes.
    pub const IDENTITY: Self = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Creates a new [`Transform`] at the position `(x, y, z)`. In 2d, the `z` component
    /// is used for z-ordering elements: higher `z`-value will be in front of lower
    /// `z`-value.
    #[inline]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
    }

    /// Extracts the translation, rotation, and scale from `matrix`. It must be a 3d affine
    /// transformation matrix.
    #[inline]
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();

        Transform {
            translation,
            rotation,
            scale,
        }
    }

    /// Creates a new [`Transform`], with `translation`. Rotation will be 0 and scale 1 on
    /// all axes.
    #[inline]
    pub const fn from_translation(translation: Vec3) -> Self {
        Transform {
            translation,
            ..Self::IDENTITY
        }
    }

    /// Creates a new [`Transform`], with `rotation`. Translation will be 0 and scale 1 on
    /// all axes.
    #[inline]
    pub const fn from_rotation(rotation: Quat) -> Self {
        Transform {
            rotation,
            ..Self::IDENTITY
        }
    }

    /// Creates a new [`Transform`], with `scale`. Translation will be 0 and rotation 0 on
    /// all axes.
    #[inline]
    pub const fn from_scale(scale: Vec3) -> Self {
        Transform {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Returns this [`Transform`] with a new rotation so that [`Transform::forward`]
    /// points towards the `target` position and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `target` is the same as the transform translation, `Vec3::Z` is used instead
    /// * if `up` is zero, `Vec3::Y` is used instead
    /// * if the resulting forward direction is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    #[must_use]
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    /// Returns this [`Transform`] with a new rotation so that [`Transform::forward`]
    /// points in the given `direction` and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` is zero, `Vec3::Z` is used instead
    /// * if `up` is zero, `Vec3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    #[must_use]
    pub fn looking_to(mut self, direction: Vec3, up: Vec3) -> Self {
        self.look_to(direction, up);
        self
    }

    /// Returns this [`Transform`] with a rotation so that the `handle` vector, reinterpreted in local coordinates,
    /// points in the given `direction`, while `weak_handle` points towards `weak_direction`.
    ///
    /// In some cases a rotation cannot be constructed. Another axis will be picked in those cases:
    /// * if `handle` or `direction` is zero, `Vec3::X` takes its place
    /// * if `weak_handle` or `weak_direction` is zero, `Vec3::Y` takes its place
    /// * if `handle` is parallel with `weak_handle` or `direction` is parallel with `weak_direction`, a rotation is
    /// constructed which takes `handle` to `direction` but ignores the weak counterparts (i.e. is otherwise unspecified)
    ///
    /// See [`Transform::align`] for additional details.
    #[inline]
    #[must_use]
    pub fn aligned_by(
        mut self,
        handle: Vec3,
        direction: Vec3,
        weak_handle: Vec3,
        weak_direction: Vec3,
    ) -> Self {
        self.align(handle, direction, weak_handle, weak_direction);
        self
    }

    /// Returns this [`Transform`] with a new translation.
    #[inline]
    #[must_use]
    pub const fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    /// Returns this [`Transform`] with a new rotation.
    #[inline]
    #[must_use]
    pub const fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Returns this [`Transform`] with a new scale.
    #[inline]
    #[must_use]
    pub const fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Returns the 3d affine transformation matrix from this transforms translation,
    /// rotation, and scale.
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Returns the 3d affine transformation matrix from this transforms translation,
    /// rotation, and scale.
    #[inline]
    pub fn compute_affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Get the unit vector in the local `X` direction.
    #[inline]
    pub fn local_x(&self) -> Direction3d {
        // Direction3d::new(x) panics if x is of invalid length, but quat * unit vector is length 1
        Direction3d::new(self.rotation * Vec3::X).unwrap()
    }

    /// Equivalent to [`-local_x()`][Transform::local_x()]
    #[inline]
    pub fn left(&self) -> Direction3d {
        -self.local_x()
    }

    /// Equivalent to [`local_x()`][Transform::local_x()]
    #[inline]
    pub fn right(&self) -> Direction3d {
        self.local_x()
    }

    /// Get the unit vector in the local `Y` direction.
    #[inline]
    pub fn local_y(&self) -> Direction3d {
        // Direction3d::new(x) panics if x is of invalid length, but quat * unit vector is length 1
        Direction3d::new(self.rotation * Vec3::Y).unwrap()
    }

    /// Equivalent to [`local_y()`][Transform::local_y]
    #[inline]
    pub fn up(&self) -> Direction3d {
        self.local_y()
    }

    /// Equivalent to [`-local_y()`][Transform::local_y]
    #[inline]
    pub fn down(&self) -> Direction3d {
        -self.local_y()
    }

    /// Get the unit vector in the local `Z` direction.
    #[inline]
    pub fn local_z(&self) -> Direction3d {
        // Direction3d::new(x) panics if x is of invalid length, but quat * unit vector is length 1
        Direction3d::new(self.rotation * Vec3::Z).unwrap()
    }

    /// Equivalent to [`-local_z()`][Transform::local_z]
    #[inline]
    pub fn forward(&self) -> Direction3d {
        -self.local_z()
    }

    /// Equivalent to [`local_z()`][Transform::local_z]
    #[inline]
    pub fn back(&self) -> Direction3d {
        self.local_z()
    }

    /// Rotates this [`Transform`] by the given rotation.
    ///
    /// If this [`Transform`] has a parent, the `rotation` is relative to the rotation of the parent.
    ///
    /// # Examples
    ///
    /// - [`3d_rotation`]
    ///
    /// [`3d_rotation`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/3d_rotation.rs
    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = rotation * self.rotation;
    }

    /// Rotates this [`Transform`] around the given `axis` by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the `axis` is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_axis(&mut self, axis: Vec3, angle: f32) {
        self.rotate(Quat::from_axis_angle(axis, angle));
    }

    /// Rotates this [`Transform`] around the `X` axis by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the axis is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_x(angle));
    }

    /// Rotates this [`Transform`] around the `Y` axis by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the axis is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_y(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_y(angle));
    }

    /// Rotates this [`Transform`] around the `Z` axis by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the axis is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_z(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_z(angle));
    }

    /// Rotates this [`Transform`] by the given `rotation`.
    ///
    /// The `rotation` is relative to this [`Transform`]'s current rotation.
    #[inline]
    pub fn rotate_local(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    /// Rotates this [`Transform`] around its local `axis` by `angle` (in radians).
    #[inline]
    pub fn rotate_local_axis(&mut self, axis: Vec3, angle: f32) {
        self.rotate_local(Quat::from_axis_angle(axis, angle));
    }

    /// Rotates this [`Transform`] around its local `X` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_local_x(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_x(angle));
    }

    /// Rotates this [`Transform`] around its local `Y` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_local_y(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_y(angle));
    }

    /// Rotates this [`Transform`] around its local `Z` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_local_z(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_z(angle));
    }

    /// Translates this [`Transform`] around a `point` in space.
    ///
    /// If this [`Transform`] has a parent, the `point` is relative to the [`Transform`] of the parent.
    #[inline]
    pub fn translate_around(&mut self, point: Vec3, rotation: Quat) {
        self.translation = point + rotation * (self.translation - point);
    }

    /// Rotates this [`Transform`] around a `point` in space.
    ///
    /// If this [`Transform`] has a parent, the `point` is relative to the [`Transform`] of the parent.
    #[inline]
    pub fn rotate_around(&mut self, point: Vec3, rotation: Quat) {
        self.translate_around(point, rotation);
        self.rotate(rotation);
    }

    /// Rotates this [`Transform`] so that [`Transform::forward`] points towards the `target` position,
    /// and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `target` is the same as the transform translation, `Vec3::Z` is used instead
    /// * if `up` is zero, `Vec3::Y` is used instead
    /// * if the resulting forward direction is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.look_to(target - self.translation, up);
    }

    /// Rotates this [`Transform`] so that [`Transform::forward`] points in the given `direction`
    /// and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` is zero, `Vec3::NEG_Z` is used instead
    /// * if `up` is zero, `Vec3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    pub fn look_to(&mut self, direction: Vec3, up: Vec3) {
        let back = -direction.try_normalize().unwrap_or(Vec3::NEG_Z);
        let up = up.try_normalize().unwrap_or(Vec3::Y);
        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthonormal_vector());
        let up = back.cross(right);
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back));
    }

    /// Rotates this [`Transform`] so that the `handle` vector, reinterpreted in local coordinates, points
    /// in the given `direction`, while `weak_handle` points towards `weak_direction`.
    ///
    /// More precisely, the [`Transform::rotation`] produced will be such that:
    /// * applying it to `handle` produces `direction`
    /// * applying it to `weak_handle` produces a vector that lies in the half-plane generated by `direction` and
    /// `weak_direction` (with positive contribution by `weak_direction`)
    ///
    /// For example, [`Transform::look_to`] is recovered when `handle` is `Vec3::NEG_Z` (the [`Transform::forward`] direction
    /// in the default orientation) and `weak_handle` is `Vec3::Y` (the [`Transform::up`] direction in the default
    /// orientation). (Failure cases may differ somewhat.)
    ///
    /// In some cases a rotation cannot be constructed. Another axis will be picked in those cases:
    /// * if `handle` or `direction` is zero, `Vec3::X` takes its place
    /// * if `weak_handle` or `weak_direction` is zero, `Vec3::Y` takes its place
    /// * if `handle` is parallel with `weak_handle` or `direction` is parallel with `weak_direction`, a rotation is
    /// constructed which takes `handle` to `direction` but ignores the weak counterparts (i.e. is otherwise unspecified)
    #[inline]
    pub fn align(
        &mut self,
        handle: Vec3,
        direction: Vec3,
        weak_handle: Vec3,
        weak_direction: Vec3,
    ) {
        let handle = handle.try_normalize().unwrap_or(Vec3::X);
        let direction = direction.try_normalize().unwrap_or(Vec3::X);
        let weak_handle = weak_handle.try_normalize().unwrap_or(Vec3::Y);
        let weak_direction = weak_direction.try_normalize().unwrap_or(Vec3::Y);

        // The solution quaternion will be constructed in two steps.
        // First, we start with a rotation that takes `handle` to `direction`.
        let first_rotation = Quat::from_rotation_arc(handle, direction);

        // Let's follow by rotating about the `direction` axis so that the image of `weak_handle`
        // is taken to something that lies in the plane of `direction` and `weak_direction`. Since
        // `direction` is fixed by this rotation, the first criterion is still satisfied.
        let weak_image = first_rotation * weak_handle;
        let weak_image_ortho = weak_image.reject_from_normalized(direction).try_normalize();
        let weak_direction_ortho = weak_direction
            .reject_from_normalized(direction)
            .try_normalize();

        // If one of the two weak vectors was parallel to `direction`, then we just do the first part
        self.rotation = match (weak_image_ortho, weak_direction_ortho) {
            (Some(weak_img_ortho), Some(weak_dir_ortho)) => {
                let second_rotation = Quat::from_rotation_arc(weak_img_ortho, weak_dir_ortho);
                second_rotation * first_rotation
            }
            _ => first_rotation,
        };
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`Transform`]
    #[inline]
    #[must_use]
    pub fn mul_transform(&self, transform: Transform) -> Self {
        let translation = self.transform_point(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    /// Transforms the given `point`, applying scale, rotation and translation.
    ///
    /// If this [`Transform`] has a parent, this will transform a `point` that is
    /// relative to the parent's [`Transform`] into one relative to this [`Transform`].
    ///
    /// If this [`Transform`] does not have a parent, this will transform a `point`
    /// that is in global space into one relative to this [`Transform`].
    ///
    /// If you want to transform a `point` in global space to the local space of this [`Transform`],
    /// consider using [`GlobalTransform::transform_point()`] instead.
    #[inline]
    pub fn transform_point(&self, mut point: Vec3) -> Vec3 {
        point = self.scale * point;
        point = self.rotation * point;
        point += self.translation;
        point
    }

    /// Returns `true` if, and only if, translation, rotation and scale all are
    /// finite. If any of them contains a `NaN`, positive or negative infinity,
    /// this will return `false`.
    #[inline]
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.translation.is_finite() && self.rotation.is_finite() && self.scale.is_finite()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// The transform is expected to be non-degenerate and without shearing, or the output
/// will be invalid.
impl From<GlobalTransform> for Transform {
    fn from(transform: GlobalTransform) -> Self {
        transform.compute_transform()
    }
}

impl Mul<Transform> for Transform {
    type Output = Transform;

    fn mul(self, transform: Transform) -> Self::Output {
        self.mul_transform(transform)
    }
}

impl Mul<GlobalTransform> for Transform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, global_transform: GlobalTransform) -> Self::Output {
        GlobalTransform::from(self) * global_transform
    }
}

impl Mul<Vec3> for Transform {
    type Output = Vec3;

    fn mul(self, value: Vec3) -> Self::Output {
        self.transform_point(value)
    }
}
