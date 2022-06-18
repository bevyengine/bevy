use super::Transform;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Affine3A, Mat4, Vec3, Vec3A};
use bevy_reflect::Reflect;

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
#[reflect(Component, PartialEq)]
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
    /// Returns the 3d affine transformation matrix as a [`Mat4`].
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from(self.0)
    }

    /// Returns the 3d affine transformation matrix as an [`Affine3A`].
    #[inline]
    pub fn compute_affine(&self) -> Affine3A {
        self.0
    }

    /// Returns the transformation as a [`Transform`].
    #[inline]
    pub fn compute_transform(&self) -> Transform {
        let (scale, rotation, translation) = self.0.to_scale_rotation_translation();
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    /// Creates a new identity [`GlobalTransform`], that maps all points in space to themselves.
    #[inline]
    pub const fn identity() -> Self {
        Self(Affine3A::IDENTITY)
    }

    impl_local_axis!(right, left, X);
    impl_local_axis!(up, down, Y);
    impl_local_axis!(back, forward, Z);

    /// Get the translation as a [`Vec3`].
    #[inline]
    pub fn translation(&self) -> Vec3 {
        self.0.translation.into()
    }

    /// Get the translation as a [`Vec3A`].
    #[inline]
    pub fn translation_vec3a(&self) -> Vec3A {
        self.0.translation
    }

    /// Get an upper bound of the radius from the given `extents`.
    #[inline]
    pub fn radius(&self, extents: Vec3) -> f32 {
        (self.0.matrix3 * extents).length()
    }

    /// Returns a [`Vec3`] of this [`Transform`] applied to `value`.
    #[inline]
    pub fn mul_vec3(&self, v: Vec3) -> Vec3 {
        self.0.transform_point3(v)
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`GlobalTransform`]
    pub fn mul_transform(&self, transform: Transform) -> Self {
        Self(self.0 * transform.compute_affine())
    }
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self::identity()
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
