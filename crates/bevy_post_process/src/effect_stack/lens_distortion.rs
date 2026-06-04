use bevy_ecs::{
    component::Component,
    query::{QueryItem, With},
    system::lifetimeless::Read,
};
use bevy_math::{ops::abs, Vec2};
use bevy_reflect::Reflect;
use bevy_render::{
    extract_component::ExtractComponent, render_resource::ShaderType, sync_component::SyncComponent,
};

/// Simulates the warping of the image caused by real-world camera lenses.
///
/// [Lens distortion] simulates the imperfections of optical systems, where
/// straight lines in the real world appear curved in the image. This effect
/// is commonly used to create a sense of unease or disorientation, to mimic
/// specific camera equipment, or to enhance the scale and immersion of scenes.
///
/// Bevy's implementation is based on a simplified special case of the
/// Brown-Conrady model, where p₁ = p₂ = 0 and control is retained only
/// for k1 and k2.
#[derive(Reflect, Component, Clone)]
pub struct LensDistortion {
    /// The overall strength of the distortion effect.
    ///
    /// Positive values typically produce **barrel distortion** (bulging outwards),
    /// while negative values produce **pincushion distortion** (pinching inwards).
    /// This corresponds roughly to the radial distortion coefficient `k1`
    /// in the simplified model.
    ///
    /// The default value is 0.5.
    pub intensity: f32,
    /// A global scale factor applied to the final distorted image.
    ///
    /// Strong distortion pushes pixels away from the center or pulls them in,
    /// resulting in visible **stretching artifacts** at the screen edges.
    /// Increasing this value zooms in to **crop out** these extreme edge artifacts,
    /// ensuring the screen remains fully covered at the cost of a tighter field of view.
    ///
    /// The default value is 1.0(No zoom).
    pub scale: f32,
    /// A multiplier that determines how the distortion scales along the X and Y axes.
    ///
    /// By default, this should be `Vec2::ONE` for uniform radial distortion.
    /// Modifying these values allows for anamorphic-like effects where the distortion
    /// is stronger on one axis than the other. When a component of multiplier is set to 0.0,
    /// no distortion effect is applied.
    ///
    /// The default value is `Vec2::ONE`
    pub multiplier: Vec2,
    /// The center point of the distortion effect in UV space `[0.0, 1.0]`.
    ///
    /// Distortion radiates outward or inward from this point.
    ///
    /// The default value is `Vec2::splat(0.5)`
    pub center: Vec2,
    /// Controls the sharpness of the distortion curve near the screen edges.
    ///
    /// `edge_curvature` provides indirect control over the k2 parameter.
    /// The reason for indirect control is that k1 and k2 are typically correlated.
    /// If k2 did not vary with k1, it would easily cause visual jumping when intensity
    /// transitions from positive to negative.
    /// For a simple and natural look in most cases, we recommend setting `edge_curvature` to 0.0.
    ///
    /// The default value is 0.0.
    pub edge_curvature: f32,
}

impl Default for LensDistortion {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            scale: 1.0,
            multiplier: Vec2::ONE,
            center: Vec2::splat(0.5),
            edge_curvature: 0.0,
        }
    }
}

impl SyncComponent for LensDistortion {
    type Target = Self;
}

impl ExtractComponent for LensDistortion {
    type QueryData = Read<LensDistortion>;
    type QueryFilter = With<LensDistortion>;
    type Out = Self;

    fn extract_component(lens_distortion: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        // Skip the postprocessing phase entirely if the intensity is negligible.
        if abs(lens_distortion.intensity) > 1e-4 {
            Some(lens_distortion.clone())
        } else {
            None
        }
    }
}

/// The on-GPU version of the [`LensDistortion`] settings.
///
/// See the documentation for [`LensDistortion`] for more information on
/// each of these fields.
#[derive(ShaderType, Default)]
pub struct LensDistortionUniform {
    pub(super) intensity: f32,
    pub(super) scale: f32,
    pub(super) multiplier: Vec2,
    pub(super) center: Vec2,
    pub(super) edge_curvature: f32,
    pub(super) unused: u32,
}
