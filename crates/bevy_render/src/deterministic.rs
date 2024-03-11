use bevy_ecs::system::Resource;

/// Configure deterministic rendering to fix flickering due to z-fighting.
#[derive(Resource, Default)]
pub struct DeterministicRenderingConfig {
    /// Sort visible entities by id before rendering to avoid flickering.
    ///
    /// Render is parallel by default, and if there's z-fighting, it may cause flickering.
    /// Default fix for the issue is to set `depth_bias` per material.
    /// When it is not possible, entities sorting can be used.
    ///
    /// This option costs performance and disabled by default.
    pub stable_sort_z_fighting: bool,
}
