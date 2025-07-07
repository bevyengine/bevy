use core::ops::Mul;

use super::Transform;
use bevy_math::{ops, Affine3A, Dir3, Isometry3d, Mat4, Quat, Vec3, Vec3A};
use derive_more::derive::From;

#[cfg(all(feature = "bevy_reflect", feature = "serialize"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

#[cfg(feature = "bevy-support")]
use bevy_ecs::{component::Component, hierarchy::validate_parent_has_component};

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::reflect::ReflectComponent,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

/// [`GlobalTransform`] is an affine transformation from entity-local coordinates to worldspace coordinates.
///
/// You cannot directly mutate [`GlobalTransform`]; instead, you change an entity's transform by manipulating
/// its [`Transform`], which indirectly causes Bevy to update its [`GlobalTransform`].
///
/// * To get the global transform of an entity, you should get its [`GlobalTransform`].
/// * For transform hierarchies to work correctly, you must have both a [`Transform`] and a [`GlobalTransform`].
///   [`GlobalTransform`] is automatically inserted whenever [`Transform`] is inserted.
///
/// ## [`Transform`] and [`GlobalTransform`]
///
/// [`Transform`] transforms an entity relative to its parent's reference frame, or relative to world space coordinates,
/// if it doesn't have a [`ChildOf`](bevy_ecs::hierarchy::ChildOf) component.
///
/// [`GlobalTransform`] is managed by Bevy; it is computed by successively applying the [`Transform`] of each ancestor
/// entity which has a Transform. This is done automatically by Bevy-internal systems in the [`TransformSystems::Propagate`]
/// system set.
///
/// This system runs during [`PostUpdate`](bevy_app::PostUpdate). If you
/// update the [`Transform`] of an entity in this schedule or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
///
/// [`TransformSystems::Propagate`]: crate::TransformSystems::Propagate
///
/// # Examples
///
/// - [`transform`][transform_example]
///
/// [transform_example]: https://github.com/bevyengine/bevy/blob/latest/examples/transforms/transform.rs
#[derive(Debug, PartialEq, Clone, Copy, From)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy-support",
    derive(Component),
    component(on_insert = validate_parent_has_component::<GlobalTransform>)
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
pub struct GlobalTransform(Affine3A);

macro_rules! impl_local_axis {
    ($pos_name: ident, $neg_name: ident, $axis: ident) => {
        #[doc=core::concat!("Return the local ", core::stringify!($pos_name), " vector (", core::stringify!($axis) ,").")]
        #[inline]
        pub fn $pos_name(&self) -> Dir3 {
            Dir3::new_unchecked((self.0.matrix3 * Vec3::$axis).normalize())
        }

        #[doc=core::concat!("Return the local ", core::stringify!($neg_name), " vector (-", core::stringify!($axis) ,").")]
        #[inline]
        pub fn $neg_name(&self) -> Dir3 {
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

    #[doc(hidden)]
    #[inline]
    pub fn from_isometry(iso: Isometry3d) -> Self {
        Self(iso.into())
    }

    /// Returns the 3d affine transformation matrix as a [`Mat4`].
    #[inline]
    pub fn to_matrix(&self) -> Mat4 {
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

    /// Computes a Scale-Rotation-Translation decomposition of the transformation and returns
    /// the isometric part as an [isometry]. Any scaling done by the transformation will be ignored.
    /// Note: this is a somewhat costly and lossy conversion.
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output
    /// will be invalid.
    ///
    /// [isometry]: Isometry3d
    #[inline]
    pub fn to_isometry(&self) -> Isometry3d {
        let (_, rotation, translation) = self.0.to_scale_rotation_translation();
        Isometry3d::new(translation, rotation)
    }

    /// Returns the [`Transform`] `self` would have if it was a child of an entity
    /// with the `parent` [`GlobalTransform`].
    ///
    /// This is useful if you want to "reparent" an [`Entity`](bevy_ecs::entity::Entity).
    /// Say you have an entity `e1` that you want to turn into a child of `e2`,
    /// but you want `e1` to keep the same global transform, even after re-parenting. You would use:
    ///
    /// ```
    /// # use bevy_transform::prelude::{GlobalTransform, Transform};
    /// # use bevy_ecs::prelude::{Entity, Query, Component, Commands, ChildOf};
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
    ///             *transform = initial.reparented_to(parent_transform);
    ///             commands.entity(entity)
    ///                 .remove::<ToReparent>()
    ///                 .insert(ChildOf(to_reparent.new_parent));
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output
    /// will be invalid.
    #[inline]
    pub fn reparented_to(&self, parent: &GlobalTransform) -> Transform {
        let relative_affine = parent.affine().inverse() * self.affine();
        let (scale, rotation, translation) = relative_affine.to_scale_rotation_translation();
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

    /// Get the translation as a [`Vec3A`].
    #[inline]
    pub fn translation_vec3a(&self) -> Vec3A {
        self.0.translation
    }

    /// Get the rotation as a [`Quat`].
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output will be invalid.
    ///
    /// # Warning
    ///
    /// This is calculated using `to_scale_rotation_translation`, meaning that you
    /// should probably use it directly if you also need translation or scale.
    #[inline]
    pub fn rotation(&self) -> Quat {
        self.to_scale_rotation_translation().1
    }

    /// Get the scale as a [`Vec3`].
    ///
    /// The transform is expected to be non-degenerate and without shearing, or the output will be invalid.
    ///
    /// Some of the computations overlap with `to_scale_rotation_translation`, which means you should use
    /// it instead if you also need rotation.
    #[inline]
    pub fn scale(&self) -> Vec3 {
        //Formula based on glam's implementation https://github.com/bitshifter/glam-rs/blob/2e4443e70c709710dfb25958d866d29b11ed3e2b/src/f32/affine3a.rs#L290
        let det = self.0.matrix3.determinant();
        Vec3::new(
            self.0.matrix3.x_axis.length() * ops::copysign(1., det),
            self.0.matrix3.y_axis.length(),
            self.0.matrix3.z_axis.length(),
        )
    }

    /// Get an upper bound of the radius from the given `extents`.
    #[inline]
    pub fn radius_vec3a(&self, extents: Vec3A) -> f32 {
        (self.0.matrix3 * extents).length()
    }

    /// Transforms the given point from local space to global space, applying shear, scale, rotation and translation.
    ///
    /// It can be used like this:
    ///
    /// ```
    /// # use bevy_transform::prelude::{GlobalTransform};
    /// # use bevy_math::prelude::Vec3;
    /// let global_transform = GlobalTransform::from_xyz(1., 2., 3.);
    /// let local_point = Vec3::new(1., 2., 3.);
    /// let global_point = global_transform.transform_point(local_point);
    /// assert_eq!(global_point, Vec3::new(2., 4., 6.));
    /// ```
    ///
    /// ```
    /// # use bevy_transform::prelude::{GlobalTransform};
    /// # use bevy_math::Vec3;
    /// let global_point = Vec3::new(2., 4., 6.);
    /// let global_transform = GlobalTransform::from_xyz(1., 2., 3.);
    /// let local_point = global_transform.affine().inverse().transform_point3(global_point);
    /// assert_eq!(local_point, Vec3::new(1., 2., 3.))
    /// ```
    ///
    /// To apply shear, scale, and rotation *without* applying translation, different functions are available:
    /// ```
    /// # use bevy_transform::prelude::{GlobalTransform};
    /// # use bevy_math::prelude::Vec3;
    /// let global_transform = GlobalTransform::from_xyz(1., 2., 3.);
    /// let local_direction = Vec3::new(1., 2., 3.);
    /// let global_direction = global_transform.affine().transform_vector3(local_direction);
    /// assert_eq!(global_direction, Vec3::new(1., 2., 3.));
    /// let roundtripped_local_direction = global_transform.affine().inverse().transform_vector3(global_direction);
    /// assert_eq!(roundtripped_local_direction, local_direction);
    /// ```
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

impl From<Mat4> for GlobalTransform {
    fn from(world_from_local: Mat4) -> Self {
        Self(Affine3A::from_mat4(world_from_local))
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

#[cfg(test)]
mod test {
    use super::*;

    use bevy_math::EulerRot::XYZ;

    fn transform_equal(left: GlobalTransform, right: Transform) -> bool {
        left.0.abs_diff_eq(right.compute_affine(), 0.01)
    }

    #[test]
    fn reparented_to_transform_identity() {
        fn reparent_to_same(t1: GlobalTransform, t2: GlobalTransform) -> Transform {
            t2.mul_transform(t1.into()).reparented_to(&t2)
        }
        let t1 = GlobalTransform::from(Transform {
            translation: Vec3::new(1034.0, 34.0, -1324.34),
            rotation: Quat::from_euler(XYZ, 1.0, 0.9, 2.1),
            scale: Vec3::new(1.0, 1.0, 1.0),
        });
        let t2 = GlobalTransform::from(Transform {
            translation: Vec3::new(0.0, -54.493, 324.34),
            rotation: Quat::from_euler(XYZ, 1.9, 0.3, 3.0),
            scale: Vec3::new(1.345, 1.345, 1.345),
        });
        let retransformed = reparent_to_same(t1, t2);
        assert!(
            transform_equal(t1, retransformed),
            "t1:{:#?} retransformed:{:#?}",
            t1.compute_transform(),
            retransformed,
        );
    }
    #[test]
    fn reparented_usecase() {
        let t1 = GlobalTransform::from(Transform {
            translation: Vec3::new(1034.0, 34.0, -1324.34),
            rotation: Quat::from_euler(XYZ, 0.8, 1.9, 2.1),
            scale: Vec3::new(10.9, 10.9, 10.9),
        });
        let t2 = GlobalTransform::from(Transform {
            translation: Vec3::new(28.0, -54.493, 324.34),
            rotation: Quat::from_euler(XYZ, 0.0, 3.1, 0.1),
            scale: Vec3::new(0.9, 0.9, 0.9),
        });
        // goal: find `X` such as `t2 * X = t1`
        let reparented = t1.reparented_to(&t2);
        let t1_prime = t2 * reparented;
        assert!(
            transform_equal(t1, t1_prime.into()),
            "t1:{:#?} t1_prime:{:#?}",
            t1.compute_transform(),
            t1_prime.compute_transform(),
        );
    }

    #[test]
    fn scale() {
        let test_values = [-42.42, 0., 42.42];
        for x in test_values {
            for y in test_values {
                for z in test_values {
                    let scale = Vec3::new(x, y, z);
                    let gt = GlobalTransform::from_scale(scale);
                    assert_eq!(gt.scale(), gt.to_scale_rotation_translation().0);
                }
            }
        }
    }
}
