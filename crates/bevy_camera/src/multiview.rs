use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_reflect::Reflect;
use bevy_transform::prelude::Transform;
use core::num::NonZeroU32;

/// Configures a camera to render to multiple view layers in a single render
/// pass.
///
/// Add this component alongside [`Camera3d`](crate::Camera3d) (or
/// [`Camera2d`](crate::Camera2d)) to enable single-pass multiview rendering.
/// Each [`MultiviewSubview`] in `views` produces one layer of the camera's
/// render target texture array; multiview-aware shaders read per-view data
/// via WGSL's `@builtin(view_index)`.
///
/// The canonical use case is single-pass stereo rendering for VR / XR, where
/// `views` holds two entries (one per eye). Other uses include cubemap
/// captures and other rare "render this thing from N angles at once"
/// scenarios.
///
/// The camera's own [`GlobalTransform`](bevy_transform::prelude::GlobalTransform)
/// is the "head" pose; each subview's
/// [`view_from_camera`](MultiviewSubview::view_from_camera) is an offset
/// applied on top of that. Sort distance, frustum culling, and other
/// view-level decisions still use the head pose, so per-eye disagreements
/// (which only matter for objects nearer than the inter-pupillary distance)
/// share the head's ordering.
///
/// `views` must contain between 1 and [`MAX_VIEW_COUNT`] entries (inclusive).
/// A camera with an empty `views` is treated as if it had no `Multiview`
/// component at all; a camera with more than [`MAX_VIEW_COUNT`] entries is
/// reported as a warning and falls back to non-multiview rendering.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone, Debug)]
pub struct Multiview {
    /// One entry per view layer. See [`Multiview`] for the length contract.
    pub views: Vec<MultiviewSubview>,
}

/// Maximum number of layers supported in a single multiview pass.
///
/// `wgpu`'s `multiview_mask` is a `u32` bitmask with one bit per layer, so
/// the platform ceiling is 32. Hardware limits may be lower (e.g. Vulkan
/// `maxMultiviewViewCount` is at least 6 on conformant devices).
pub const MAX_VIEW_COUNT: usize = 32;

impl Multiview {
    /// Returns the number of view layers as a `NonZeroU32`, or `None` if
    /// `views` is empty (in which case the component should be ignored).
    pub fn view_count(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.views.len() as u32)
    }

    /// Returns the multiview mask covering all layers (bits `0..view_count`).
    /// Suitable for passing as `multiview_mask` on a `RenderPipelineDescriptor`
    /// or a `RenderPassDescriptor` for this camera. Returns `None` if `views`
    /// is empty or longer than [`MAX_VIEW_COUNT`].
    pub fn view_mask(&self) -> Option<NonZeroU32> {
        let count = self.views.len();
        if count == 0 || count > MAX_VIEW_COUNT {
            return None;
        }
        // count == 32 sets every bit; smaller counts mask the low N bits.
        let mask = if count == 32 {
            u32::MAX
        } else {
            (1u32 << count) - 1
        };
        NonZeroU32::new(mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::Mat4;

    fn dummy_subview() -> MultiviewSubview {
        MultiviewSubview {
            view_from_camera: Transform::IDENTITY,
            clip_from_view: Mat4::IDENTITY,
        }
    }

    #[test]
    fn view_mask_empty_is_none() {
        let m = Multiview { views: vec![] };
        assert!(m.view_mask().is_none());
        assert!(m.view_count().is_none());
    }

    #[test]
    fn view_mask_two_eyes_sets_low_bits() {
        let m = Multiview {
            views: vec![dummy_subview(), dummy_subview()],
        };
        assert_eq!(m.view_mask().unwrap().get(), 0b11);
    }

    #[test]
    fn view_mask_at_max_sets_all_bits() {
        let m = Multiview {
            views: vec![dummy_subview(); MAX_VIEW_COUNT],
        };
        assert_eq!(m.view_mask().unwrap().get(), u32::MAX);
    }

    #[test]
    fn view_mask_above_max_is_none() {
        let m = Multiview {
            views: vec![dummy_subview(); MAX_VIEW_COUNT + 1],
        };
        assert!(m.view_mask().is_none());
    }
}

/// Per-layer data for a [`Multiview`] camera.
#[derive(Clone, Debug, Reflect)]
pub struct MultiviewSubview {
    /// Transform of this view relative to the camera's
    /// [`GlobalTransform`](bevy_transform::prelude::GlobalTransform).
    ///
    /// For stereo VR the canonical use is a small translation along the
    /// camera's local X axis (`±IPD/2`), optionally with a per-eye
    /// rotation if the headset's eye plates are canted.
    pub view_from_camera: Transform,
    /// Projection matrix for this view (`clip <- view`).
    ///
    /// Distinct from the camera's [`Projection`](crate::Projection) because
    /// VR runtimes typically supply asymmetric per-eye projections.
    pub clip_from_view: Mat4,
}
