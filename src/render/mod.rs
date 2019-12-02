pub mod camera;
pub mod shader;
mod forward;
mod shadow;
mod light;
mod pass;

pub use forward::{ForwardPass, ForwardUniforms};
pub use shadow::ShadowPass;
pub use light::*;
pub use shader::*;
pub use pass::*;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
}