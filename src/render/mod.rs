pub mod camera;
pub mod shader;
pub mod mesh;
mod forward;
mod shadow;
mod light;
mod pass;

pub use forward::{ForwardPass, ForwardUniforms};
pub use shadow::ShadowPass;
pub use light::*;
pub use shader::*;
pub use pass::*;

use wgpu::BindGroup;

pub struct UniformBuffer {
    pub buffer: wgpu::Buffer,
    pub size: u64,
}

pub struct Rendered {
    pub bind_group: Option<BindGroup>,
}