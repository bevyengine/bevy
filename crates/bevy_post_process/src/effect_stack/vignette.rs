use bevy_camera::Camera;
use bevy_ecs::{
    component::Component,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    system::lifetimeless::Read,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{extract_component::ExtractComponent, render_resource::ShaderType};

/// The default vignette intensity amount.
const DEFAULT_VIGNETTE_INTENSITY: f32 = 0.50;

/// The default vignette radius amount.
const DEFAULT_VIGNETTE_RADIUS: f32 = 1.00;

/// The default vignette smoothness amount.
const DEFAULT_VIGNETTE_SMOOTHNESS: f32 = 0.50;

/// The default vignette roundness amount.
const DEFAULT_VIGNETTE_ROUNDNESS: f32 = 0.75;

/// Adds a gradual shading effect to the edges of the screen, drawing focus
/// towards the center.
///
/// A [Vignette] darkens the corners of the image relative to the center,
/// simulating the natural fall-off seen in camera lenses and human vision.
/// This effect is widely used in cinematography and games to direct the
/// player's attention or to evoke a specific mood (e.g., nostalgia or
/// claustrophobia).
///
/// Bevy's implementation applies a radial mask to the screen, modifying
/// the alpha channel or luminance of the final image. It supports adjusting
/// the size, roundness, and softness of the falloff, allowing you to
/// simulate various optical hardware or stylistic choices.
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default, Clone)]
pub struct Vignette {
    /// Controls the strength of the darkening effect.
    ///
    /// Range: `0.0` (No effect) to `1.0` (Fully black corners)
    ///
    /// The default value is 0.50
    pub intensity: f32,
    /// The size of the unvignetted center area.
    ///
    /// Range: `0.0` (Tiny center) to `2.0+` (Large center)
    ///
    /// The default value is 1.00
    pub radius: f32,
    /// The softness of the edge between the clear and dark areas.
    ///
    /// Range: `0.01` (Sharp edge) to `1.0` (Very soft edge)
    ///
    /// The default value is 0.50
    pub smoothness: f32,
    /// The shape of the vignette.
    ///
    /// `1.0` represents a perfect circle.
    ///
    /// The default value is 0.75
    pub roundness: f32,
}

impl Default for Vignette {
    fn default() -> Self {
        Self {
            intensity: DEFAULT_VIGNETTE_INTENSITY,
            radius: DEFAULT_VIGNETTE_RADIUS,
            smoothness: DEFAULT_VIGNETTE_SMOOTHNESS,
            roundness: DEFAULT_VIGNETTE_ROUNDNESS,
        }
    }
}

impl ExtractComponent for Vignette {
    type QueryData = Read<Vignette>;

    type QueryFilter = With<Camera>;

    type Out = Vignette;

    fn extract_component(vignette: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        // Skip the postprocessing phase entirely if the intensity is zero.
        if vignette.intensity > 0.0 {
            Some(vignette.clone())
        } else {
            None
        }
    }
}

#[derive(ShaderType, Default)]
pub struct VignetteUniform {
    /// Controls the strength of the darkening effect.
    pub(super) intensity: f32,
    /// The size of the unvignetted center area.
    pub(super) radius: f32,
    /// The softness of the edge between the clear and dark areas.
    pub(super) smoothness: f32,
    /// The shape of the vignette.
    pub(super) roundness: f32,
}
