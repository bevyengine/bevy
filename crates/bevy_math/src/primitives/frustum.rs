#[cfg(feature = "bevy_ecs")]
use bevy_ecs::component::Component;
#[cfg(all(feature = "bevy_reflect", feature = "bevy_ecs"))]
use bevy_ecs::prelude::ReflectComponent;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use glam::{Affine3A, Mat4, Vec3};

use super::{
    legacy_bevy_render::{Aabb, Sphere},
    HalfSpace,
};

/// A region of 3D space defined by the intersection of 6 [`HalfSpace`]s.
///
/// Frustums are typically an apex-truncated square pyramid (a pyramid without the top) or a cuboid.
///
/// Half spaces are ordered left, right, top, bottom, near, far. The normal vectors
/// of the half-spaces point towards the interior of the frustum.
///
/// A frustum component is used on an entity with a [`Camera`] component to
/// determine which entities will be considered for rendering by this camera.
/// All entities with an [`Aabb`] component that are not contained by (or crossing
/// the boundary of) the frustum will not be rendered, and not be used in rendering computations.
///
/// This process is called frustum culling, and entities can opt out of it using
/// the [`NoFrustumCulling`] component.
///
/// The frustum component is typically added from a bundle, either the `Camera2dBundle`
/// or the `Camera3dBundle`.
/// It is usually updated automatically by [`update_frusta`] from the
/// [`CameraProjection`] component and [`GlobalTransform`] of the camera entity.
///
/// [`Camera`]: crate::camera::Camera
/// [`NoFrustumCulling`]: crate::view::visibility::NoFrustumCulling
/// [`update_frusta`]: crate::view::visibility::update_frusta
/// [`CameraProjection`]: crate::camera::CameraProjection
/// [`GlobalTransform`]: bevy_transform::components::GlobalTransform
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_ecs", derive(Component))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
#[cfg_attr(
    all(feature = "bevy_ecs", feature = "bevy_reflect"),
    reflect(Component)
)]
pub struct Frustum {
    #[reflect(ignore)]
    /// The six half-spaces making up the frustum
    pub half_spaces: [HalfSpace; 6],
}

impl Frustum {
    /// Returns a frustum derived from `clip_from_world`.
    #[inline]
    pub fn from_clip_from_world(clip_from_world: &Mat4) -> Self {
        let mut frustum = Frustum::from_clip_from_world_no_far(clip_from_world);
        frustum.half_spaces[5] = HalfSpace::new(clip_from_world.row(2));
        frustum
    }

    /// Returns a frustum derived from `clip_from_world`,
    /// but with a custom far plane.
    #[inline]
    pub fn from_clip_from_world_custom_far(
        clip_from_world: &Mat4,
        view_translation: &Vec3,
        view_backward: &Vec3,
        far: f32,
    ) -> Self {
        let mut frustum = Frustum::from_clip_from_world_no_far(clip_from_world);
        let far_center = *view_translation - far * *view_backward;
        frustum.half_spaces[5] =
            HalfSpace::new(view_backward.extend(-view_backward.dot(far_center)));
        frustum
    }

    // NOTE: This approach of extracting the frustum half-space from the view
    // projection matrix is from Foundations of Game Engine Development 2
    // Rendering by Lengyel.
    /// Returns a frustum derived from `view_projection`,
    /// without a far plane.
    fn from_clip_from_world_no_far(clip_from_world: &Mat4) -> Self {
        let row3 = clip_from_world.row(3);
        let mut half_spaces = [HalfSpace::default(); 6];
        for (i, half_space) in half_spaces.iter_mut().enumerate().take(5) {
            let row = clip_from_world.row(i / 2);
            *half_space = HalfSpace::new(if (i & 1) == 0 && i != 4 {
                row3 + row
            } else {
                row3 - row
            });
        }
        Self { half_spaces }
    }

    /// Checks if a sphere intersects the frustum.
    #[inline]
    pub fn intersects_sphere(&self, sphere: &Sphere, intersect_far: bool) -> bool {
        let sphere_center = sphere.center.extend(1.0);
        let max = if intersect_far { 6 } else { 5 };
        for half_space in &self.half_spaces[..max] {
            if half_space.normal_d().dot(sphere_center) + sphere.radius <= 0.0 {
                return false;
            }
        }
        true
    }

    /// Checks if an Oriented Bounding Box (obb) intersects the frustum.
    #[inline]
    pub fn intersects_obb(
        &self,
        aabb: &Aabb,
        world_from_local: &Affine3A,
        intersect_near: bool,
        intersect_far: bool,
    ) -> bool {
        let aabb_center_world = world_from_local.transform_point3a(aabb.center).extend(1.0);
        for (idx, half_space) in self.half_spaces.into_iter().enumerate() {
            if idx == 4 && !intersect_near {
                continue;
            }
            if idx == 5 && !intersect_far {
                continue;
            }
            let p_normal = half_space.normal();
            let relative_radius = aabb.relative_radius(&p_normal, &world_from_local.matrix3);
            if half_space.normal_d().dot(aabb_center_world) + relative_radius <= 0.0 {
                return false;
            }
        }
        true
    }
}
