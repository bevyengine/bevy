use crate::{primitives::HalfSpace, Mat4, Vec3, Vec4};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A region of 3D space defined by the intersection of 6 [`HalfSpace`]s.
///
/// View Frustums are typically an apex-truncated square pyramid (a pyramid without the top) or a cuboid.
///
/// Half spaces are ordered left, right, top, bottom, near, far. The normal vectors
/// of the half-spaces point towards the interior of the frustum.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, Debug, Default, PartialEq)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct ViewFrustum {
    /// The six half-spaces making up the frustum
    pub half_spaces: [HalfSpace; 6],
}

impl ViewFrustum {
    /// The index for the near plane in `half_spaces`
    pub const NEAR_PLANE_IDX: usize = 4;
    /// The index for the far plane in `half_spaces`
    pub const FAR_PLANE_IDX: usize = 5;
    /// Vec4 representing an inactive half space.
    /// The bisecting plane's unit normal is set to (0, 0, 0).
    /// The signed distance along the normal from the plane to the origin is set to `f32::INFINITY`.
    const INACTIVE_HALF_SPACE: Vec4 = Vec4::new(0.0, 0.0, 0.0, f32::INFINITY);

    /// Returns a view frustum derived from `clip_from_world`.
    #[inline]
    pub fn from_clip_from_world(clip_from_world: &Mat4) -> Self {
        let mut frustum = ViewFrustum::from_clip_from_world_no_far(clip_from_world);
        frustum.half_spaces[Self::FAR_PLANE_IDX] = HalfSpace::new(clip_from_world.row(2));
        frustum
    }

    /// Returns a view frustum derived from `clip_from_world`,
    /// but with a custom far plane.
    #[inline]
    pub fn from_clip_from_world_custom_far(
        clip_from_world: &Mat4,
        view_translation: &Vec3,
        view_backward: &Vec3,
        far: f32,
    ) -> Self {
        let mut frustum = ViewFrustum::from_clip_from_world_no_far(clip_from_world);
        let far_center = *view_translation - far * *view_backward;
        frustum.half_spaces[Self::FAR_PLANE_IDX] =
            HalfSpace::new(view_backward.extend(-view_backward.dot(far_center)));
        frustum
    }

    /// Calculates the corners of this frustum. Returns `None` if the frustum isn't properly defined.
    ///
    /// If `Some`, the corners are returned in the following order:
    /// near top left, near top right, near bottom right, near bottom left,
    /// far top left, far top right, far bottom right, far bottom left.
    /// If the far plane is an inactive half space, the intersection points
    /// that include the far plane will be `Vec3::NAN`.
    #[inline]
    pub fn corners(&self) -> Option<[Vec3; 8]> {
        let [left, right, top, bottom, near, far] = self.half_spaces;
        Some([
            HalfSpace::intersection_point(top, left, near)?,
            HalfSpace::intersection_point(top, right, near)?,
            HalfSpace::intersection_point(bottom, right, near)?,
            HalfSpace::intersection_point(bottom, left, near)?,
            HalfSpace::intersection_point(top, left, far)?,
            HalfSpace::intersection_point(top, right, far)?,
            HalfSpace::intersection_point(bottom, right, far)?,
            HalfSpace::intersection_point(bottom, left, far)?,
        ])
    }

    // NOTE: This approach of extracting the frustum half-space from the view
    // projection matrix is from Foundations of Game Engine Development 2
    // Rendering by Lengyel.
    /// Returns a view frustum derived from `view_projection`,
    /// without a far plane.
    fn from_clip_from_world_no_far(clip_from_world: &Mat4) -> Self {
        let row0 = clip_from_world.row(0);
        let row1 = clip_from_world.row(1);
        let row2 = clip_from_world.row(2);
        let row3 = clip_from_world.row(3);

        Self {
            half_spaces: [
                HalfSpace::new(row3 + row0),
                HalfSpace::new(row3 - row0),
                HalfSpace::new(row3 + row1),
                HalfSpace::new(row3 - row1),
                HalfSpace::new(row3 + row2),
                HalfSpace::new(Self::INACTIVE_HALF_SPACE),
            ],
        }
    }
}

#[cfg(test)]
mod view_frustum_tests {
    use core::f32::consts::FRAC_1_SQRT_2;

    use approx::assert_relative_eq;

    use super::ViewFrustum;
    use crate::{primitives::HalfSpace, Vec3, Vec4};

    #[test]
    fn cuboid_frustum_corners() {
        let cuboid_frustum = ViewFrustum {
            // left: x = -5; right: x = 4
            // near: y = 0; far: y = 6
            // top: z = 3; bottom: z = -2
            half_spaces: [
                // left: yz plane at x = -5
                HalfSpace::new(Vec4::new(1., 0., 0., 5.)),
                // right: yz plane at x = 4
                HalfSpace::new(Vec4::new(-1., 0., 0., 4.)),
                // top: xy plane at z = 3
                HalfSpace::new(Vec4::new(0., 0., -1., 3.)),
                // bottom: xy plane at z = -2
                HalfSpace::new(Vec4::new(0., 0., 1., 2.)),
                // near: xz plane at origin (y = 0)
                HalfSpace::new(Vec4::new(0., 1., 0., 0.)),
                // far: xz plane at y = 6
                HalfSpace::new(Vec4::new(0., -1., 0., 6.)),
            ],
        };
        let corners = cuboid_frustum.corners().unwrap();
        // near top left
        assert_relative_eq!(corners[0], Vec3::new(-5., 0., 3.), epsilon = 2e-7);
        // near top right
        assert_relative_eq!(corners[1], Vec3::new(4., 0., 3.), epsilon = 2e-7);
        // near bottom right
        assert_relative_eq!(corners[2], Vec3::new(4., 0., -2.), epsilon = 2e-7);
        // near bottom left
        assert_relative_eq!(corners[3], Vec3::new(-5., 0., -2.), epsilon = 2e-7);
        // far top left
        assert_relative_eq!(corners[4], Vec3::new(-5., 6., 3.), epsilon = 2e-7);
        // far top right
        assert_relative_eq!(corners[5], Vec3::new(4., 6., 3.), epsilon = 2e-7);
        // far bottom right
        assert_relative_eq!(corners[6], Vec3::new(4., 6., -2.), epsilon = 2e-7);
        // far bottom left
        assert_relative_eq!(corners[7], Vec3::new(-5., 6., -2.), epsilon = 2e-7);
    }

    #[test]
    fn pyramid_frustum_corners() {
        // a frustum where the near plane intersects the left right top and bottom planes
        // at a single point
        let pyramid_frustum = ViewFrustum {
            half_spaces: [
                // left
                HalfSpace::new(Vec4::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2)),
                // right
                HalfSpace::new(Vec4::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2)),
                // top
                HalfSpace::new(Vec4::new(0., FRAC_1_SQRT_2, -FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
                // bottom
                HalfSpace::new(Vec4::new(0., FRAC_1_SQRT_2, FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
                // near: xz plane at y = -1
                HalfSpace::new(Vec4::new(0., 1., 0., 1.)),
                // far: xz plane at y = 3
                HalfSpace::new(Vec4::new(0., -1., 0., 3.)),
            ],
        };
        let corners = pyramid_frustum.corners().unwrap();
        // near top left
        assert_relative_eq!(corners[0], Vec3::new(0., -1., 0.), epsilon = 2e-7);
        // near top right
        assert_relative_eq!(corners[1], Vec3::new(0., -1., 0.), epsilon = 2e-7);
        // near bottom right
        assert_relative_eq!(corners[2], Vec3::new(0., -1., 0.), epsilon = 2e-7);
        // near bottom left
        assert_relative_eq!(corners[3], Vec3::new(0., -1., 0.), epsilon = 2e-7);
        // far top left
        assert_relative_eq!(corners[4], Vec3::new(-4., 3., 4.), epsilon = 2e-7);
        // far top right
        assert_relative_eq!(corners[5], Vec3::new(4., 3., 4.), epsilon = 2e-7);
        // far bottom right
        assert_relative_eq!(corners[6], Vec3::new(4., 3., -4.), epsilon = 2e-7);
        // far bottom left
        assert_relative_eq!(corners[7], Vec3::new(-4., 3., -4.), epsilon = 2e-7);
    }

    #[test]
    fn frustum_with_some_nan_corners() {
        // frustum with no far plane has NAN far corners
        let no_far = ViewFrustum {
            half_spaces: [
                // left: a yz plane rotated outwards
                HalfSpace::new(Vec4::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2)),
                // right: a yz plane rotated outwards
                HalfSpace::new(Vec4::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2)),
                // top: an xz plane rotated outwards
                HalfSpace::new(Vec4::new(0., FRAC_1_SQRT_2, -FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
                // bottom: xz plane rotated outwards
                HalfSpace::new(Vec4::new(0., FRAC_1_SQRT_2, FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
                // near: xz plane at origin (y = 0)
                HalfSpace::new(Vec4::new(0., 1., 0., 0.)),
                // far
                HalfSpace::new(ViewFrustum::INACTIVE_HALF_SPACE),
            ],
        };
        let corners = no_far.corners().unwrap();
        // near top left
        assert_relative_eq!(corners[0], Vec3::new(-1., 0., 1.), epsilon = 2e-7);
        // near top right
        assert_relative_eq!(corners[1], Vec3::new(1., 0., 1.), epsilon = 2e-7);
        // near bottom right
        assert_relative_eq!(corners[2], Vec3::new(1., 0., -1.), epsilon = 2e-7);
        // near bottom left
        assert_relative_eq!(corners[3], Vec3::new(-1., 0., -1.), epsilon = 2e-7);
        // far top left
        assert!(corners[4].is_nan());
        // far top right
        assert!(corners[5].is_nan());
        // far bottom right
        assert!(corners[6].is_nan());
        // far bottom left
        assert!(corners[7].is_nan());
    }

    #[test]
    fn invalid_frustum_corners() {
        let invalid = ViewFrustum {
            half_spaces: [
                // the left and the top half spaces are the same, resulting in no intersection point
                HalfSpace::new(Vec4::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2)),
                HalfSpace::new(Vec4::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., -FRAC_1_SQRT_2)),
                HalfSpace::new(Vec4::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2)),
                HalfSpace::new(Vec4::new(0., FRAC_1_SQRT_2, FRAC_1_SQRT_2, FRAC_1_SQRT_2)),
                HalfSpace::new(Vec4::new(0., 1., 0., 0.)),
                HalfSpace::new(Vec4::new(0., -1., 0., 3.)),
            ],
        };
        assert!(invalid.corners().is_none());
    }
}
