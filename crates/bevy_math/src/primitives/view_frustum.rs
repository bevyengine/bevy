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
    #[inline]
    pub fn corners(&self) -> Option<[Vec3; 8]> {
        let [left, right, top, bottom, near, far] = self.half_spaces;
        Some([
            HalfSpace::intersect(top, left, near)?,
            HalfSpace::intersect(top, right, near)?,
            HalfSpace::intersect(bottom, right, near)?,
            HalfSpace::intersect(bottom, left, near)?,
            HalfSpace::intersect(top, left, far)?,
            HalfSpace::intersect(top, right, far)?,
            HalfSpace::intersect(bottom, right, far)?,
            HalfSpace::intersect(bottom, left, far)?,
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
