pub mod camera;
pub mod shader;
pub mod mesh;
mod forward;
mod shadow;
mod light;
mod pass;
mod material;

pub use forward::{ForwardPass, ForwardUniforms};
pub use shadow::ShadowPass;
pub use light::*;
pub use shader::*;
pub use pass::*;
pub use material::*;
pub use mesh::*;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
}