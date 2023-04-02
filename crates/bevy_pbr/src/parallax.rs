use bevy_reflect::{FromReflect, Reflect};

/// The parallax mapping method to use to compute depth based on the
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
    /// Discovers the best depth value based on binary search.
    ///
    /// Each iteration incurs a texture sample.
    /// The result has fewer visual artifacts than `ParallaxOcclusionMapping`.
    ReliefMapping {
        /// How many additional steps to use at most to find the depth value.
        max_steps: u32,
    },
}
impl ParallaxMappingMethod {
    /// [`ReliefMapping`] with a 5 steps, a reasonable default.
    ///
    /// [`ReliefMapping`]: Self::ReliefMapping
    pub const DEFAULT_RELIEF_MAPPING: Self = ParallaxMappingMethod::ReliefMapping { max_steps: 5 };

    pub(crate) fn max_steps(&self) -> u32 {
        match self {
            ParallaxMappingMethod::ParallaxOcclusionMapping => 0,
            ParallaxMappingMethod::ReliefMapping { max_steps } => *max_steps,
        }
    }
}
