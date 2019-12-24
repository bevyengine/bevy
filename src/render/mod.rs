pub mod camera;
pub mod shader;
pub mod mesh;
mod forward;
mod forward_shadow;
mod shadow;
mod light;
mod pass;
mod material;
mod render_resources;

pub use forward_shadow::{ForwardShadowPass};
pub use forward::{ForwardPass, ForwardUniforms};
pub use shadow::ShadowPass;
pub use light::*;
pub use shader::*;
pub use pass::*;
pub use material::*;
pub use mesh::*;
pub use camera::*;
pub use render_resources::RenderResources;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
}