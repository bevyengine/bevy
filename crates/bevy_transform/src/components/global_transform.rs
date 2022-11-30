use std::ops::Mul;

use super::Transform;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Affine3A, Mat4, Quat, Vec3, Vec3A};
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect};

/// Describe the position of an entity relative to the reference frame.
///
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform`].
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
/// [`GlobalTransform`] is updated from [`Transform`] in the systems labeled
/// [`TransformPropagate`](crate::TransformSystem::TransformPropagate).
///
/// This system runs in stage [`CoreStage::PostUpdate`](crate::CoreStage::PostUpdate). If you
/// update the [`Transform`] of an entity in this stage or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
///
/// # Examples
///
/// - [`global_vs_local_translation`]
///
/// [`global_vs_local_translation`]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/global_vs_local_translation.rs
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, FromReflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component, Default, PartialEq)]
pub struct GlobalTransform(Affine3A);

macro_rules! impl_local_axis {
    ($pos_name: ident, $neg_name: ident, $axis: ident) => {
        #[doc=std::concat!("Return the local ", std::stringify!($pos_name), " vector (", std::stringify!($axis) ,").")]
        #[inline]
        pub fn $pos_name(&self) -> Vec3 {
            (self.0.matrix3 * Vec3::$axis).normalize()
        }

        #[doc=std::concat!("Return the local ", std::stringify!($neg_name), " vector (-", std::stringify!($axis) ,").")]
        #[inline]
        pub fn $neg_name(&self) -> Vec3 {
            -self.$pos_name()
        }
    };
}

impl GlobalTransform {
    /// An identity [`GlobalTransform`] that maps all points in space to themselves.
    pub const IDENTITY: Self = Self(Affine3A::IDENTITY);

    #[doc(hidden)]
    #[inline]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        GlobalTransform(Affine3A::from_translation(translation))
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        GlobalTransform(Affine3A::from_rotation_translation(rotation, Vec3::ZERO))
    }

    #[doc(hidden)]
    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        GlobalTransform(Affine3A::from_scale(scale))
    }

    /// Returns the 3d affine transformation matrix as a [`Mat4`].
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from(self.0)
    }

    /// Returns the 3d affine transformation matrix as an [`Affine3A`].
    #[inline]
    pub fn affine(&self) -> Affine3A {
        self.0
    }

    /// Returns the transformation as a [`Transform`].
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output
    /// will be invalid.
    #[inline]
    pub fn compute_transform(&self) -> Transform {
        let (scale, rotation, translation) = self.0.to_scale_rotation_translation();
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    /// Extracts `scale`, `rotation` and `translation` from `self`.
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output
    /// will be invalid.
    #[inline]
    pub fn to_scale_rotation_translation(&self) -> (Vec3, Quat, Vec3) {
        self.0.to_scale_rotation_translation()
    }

    impl_local_axis!(right, left, X);
    impl_local_axis!(up, down, Y);
    impl_local_axis!(back, forward, Z);

    /// Get the translation as a [`Vec3`].
    #[inline]
    pub fn translation(&self) -> Vec3 {
        self.0.translation.into()
    }

    /// Mutably access the internal translation.
    #[inline]
    pub fn translation_mut(&mut self) -> &mut Vec3A {
        &mut self.0.translation
    }

    /// Get the translation as a [`Vec3A`].
    #[inline]
    pub fn translation_vec3a(&self) -> Vec3A {
        self.0.translation
    }

    /// Get an upper bound of the radius from the given `extents`.
    #[inline]
    pub fn radius_vec3a(&self, extents: Vec3A) -> f32 {
        (self.0.matrix3 * extents).length()
    }

    /// Transforms the given `point`, applying shear, scale, rotation and translation.
    ///
    /// This moves `point` into the local space of this [`GlobalTransform`].
    #[inline]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.0.transform_point3(point)
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`GlobalTransform`]
    #[inline]
    pub fn mul_transform(&self, transform: Transform) -> Self {
        Self(self.0 * transform.compute_affine())
    }
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl From<Transform> for GlobalTransform {
    fn from(transform: Transform) -> Self {
        Self(transform.compute_affine())
    }
}

impl From<Affine3A> for GlobalTransform {
    fn from(affine: Affine3A) -> Self {
        Self(affine)
    }
}

impl From<Mat4> for GlobalTransform {
    fn from(matrix: Mat4) -> Self {
        Self(Affine3A::from_mat4(matrix))
    }
}

impl Mul<GlobalTransform> for GlobalTransform {
    type Output = GlobalTransform;

    #[inline]
    fn mul(self, global_transform: GlobalTransform) -> Self::Output {
        GlobalTransform(self.0 * global_transform.0)
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
        self.transform_point(value)
    }
}
