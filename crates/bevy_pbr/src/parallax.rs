use bevy_reflect::{FromReflect, Reflect};

/// The parallax mapping method to use to compute a displacement based on the
/// material's [`depth_map`].
///
/// See the `parallax_mapping.wgsl` shader code for implementation details
/// and explanation of the methods used.
///
/// [`depth_map`]: crate::StandardMaterial::depth_map
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Reflect, FromReflect)]
pub enum ParallaxMappingMethod {
    /// A simple linear interpolation, using a single texture sample.
    #[default]
    ParallaxOcclusionMapping,
    /// A discovery of 5 iterations of the best displacement
    /// value. Each iteration incurs a texture sample.
    ///
    /// The result has fewer visual artifacts than `ParallaxOcclusionMapping`.
    ReliefMapping { n_steps: u32 },
}
