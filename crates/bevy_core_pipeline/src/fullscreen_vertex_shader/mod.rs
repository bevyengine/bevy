use bevy_asset::HandleUntyped;
use bevy_reflect::TypeUuid;
use bevy_render::{prelude::Shader, render_resource::VertexState};

pub const FULLSCREEN_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7837534426033940724);

/// uses the [`FULLSCREEN_SHADER_HANDLE`] to output a
/// ```wgsl
/// struct FullscreenVertexOutput {
///     [[builtin(position)]]
///     position: vec4<f32>;
///     [[location(0)]]
///     uv: vec2<f32>;
/// };
/// ```
/// from the vertex shader.
/// The draw call should render one triangle: `render_pass.draw(0..3, 0..1);`
pub fn fullscreen_shader_vertex_state() -> VertexState {
    VertexState {
        shader: FULLSCREEN_SHADER_HANDLE.typed(),
        shader_defs: Vec::new(),
        entry_point: "fullscreen_vertex_shader".into(),
        buffers: Vec::new(),
    }
}
