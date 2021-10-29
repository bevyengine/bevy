use bevy_ecs::component::Component;
use bevy_math::{Mat3, Mat4};

/// Describes the relationship between [`Parent`](crate::Parent) and [`Children`](crate::Children) by mapping [`Transform`](crate::Transform)s with matrices
///
/// Add to a child entity to control how the parent's transform affects the child's
///
/// To remove a relationship, use the respective `ZERO` constant matrix
///
/// The [`Default`] implementation is a 1:1 relationship from the `IDENTITY` matrices, the same result as if not added to a child entity
///
/// ## Example
///
/// ```
/// use bevy::math::const_mat3;
/// LocalSpace {
///     translation: const_mat3!(
///         [0.0, 1.0, 0.0],
///         [1.0, 0.0, 0.0],
///         [0.0, 0.0, 1.0]
///     ),
///     rotation: Mat4::ZERO,
///     ..Default::default()
/// }
/// ```
///
/// See the `local_space` example in `./examples/3d/local_space.rs`
#[derive(Component, Clone, Copy, Debug)]
pub struct LocalSpace {
    pub translation: Mat3,
    pub rotation: Mat4,
    pub scale: Mat3,
}

impl Default for LocalSpace {
    fn default() -> Self {
        Self {
            translation: Mat3::IDENTITY,
            rotation: Mat4::IDENTITY,
            scale: Mat3::IDENTITY,
        }
    }
}
