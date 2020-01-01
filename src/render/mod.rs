pub mod camera;
pub mod shader;
pub mod mesh;
pub mod render_resources;
mod forward;
mod forward_shadow;
mod forward_instanced;
mod shadow;
mod light;
mod pipeline;
mod pass;
mod material;

pub use forward::{ForwardUniforms, ForwardPipelineNew, ForwardPass};
pub use forward_shadow::{ForwardShadowPass};
pub use forward_instanced::ForwardInstancedPass;
pub use shadow::ShadowPass;
pub use light::*;
pub use shader::*;
pub use pipeline::*;
pub use pass::*;
pub use material::*;
pub use mesh::*;
pub use camera::*;
pub use render_resources::RenderResources;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
}

impl UniformBuffer {
    pub fn get_binding_resource<'a>(&'a self) -> wgpu::BindingResource<'a> {
        wgpu::BindingResource::Buffer {
            buffer: &self.buffer,
            range: 0 .. self.size,
        }
    }
}