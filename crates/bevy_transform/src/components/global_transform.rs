use super::Transform;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{const_vec3, Affine3A, Mat3, Mat4, Quat, Vec3};
use bevy_reflect::prelude::*;
use std::ops::Mul;

/// Describe the position of an entity relative to the reference frame.
///
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global position of an entity, you should get its [`GlobalTransform`].
/// * For transform hierarchies to work correctly, you must have both a [`Transform`] and a [`GlobalTransform`].
///   * You may use the [`TransformBundle`](crate::TransformBundle) to guarantee this.
///
/// ## [`Transform`] and [`GlobalTransform`]
///
/// [`Transform`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a [`Parent`](bevy_hierarchy::Parent).
///
/// [`GlobalTransform`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform`] is updated from [`Transform`] in the system
/// [`transform_propagate_system`](crate::transform_propagate_system).
///
/// This system runs in stage [`CoreStage::PostUpdate`](crate::CoreStage::PostUpdate). If you
/// update the[`Transform`] of an entity in this stage or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct GlobalTransform {
    /// The position of the global transform
    pub translation: Vec3,
    /// The rotation of the global transform
    pub rotation: Quat,
    /// The scale of the global transform
    pub scale: Vec3,
}

impl GlobalTransform {
    #[doc(hidden)]
    #[inline]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(const_vec3!([x, y, z]))
    }

    /// Creates a new identity [`GlobalTransform`], with no translation, rotation, and a scale of 1
    /// on all axes.
    #[inline]
    pub const fn identity() -> Self {
        GlobalTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();

        GlobalTransform {
            translation,
            rotation,
            scale,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub const fn from_translation(translation: Vec3) -> Self {
        GlobalTransform {
            translation,
            ..Self::identity()
        }
    }

    #[doc(hidden)]
    #[inline]
    pub const fn from_rotation(rotation: Quat) -> Self {
        GlobalTransform {
            rotation,
            ..Self::identity()
        }
    }

    #[doc(hidden)]
    #[inline]
    pub const fn from_scale(scale: Vec3) -> Self {
        GlobalTransform {
            scale,
            ..Self::identity()
        }
    }

    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    #[doc(hidden)]
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

    /// Returns the 3d affine transformation from this transforms translation,
    /// rotation, and scale.
    #[inline]
    pub fn compute_affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Get the unit vector in the local x direction
    #[inline]
    pub fn local_x(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Equivalent to [`-local_x()`][GlobalTransform::local_x]
    #[inline]
    pub fn left(&self) -> Vec3 {
        -self.local_x()
    }

    /// Equivalent to [`local_x()`][GlobalTransform::local_x]
    #[inline]
    pub fn right(&self) -> Vec3 {
        self.local_x()
    }

    /// Get the unit vector in the local y direction
    #[inline]
    pub fn local_y(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Equivalent to [`local_y()`][GlobalTransform::local_y]
    #[inline]
    pub fn up(&self) -> Vec3 {
        self.local_y()
    }

    /// Equivalent to [`-local_y()`][GlobalTransform::local_y]
    #[inline]
    pub fn down(&self) -> Vec3 {
        -self.local_y()
    }

    /// Get the unit vector in the local z direction
    #[inline]
    pub fn local_z(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    /// Equivalent to [`-local_z()`][GlobalTransform::local_z]
    #[inline]
    pub fn forward(&self) -> Vec3 {
        -self.local_z()
    }

    /// Equivalent to [`local_z()`][GlobalTransform::local_z]
    #[inline]
    pub fn back(&self) -> Vec3 {
        self.local_z()
    }

    #[doc(hidden)]
    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = rotation * self.rotation;
    }

    #[doc(hidden)]
    #[inline]
    pub fn rotate_around(&mut self, point: Vec3, rotation: Quat) {
        self.translation = point + rotation * (self.translation - point);
        self.rotation *= rotation;
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`GlobalTransform`].
    ///
    /// Note that `self.mul_transform(transform)` is identical to `self * transform`.
    ///
    /// To find `X` such as `transform * X = self`, see [`Self::reparented_to`].
    #[inline]
    #[must_use]
    pub fn mul_transform(&self, transform: Transform) -> Self {
        let translation = self.mul_vec3(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Returns `X` such as `transform * X = self`.
    ///
    /// `(t2 * t1).reparented_to(t2) == t1` Note that transforms are not commutative, meaning that
    /// `(t1 * t2).reparented_to(t2) != t1`.
    ///
    /// This is useful if you want to "reparent" an `Entity`. Say you have an entity
    /// `e1` that you want to turn into a child of `e2`, but you want `e1` to keep the
    /// same global transform, even after re-partenting. You would use:
    /// ```rust
    /// # use bevy_math::{Vec3, Quat};
    /// # use bevy_transform::prelude::{GlobalTransform, Transform};
    /// # use bevy_ecs::prelude::{Entity, Query, Component, Commands};
    /// # use bevy_hierarchy::prelude::Parent;
    /// #[derive(Component)]
    /// struct ToReparent {
    ///     new_parent: Entity,
    /// }
    /// fn reparent_system(
    ///     mut commands: Commands,
    ///     mut targets: Query<(&mut Transform, Entity, &GlobalTransform, &ToReparent)>,
    ///     transforms: Query<&GlobalTransform>,
    /// ) {
    ///     for (mut transform, entity, initial, to_reparent) in targets.iter_mut() {
    ///         if let Ok(parent_transform) = transforms.get(to_reparent.new_parent) {
    ///             *transform = initial.reparented_to(*parent_transform);
    ///             commands.entity(entity)
    ///                 .remove::<ToReparent>()
    ///                 .insert(Parent(to_reparent.new_parent));
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn reparented_to(&self, transform: GlobalTransform) -> Transform {
        let (spos, srot, sscale) = (self.translation, self.rotation, self.scale);
        let (tpos, trot, tscale) = (transform.translation, transform.rotation, transform.scale);
        Transform {
            translation: trot.inverse() * (spos - tpos) / tscale,
            rotation: trot.inverse() * srot,
            scale: sscale / tscale,
        }
    }

    /// Returns a [`Vec3`] of this [`Transform`] applied to `value`.
    #[inline]
    pub fn mul_vec3(&self, mut value: Vec3) -> Vec3 {
        value = self.scale * value;
        value = self.rotation * value;
        value += self.translation;
        value
    }

    #[doc(hidden)]
    #[inline]
    pub fn apply_non_uniform_scale(&mut self, scale: Vec3) {
        self.scale *= scale;
    }

    #[doc(hidden)]
    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = Vec3::normalize(self.translation - target);
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
    }
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<Transform> for GlobalTransform {
    fn from(transform: Transform) -> Self {
        Self {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
}

impl Mul<GlobalTransform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, global_transform: GlobalTransform) -> Self::Output {
        self.mul_transform(global_transform.into())
    }
}

impl Mul<Transform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, transform: Transform) -> Self::Output {
        self.mul_transform(transform)
    }
}

impl Mul<Vec3> for GlobalTransform {
    type Output = Vec3;

    #[inline]
    fn mul(self, value: Vec3) -> Self::Output {
        self.mul_vec3(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn reparented_to_transform_identity() {
        fn identity(t1: GlobalTransform, t2: GlobalTransform) -> Transform {
            t2.mul_transform(t1.into()).reparented_to(t2)
        }
        let t1 = GlobalTransform {
            translation: Vec3::new(1034.0, 34.0, -1324.34),
            rotation: Quat::from_rotation_x(1.2),
            scale: Vec3::new(1.0, 2.345, 0.9),
        };
        let t2 = GlobalTransform {
            translation: Vec3::new(28.0, -54.493, 324.34),
            rotation: Quat::from_rotation_z(1.9),
            scale: Vec3::new(3.0, 1.345, 0.9),
        };
        let f32_equal = |left: f32, right: f32| (left - right).abs() < 0.0001;
        let rt_t1_pos = identity(t1, t2).translation;
        let rt_t2_pos = identity(t2, t1).translation;
        assert!(f32_equal(t1.translation.length(), rt_t1_pos.length()));
        assert!(f32_equal(t2.translation.length(), rt_t2_pos.length()));
        assert!(f32_equal(
            t1.scale.length(),
            identity(t1, t2).scale.length()
        ));
        assert!(f32_equal(
            t2.scale.length(),
            identity(t2, t1).scale.length()
        ));
    }
}
