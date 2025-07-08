use bevy_color::Color;
use bevy_color::ColorToComponents;
use bevy_ecs::prelude::*;
use bevy_math::*;
use bevy_render::render_resource::BufferUsages;
use bevy_render::render_resource::*;
use bevy_render::renderer::RenderDevice;
use bevy_render::Extract;
use bytemuck::{Pod, Zeroable};

use crate::light::point_light_2d::{FalloffType, PointLight2D};
use crate::render::GlobalTransform;
use crate::render::SpritePipeline;

pub const MAX_POINT_LIGHTS_2D: usize = 32;

/// This structure is sent to the GPU for lighting calculations.
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Default, Pod, Zeroable)]
pub struct GpuPointLight2D {
    /// RGBA color multiplied by intensity.
    pub color_intensity: [f32; 4],
    /// XY position of the light, Z unused, W as radius.
    pub position_radius: [f32; 4],
}

/// A GPU buffer resource containing all point lights visible to the renderer.
#[derive(Resource)]
pub struct GpuLights2D {
    /// The GPU buffer that stores light data.
    pub buffer: Buffer,
    /// Bind group referencing the buffer for use in shaders.
    pub bind_group: BindGroup,
    /// Number of lights currently in the buffer.
    pub length: u32,
}

/// Light data transferred to the Render World
#[derive(Clone)]
pub struct ExtractedPointLight2D {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub falloff: FalloffType,
    pub position: Vec2,
}

/// Collection of all extracted 2D lights used for rendering.
#[derive(Resource, Default)]
pub struct ExtractedPointLights2D(pub Vec<ExtractedPointLight2D>);

/// Extracts `PointLight2D` components from the main world into the render world.
pub fn extract_point_lights_2d(
    mut extracted: ResMut<ExtractedPointLights2D>,
    query: Extract<Query<(&PointLight2D, &GlobalTransform)>>,
) {
    extracted.0.clear();
    for (light, transform) in &query {
        extracted.0.push(ExtractedPointLight2D {
            radius: light.radius,
            intensity: light.intensity,
            color: light.color,
            falloff: light.falloff,
            position: transform.translation().truncate(),
        });
    }
}

/// Writes light data into a buffer to prepare it for the GPU
pub fn prepare_point_lights_2d(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    extracted_lights: Res<ExtractedPointLights2D>,
    sprite_pipeline: Res<SpritePipeline>,
) {
    let mut lights = Vec::new();

    for light in &extracted_lights.0 {
        let linear = Vec4::from_array(light.color.to_srgba().to_f32_array());
        let color_intensity = Vec4::new(
            linear[0] * light.intensity,
            linear[1] * light.intensity,
            linear[2] * light.intensity,
            linear[3],
        );

        lights.push(GpuPointLight2D {
            color_intensity: color_intensity.to_array(),
            position_radius: [
                light.position.x,
                light.position.y,
                light.radius,
                match light.falloff {
                    FalloffType::Linear => 1.0,
                    FalloffType::Exponential => 2.0,
                },
            ],
        });
    }

    // Pad the array to 16 lights
    lights.resize(
        16,
        GpuPointLight2D {
            color_intensity: [0.0; 4],
            position_radius: [0.0; 4],
        },
    );

    // Create GPU buffer
    let light_bytes = bytemuck::cast_slice(&lights);
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Point Light 2D Buffer"),
        contents: light_bytes,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        Some("Point Light 2D BindGroup"),
        &sprite_pipeline.point_light_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    );

    // Insert the resource
    commands.insert_resource(GpuLights2D {
        buffer,
        bind_group,
        length: lights.len().min(16) as u32,
    });
}

/// Initializes the GPU resources used for 2D lighting.
///
/// This system creates an empty buffer and corresponding bind group, which are later
/// populated with actual light data during rendering.
/// It should run once during startup before any frame rendering occurs.
pub fn setup_gpu_lights(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    sprite_pipeline: Res<SpritePipeline>,
) {
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("GpuLights2D.buffer"),
        size: (size_of::<GpuPointLight2D>() * MAX_POINT_LIGHTS_2D) as u64,
        usage: BufferUsages::UNIFORM | BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = render_device.create_bind_group(
        Some("GpuLights2D.bind_group"),
        &sprite_pipeline.point_light_layout,
        &[BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    );

    commands.insert_resource(GpuLights2D {
        buffer,
        bind_group,
        length: 0,
    });
}
