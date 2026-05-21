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
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone, Debug)]
pub struct Multiview {
    /// One entry per view layer. Must contain at least one element; the
    /// length determines how many array layers the render target needs.
    pub views: Vec<MultiviewSubview>,
}

impl Multiview {
    /// Returns the number of view layers as a `NonZeroU32`, or `None` if
    /// `views` is empty (in which case the component should be ignored).
    pub fn view_count(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.views.len() as u32)
    }

    /// Returns the multiview mask covering all layers (bits `0..view_count`).
    /// Suitable for passing as `multiview_mask` on a `RenderPipelineDescriptor`
    /// or a `RenderPassDescriptor` for this camera. Returns `None` if `views`
    /// is empty.
    pub fn view_mask(&self) -> Option<NonZeroU32> {
        let count = self.views.len();
        if count == 0 {
            return None;
        }
        // `count <= 32` is enforced at extraction time; here we just compute
        // the mask. For count == 32 we set every bit.
        let mask = if count >= 32 {
            u32::MAX
        } else {
            (1u32 << count) - 1
        };
        NonZeroU32::new(mask)
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
