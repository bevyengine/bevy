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

/// The default lens distortion intensity amount.
const DEFAULT_LENS_DISTORTION_INTENSITY: f32 = 0.5;

/// The default lens distortion scale amount.
const DEFAULT_LENS_DISTORTION_SCALE: f32 = 1.0;

/// The default lens distortion multiplier.
const DEFAULT_LENS_DISTORTION_MULTIPLIER: Vec2 = Vec2::ONE;

/// The default lens distortion center.
const DEFAULT_LENS_DISTORTION_CENTER: Vec2 = Vec2::splat(0.5);

/// The default lens distortion edge curvature.
const DEFAULT_LENS_DISTORTION_EDGE_CURVATURE: f32 = 0.0;

/// Simulates the warping of the image caused by real-world camera lenses.
///
/// [Lens distortion] simulates the imperfections of optical systems, where
/// straight lines in the real world appear curved in the image. This effect
/// is commonly used to create a sense of unease or disorientation, to mimic
/// specific camera equipment, or to enhance the scale and immersion of scenes.
///
/// Bevy's implementation is based on a simplified special case of the
/// Brown-Conrady model, where p₁ = p₂ = 0 and control is retained only
/// for k₁ and k₂.
#[derive(Reflect, Component, Clone)]
pub struct LensDistortion {
    /// The overall strength of the distortion effect.
    ///
    /// Positive values typically produce **barrel distortion** (bulging outwards),
    /// while negative values produce **pincushion distortion** (pinching inwards).
    /// This corresponds roughly to the radial distortion coefficient `k₁`
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
    /// `edge_curvature` provides indirect control over the k₂ parameter.
    /// The reason for indirect control is that k₁ and k₂ are typically correlated.
    /// If k₂ did not vary with k₁, it would easily cause visual jumping when intensity
    /// transitions from positive to negative. Furthermore, improper configuration of
    /// the k₂ parameter can result in unnatural distortion artifacts. In most cases,
    /// setting k₂ to 0.0 is appropriate.
    ///
    /// The default value is 0.0.
    pub edge_curvature: f32,
}

impl Default for LensDistortion {
    fn default() -> Self {
        Self {
            intensity: DEFAULT_LENS_DISTORTION_INTENSITY,
            scale: DEFAULT_LENS_DISTORTION_SCALE,
            multiplier: DEFAULT_LENS_DISTORTION_MULTIPLIER,
            center: DEFAULT_LENS_DISTORTION_CENTER,
            edge_curvature: DEFAULT_LENS_DISTORTION_EDGE_CURVATURE,
        }
    }
}

impl SyncComponent for LensDistortion {
    type Out = Self;
}

impl ExtractComponent for LensDistortion {
    type QueryData = Read<LensDistortion>;
    type QueryFilter = With<LensDistortion>;

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
    /// The overall strength of the distortion effect.
    pub(super) intensity: f32,
    /// A global scale factor applied to the final distorted image.
    pub(super) scale: f32,
    /// A multiplier that determines how the distortion scales along the X and Y axes.
    pub(super) multiplier: Vec2,
    /// The center point of the distortion effect in UV space `[0.0, 1.0]`.
    pub(super) center: Vec2,
    /// Controls the sharpness of the distortion curve near the screen edges.
    pub(super) edge_curvature: f32,
    /// Padding data.
    pub(super) unused: u32,
}
