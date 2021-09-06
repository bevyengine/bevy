use super::Transform;
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::{Mat3, Mat4, Quat, Vec3};
use bevy_reflect::Reflect;
use std::ops::Mul;

/// Describe the position of an entity relative to the reference frame.
///
/// * To place or move an entity, you should set its [`Transform`].
/// * To be displayed, an entity must have both a [`Transform`] and a [`GlobalTransform`].
/// * To get the global position of an entity, you should get its [`GlobalTransform`].
///
/// ## [`Transform`] and [`GlobalTransform`]
///
/// [`Transform`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a [`Parent`](super::Parent).
///
/// [`GlobalTransform`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform`] is updated from [`Transform`] in the system
/// [`transform_propagate_system`](crate::transform_propagate_system::transform_propagate_system).
///
/// In pseudo code:
/// ```ignore
/// for entity in entities_without_parent:
///     set entity.global_transform to entity.transform
///     recursively:
///         set parent to current entity
///         for child in parent.children:
///             set child.global_transform to parent.global_transform * child.transform
/// ```
///
/// This system runs in stage [`CoreStage::PostUpdate`](crate::CoreStage::PostUpdate). If you
/// update the[`Transform`] of an entity in this stage or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
#[derive(Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Component, PartialEq)]
pub struct GlobalTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl GlobalTransform {
    #[doc(hidden)]
    #[inline]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
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
    pub fn from_translation(translation: Vec3) -> Self {
        GlobalTransform {
            translation,
            ..Default::default()
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        GlobalTransform {
            rotation,
            ..Default::default()
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        GlobalTransform {
            scale,
            ..Default::default()
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    #[doc(hidden)]
    #[inline]
    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    #[doc(hidden)]
    #[inline]
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    #[doc(hidden)]
    #[inline]
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Returns the 3d affine transformation matrix from this transforms translation,
    /// rotation, and scale.
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Get the unit vector in the local x direction
    #[inline]
    pub fn local_x(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Equivalent to -local_x()
    #[inline]
    pub fn left(&self) -> Vec3 {
        -self.local_x()
    }

    /// Equivalent to local_x()
    #[inline]
    pub fn right(&self) -> Vec3 {
        self.local_x()
    }

    /// Get the unit vector in the local y direction
    #[inline]
    pub fn local_y(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Equivalent to local_y()
    #[inline]
    pub fn up(&self) -> Vec3 {
        self.local_y()
    }

    /// Equivalent to -local_y()
    #[inline]
    pub fn down(&self) -> Vec3 {
        -self.local_y()
    }

    /// Get the unit vector in the local z direction
    #[inline]
    pub fn local_z(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    /// Equivalent to -local_z()
    #[inline]
    pub fn forward(&self) -> Vec3 {
        -self.local_z()
    }

    /// Equivalent to local_z()
    #[inline]
    pub fn back(&self) -> Vec3 {
        self.local_z()
    }

    #[doc(hidden)]
    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`GlobalTransform`]
    #[inline]
    pub fn mul_transform(&self, transform: Transform) -> GlobalTransform {
        let translation = self.mul_vec3(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        GlobalTransform {
            translation,
            rotation,
            scale,
        }
    }

    /// Returns a [`Vec3`] of this [`Transform`] applied to `value`.
    #[inline]
    pub fn mul_vec3(&self, mut value: Vec3) -> Vec3 {
        value = self.rotation * value;
        value = self.scale * value;
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
