use crate::DirectionalLight;
use bevy_core::FrameCount;
use bevy_math::{Vec3, Vec4};
use bevy_render::{
    render_resource::{ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_transform::components::GlobalTransform;

#[derive(ShaderType)]
pub struct GpuSolariMaterial {
    pub base_color: Vec4,
    pub base_color_map_index: u32,
    pub normal_map_index: u32,
    pub emissive: Vec3,
    pub emissive_map_index: u32,
}

#[derive(ShaderType)]
pub struct SolariUniforms {
    pub frame_count: u32,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
}

impl SolariUniforms {
    pub fn new(
        frame_count: &FrameCount,
        sun: (&DirectionalLight, &GlobalTransform),
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> UniformBuffer<Self> {
        let sun_color = sun.0.color.as_linear_rgba_f32();
        let uniforms = Self {
            frame_count: frame_count.0,
            sun_direction: sun.1.back(),
            sun_color: Vec3::new(sun_color[0], sun_color[1], sun_color[2]) * sun.0.illuminance,
        };

        let mut buffer = UniformBuffer::from(uniforms);
        buffer.set_label(Some("solari_uniforms"));
        buffer.write_buffer(render_device, render_queue);
        buffer
    }
}
