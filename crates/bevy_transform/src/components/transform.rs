use super::GlobalTransform;
#[cfg(feature = "bevy-support")]
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Affine3A, Dir3, Mat3, Mat4, Quat, Vec3};
#[cfg(feature = "bevy-support")]
use bevy_reflect::{prelude::*, Reflect};
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
/// - [`transform`][transform_example]
///
/// [transform_example]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/transform.rs
#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy-support",
    derive(Component, Reflect),
    reflect(Component, Default, PartialEq)
)]
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
    pub fn from_matrix(world_from_local: Mat4) -> Self {
        let (scale, rotation, translation) = world_from_local.to_scale_rotation_translation();

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
    /// * if `up` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::Y` is used instead
    /// * if the resulting forward direction is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    #[must_use]
    pub fn looking_at(mut self, target: Vec3, up: impl TryInto<Dir3>) -> Self {
        self.look_at(target, up);
        self
    }

    /// Returns this [`Transform`] with a new rotation so that [`Transform::forward`]
    /// points in the given `direction` and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::Z` is used instead
    /// * if `up` fails converting to `Dir3`, `Dir3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    #[must_use]
    pub fn looking_to(mut self, direction: impl TryInto<Dir3>, up: impl TryInto<Dir3>) -> Self {
        self.look_to(direction, up);
        self
    }

    /// Rotates this [`Transform`] so that the `main_axis` vector, reinterpreted in local coordinates, points
    /// in the given `main_direction`, while `secondary_axis` points towards `secondary_direction`.
    /// For example, if a spaceship model has its nose pointing in the X-direction in its own local coordinates
    /// and its dorsal fin pointing in the Y-direction, then `align(Dir3::X, v, Dir3::Y, w)` will make the spaceship's
    /// nose point in the direction of `v`, while the dorsal fin does its best to point in the direction `w`.
    ///
    ///
    /// In some cases a rotation cannot be constructed. Another axis will be picked in those cases:
    /// * if `main_axis` or `main_direction` fail converting to `Dir3` (e.g are zero), `Dir3::X` takes their place
    /// * if `secondary_axis` or `secondary_direction` fail converting, `Dir3::Y` takes their place
    /// * if `main_axis` is parallel with `secondary_axis` or `main_direction` is parallel with `secondary_direction`,
    /// a rotation is constructed which takes `main_axis` to `main_direction` along a great circle, ignoring the secondary
    /// counterparts
    ///
    /// See [`Transform::align`] for additional details.
    #[inline]
    #[must_use]
    pub fn aligned_by(
        mut self,
        main_axis: impl TryInto<Dir3>,
        main_direction: impl TryInto<Dir3>,
        secondary_axis: impl TryInto<Dir3>,
        secondary_direction: impl TryInto<Dir3>,
    ) -> Self {
        self.align(
            main_axis,
            main_direction,
            secondary_axis,
            secondary_direction,
        );
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
    pub fn local_x(&self) -> Dir3 {
        // Quat * unit vector is length 1
        Dir3::new_unchecked(self.rotation * Vec3::X)
    }

    /// Equivalent to [`-local_x()`][Transform::local_x()]
    #[inline]
    pub fn left(&self) -> Dir3 {
        -self.local_x()
    }

    /// Equivalent to [`local_x()`][Transform::local_x()]
    #[inline]
    pub fn right(&self) -> Dir3 {
        self.local_x()
    }

    /// Get the unit vector in the local `Y` direction.
    #[inline]
    pub fn local_y(&self) -> Dir3 {
        // Quat * unit vector is length 1
        Dir3::new_unchecked(self.rotation * Vec3::Y)
    }

    /// Equivalent to [`local_y()`][Transform::local_y]
    #[inline]
    pub fn up(&self) -> Dir3 {
        self.local_y()
    }

    /// Equivalent to [`-local_y()`][Transform::local_y]
    #[inline]
    pub fn down(&self) -> Dir3 {
        -self.local_y()
    }

    /// Get the unit vector in the local `Z` direction.
    #[inline]
    pub fn local_z(&self) -> Dir3 {
        // Quat * unit vector is length 1
        Dir3::new_unchecked(self.rotation * Vec3::Z)
    }

    /// Equivalent to [`-local_z()`][Transform::local_z]
    #[inline]
    pub fn forward(&self) -> Dir3 {
        -self.local_z()
    }

    /// Equivalent to [`local_z()`][Transform::local_z]
    #[inline]
    pub fn back(&self) -> Dir3 {
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
    pub fn rotate_axis(&mut self, axis: Dir3, angle: f32) {
        self.rotate(Quat::from_axis_angle(axis.into(), angle));
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
    pub fn rotate_local_axis(&mut self, axis: Dir3, angle: f32) {
        self.rotate_local(Quat::from_axis_angle(axis.into(), angle));
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
    /// * if `up` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::Y` is used instead
    /// * if the resulting forward direction is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: impl TryInto<Dir3>) {
        self.look_to(target - self.translation, up);
    }

    /// Rotates this [`Transform`] so that [`Transform::forward`] points in the given `direction`
    /// and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::NEG_Z` is used instead
    /// * if `up` fails converting to `Dir3`, `Dir3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    pub fn look_to(&mut self, direction: impl TryInto<Dir3>, up: impl TryInto<Dir3>) {
        let back = -direction.try_into().unwrap_or(Dir3::NEG_Z);
        let up = up.try_into().unwrap_or(Dir3::Y);
        let right = up
            .cross(back.into())
            .try_normalize()
            .unwrap_or_else(|| up.any_orthonormal_vector());
        let up = back.cross(right);
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back.into()));
    }

    /// Rotates this [`Transform`] so that the `main_axis` vector, reinterpreted in local coordinates, points
    /// in the given `main_direction`, while `secondary_axis` points towards `secondary_direction`.
    ///
    /// For example, if a spaceship model has its nose pointing in the X-direction in its own local coordinates
    /// and its dorsal fin pointing in the Y-direction, then `align(Dir3::X, v, Dir3::Y, w)` will make the spaceship's
    /// nose point in the direction of `v`, while the dorsal fin does its best to point in the direction `w`.
    ///
    /// More precisely, the [`Transform::rotation`] produced will be such that:
    /// * applying it to `main_axis` results in `main_direction`
    /// * applying it to `secondary_axis` produces a vector that lies in the half-plane generated by `main_direction` and
    /// `secondary_direction` (with positive contribution by `secondary_direction`)
    ///
    /// [`Transform::look_to`] is recovered, for instance, when `main_axis` is `Dir3::NEG_Z` (the [`Transform::forward`]
    /// direction in the default orientation) and `secondary_axis` is `Dir3::Y` (the [`Transform::up`] direction in the default
    /// orientation). (Failure cases may differ somewhat.)
    ///
    /// In some cases a rotation cannot be constructed. Another axis will be picked in those cases:
    /// * if `main_axis` or `main_direction` fail converting to `Dir3` (e.g are zero), `Dir3::X` takes their place
    /// * if `secondary_axis` or `secondary_direction` fail converting, `Dir3::Y` takes their place
    /// * if `main_axis` is parallel with `secondary_axis` or `main_direction` is parallel with `secondary_direction`,
    /// a rotation is constructed which takes `main_axis` to `main_direction` along a great circle, ignoring the secondary
    /// counterparts
    ///
    /// Example
    /// ```
    /// # use bevy_math::{Dir3, Vec3, Quat};
    /// # use bevy_transform::components::Transform;
    /// # let mut t1 = Transform::IDENTITY;
    /// # let mut t2 = Transform::IDENTITY;
    /// t1.align(Dir3::X, Dir3::Y, Vec3::new(1., 1., 0.), Dir3::Z);
    /// let main_axis_image = t1.rotation * Dir3::X;
    /// let secondary_axis_image = t1.rotation * Vec3::new(1., 1., 0.);
    /// assert!(main_axis_image.abs_diff_eq(Vec3::Y, 1e-5));
    /// assert!(secondary_axis_image.abs_diff_eq(Vec3::new(0., 1., 1.), 1e-5));
    ///
    /// t1.align(Vec3::ZERO, Dir3::Z, Vec3::ZERO, Dir3::X);
    /// t2.align(Dir3::X, Dir3::Z, Dir3::Y, Dir3::X);
    /// assert_eq!(t1.rotation, t2.rotation);
    ///
    /// t1.align(Dir3::X, Dir3::Z, Dir3::X, Dir3::Y);
    /// assert_eq!(t1.rotation, Quat::from_rotation_arc(Vec3::X, Vec3::Z));
    /// ```
    #[inline]
    pub fn align(
        &mut self,
        main_axis: impl TryInto<Dir3>,
        main_direction: impl TryInto<Dir3>,
        secondary_axis: impl TryInto<Dir3>,
        secondary_direction: impl TryInto<Dir3>,
    ) {
        let main_axis = main_axis.try_into().unwrap_or(Dir3::X);
        let main_direction = main_direction.try_into().unwrap_or(Dir3::X);
        let secondary_axis = secondary_axis.try_into().unwrap_or(Dir3::Y);
        let secondary_direction = secondary_direction.try_into().unwrap_or(Dir3::Y);

        // The solution quaternion will be constructed in two steps.
        // First, we start with a rotation that takes `main_axis` to `main_direction`.
        let first_rotation = Quat::from_rotation_arc(main_axis.into(), main_direction.into());

        // Let's follow by rotating about the `main_direction` axis so that the image of `secondary_axis`
        // is taken to something that lies in the plane of `main_direction` and `secondary_direction`. Since
        // `main_direction` is fixed by this rotation, the first criterion is still satisfied.
        let secondary_image = first_rotation * secondary_axis;
        let secondary_image_ortho = secondary_image
            .reject_from_normalized(main_direction.into())
            .try_normalize();
        let secondary_direction_ortho = secondary_direction
            .reject_from_normalized(main_direction.into())
            .try_normalize();

        // If one of the two weak vectors was parallel to `main_direction`, then we just do the first part
        self.rotation = match (secondary_image_ortho, secondary_direction_ortho) {
            (Some(secondary_img_ortho), Some(secondary_dir_ortho)) => {
                let second_rotation =
                    Quat::from_rotation_arc(secondary_img_ortho, secondary_dir_ortho);
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
