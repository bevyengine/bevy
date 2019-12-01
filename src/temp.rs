use std::{rc::Rc, ops::Range};
use zerocopy::{AsBytes, FromBytes};
use crate::math;


pub fn opengl_to_wgpu_matrix() -> math::Mat4 {
    math::mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}
pub struct Entity {
    pub mx_world: math::Mat4,
    pub rotation_speed: f32,
    pub color: wgpu::Color,
    pub vertex_buf: Rc<wgpu::Buffer>,
    pub index_buf: Rc<wgpu::Buffer>,
    pub index_count: usize,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buf: wgpu::Buffer,
}

pub struct Light {
    pub pos: math::Vec3,
    pub color: wgpu::Color,
    pub fov: f32,
    pub depth: Range<f32>,
    pub target_view: wgpu::TextureView,
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct LightRaw {
    pub proj: [[f32; 4]; 4],
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

impl Light {
    pub fn to_raw(&self) -> LightRaw {
        LightRaw {
            proj: generate_matrix(&self.pos, self.fov, 1.0, self.depth.start, self.depth.end).into(),
            pos: [self.pos.x, self.pos.y, self.pos.z, 1.0],
            color: [
                self.color.r as f32,
                self.color.g as f32,
                self.color.b as f32,
                1.0,
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct EntityUniforms {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

#[repr(C)]
pub struct ShadowUniforms {
    pub proj: [[f32; 4]; 4],
}

// pub struct Pass {
//     pub pipeline: wgpu::RenderPipeline,
//     pub bind_group: wgpu::BindGroup,
//     pub uniform_buf: wgpu::Buffer,
// }

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

pub fn load_glsl(code: &str, stage: ShaderStage) -> Vec<u32> {
    let ty = match stage {
        ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
        ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
        ShaderStage::Compute => glsl_to_spirv::ShaderType::Compute,
    };

    wgpu::read_spirv(glsl_to_spirv::compile(&code, ty).unwrap()).unwrap()
}

pub fn generate_matrix(eye: &math::Vec3, fov: f32, aspect_ratio: f32, near: f32, far: f32) -> math::Mat4 {
    let projection = math::perspective(aspect_ratio, fov, near, far);

    let view = math::look_at_rh::<f32>(
        &eye,
        &math::vec3(0.0, 0.0, 0.0),
        &math::vec3(0.0, 0.0, 1.0),
    );

    opengl_to_wgpu_matrix() * projection * view
}
