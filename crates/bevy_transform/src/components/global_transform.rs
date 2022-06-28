use super::Transform;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::Vec3;
use bevy_reflect::prelude::*;
use std::ops::{Deref, Mul};

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
#[derive(Component, Default, Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct GlobalTransform(pub(crate) Transform);

impl GlobalTransform {
    /// Retrieves the [`Transform`] inner type of `Self`
    pub fn into_inner(self) -> Transform {
        self.0
    }
}

impl Deref for GlobalTransform {
    type Target = Transform;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Transform> for GlobalTransform {
    fn from(t: Transform) -> Self {
        Self(t)
    }
}

impl Mul<GlobalTransform> for GlobalTransform {
    type Output = Self;

    #[inline]
    fn mul(self, global_transform: GlobalTransform) -> Self::Output {
        Self(self.mul_transform(global_transform.0))
    }
}

impl Mul<Transform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, transform: Transform) -> Self::Output {
        Self(self.mul_transform(transform))
    }
}

impl Mul<Vec3> for GlobalTransform {
    type Output = Vec3;

    #[inline]
    fn mul(self, value: Vec3) -> Self::Output {
        self.mul_vec3(value)
    }
}
