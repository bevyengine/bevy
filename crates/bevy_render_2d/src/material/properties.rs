use bevy_render::render_phase::DrawFunctionId;

use crate::{material::AlphaMode2d, mesh_pipeline::pipeline::Mesh2dPipelineKey};

/// Common [`Material2d`](super::Material2d) properties, calculated for a specific material instance.
pub struct Material2dProperties {
    /// The [`AlphaMode2d`] of this material.
    pub alpha_mode: AlphaMode2d,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may
    pub depth_bias: f32,
    /// The bits in the [`Mesh2dPipelineKey`] for this material.
    ///
    /// [`Mesh2dPipelineKey`] are precalculated so that we can just "or" them together.
    pub mesh_pipeline_key_bits: Mesh2dPipelineKey,
    pub draw_function_id: DrawFunctionId,
}
