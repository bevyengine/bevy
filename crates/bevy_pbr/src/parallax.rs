use bevy_reflect::Reflect;

/// The [parallax mapping] method to use to compute depth based on the
/// material's [`depth_map`].
///
/// Parallax Mapping uses a depth map texture to give the illusion of depth
/// variation on a mesh surface that is geometrically flat.
///
/// See the `parallax_mapping.wgsl` shader code for implementation details
/// and explanation of the methods used.
///
/// [`depth_map`]: crate::StandardMaterial::depth_map
/// [parallax mapping]: https://en.wikipedia.org/wiki/Parallax_mapping
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Reflect)]
pub enum ParallaxMappingMethod {
    /// A simple linear interpolation, using a single texture sample.
    ///
    /// This method is named "Parallax Occlusion Mapping".
    ///
    /// Unlike [`ParallaxMappingMethod::Relief`], only requires a single lookup,
    /// but may skip small details and result in writhing material artifacts.
    #[default]
    Occlusion,
    /// Discovers the best depth value based on binary search.
    ///
    /// Each iteration incurs a texture sample.
    /// The result has fewer visual artifacts than [`ParallaxMappingMethod::Occlusion`].
    ///
    /// This method is named "Relief Mapping".
    Relief {
        /// How many additional steps to use at most to find the depth value.
        max_steps: u32,
    },
}
impl ParallaxMappingMethod {
    /// [`ParallaxMappingMethod::Relief`] with a 5 steps, a reasonable default.
    pub const DEFAULT_RELIEF_MAPPING: Self = ParallaxMappingMethod::Relief { max_steps: 5 };

    pub(crate) fn max_steps(&self) -> u32 {
        match self {
            ParallaxMappingMethod::Occlusion => 0,
            ParallaxMappingMethod::Relief { max_steps } => *max_steps,
        }
    }
}
