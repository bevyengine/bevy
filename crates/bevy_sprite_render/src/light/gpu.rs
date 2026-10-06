use bevy_color::ColorToComponents;
use bevy_ecs::prelude::*;
use bevy_math::{Vec2, Vec3, Vec4};
use bevy_render::{
    render_resource::{ShaderSize, ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_sprite_light::GlobalAmbientLight2d;
use bevy_utils::once;
use tracing::warn;

use super::point_light::ExtractedPointLight2d;

pub const MAX_POINT_LIGHTS_2D: usize = 256;

#[derive(Default, Clone, Copy, ShaderType)]
pub struct GpuPointLight2d {
    pub position: Vec2,
    pub range: f32,
    pub intensity: f32,
    pub color: Vec4,
}

#[derive(Clone, ShaderType)]
pub struct Lights2dUniform {
    pub ambient: Vec3,
    pub point_light_count: u32,
    pub point_lights: [GpuPointLight2d; MAX_POINT_LIGHTS_2D],
}

// WebGL2 only guarantees 16 KiB per uniform binding.
const _: () = assert!(Lights2dUniform::SHADER_SIZE.get() <= 16384);

impl Default for Lights2dUniform {
    fn default() -> Self {
        Self {
            ambient: Vec3::ZERO,
            point_light_count: 0,
            point_lights: [GpuPointLight2d::default(); MAX_POINT_LIGHTS_2D],
        }
    }
}

#[derive(Resource, Default)]
pub struct Lights2dBuffer {
    pub buffer: UniformBuffer<Lights2dUniform>,
}

pub fn prepare_lights_2d_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut lights_buffer: ResMut<Lights2dBuffer>,
    ambient: Res<GlobalAmbientLight2d>,
    point_lights: Query<&ExtractedPointLight2d>,
) {
    let uniform = lights_buffer.buffer.get_mut();
    uniform.ambient = ambient.color.to_linear().to_vec3() * ambient.brightness;

    let mut count = 0;

    for light in &point_lights {
        if count == MAX_POINT_LIGHTS_2D {
            once!(warn!(
                "More than {MAX_POINT_LIGHTS_2D} `PointLight2d`s are visible. The rest will be ignored."
            ));
            break;
        }
        uniform.point_lights[count] = GpuPointLight2d {
            position: light.transform.translation().truncate(),
            range: light.range,
            intensity: light.intensity,
            color: light.color.to_linear().to_vec4(),
        };
        count += 1;
    }

    uniform.point_light_count = count as u32;

    lights_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
}
