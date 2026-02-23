//! Tileable 3D Perlin–Worley Noise Generation for Volumetric Clouds
//!
//! Generates a 3D texture (RGBA) similar to the reference Shadertoy:
//! - R: Perlin–Worley (remapped billowy Perlin by low-frequency Worley FBM)
//! - G: Worley FBM (base frequency)
//! - B: Worley FBM (2x frequency)
//! - A: Worley FBM (4x frequency)

use bevy_asset::load_embedded_asset;
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::UVec3;
use bevy_render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
};
use bevy_utils::default;

/// Parameters for controlling Perlin–Worley noise generation.
#[derive(Clone, Copy, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct PerlinWorleyNoiseParams {
    /// Base frequency / tile period (must be > 0).
    pub base_frequency: u32,
    /// Number of octaves for Perlin FBM.
    pub perlin_octaves: u32,
    /// Z animation offset (in texture space).
    pub z_offset: f32,
    /// Padding.
    pub _pad0: f32,
}

impl Default for PerlinWorleyNoiseParams {
    fn default() -> Self {
        Self {
            base_frequency: 4,
            perlin_octaves: 7,
            z_offset: 0.0,
            _pad0: 0.0,
        }
    }
}

/// Size of the 3D noise texture.
#[derive(Clone, Copy)]
pub struct PerlinWorleyTextureSize {
    pub size: UVec3,
}

impl Default for PerlinWorleyTextureSize {
    fn default() -> Self {
        Self {
            // Reference uses 128 slices; this is ~16MB at RGBA16F.
            size: UVec3::new(32, 32, 32),
        }
    }
}

#[derive(Resource)]
pub struct PerlinWorleyNoiseBindGroupLayout {
    pub layout: BindGroupLayout,
    pub descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for PerlinWorleyNoiseBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let descriptor = BindGroupLayoutDescriptor::new(
            "perlin_worley_noise_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        // 3D noise texture storage
                        13,
                        texture_storage_3d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                    (
                        // params uniform buffer
                        14,
                        uniform_buffer::<PerlinWorleyNoiseParams>(false),
                    ),
                ),
            ),
        );

        let layout =
            render_device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

#[derive(Resource)]
pub struct PerlinWorleyNoisePipeline {
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for PerlinWorleyNoisePipeline {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layout = world.resource::<PerlinWorleyNoiseBindGroupLayout>();

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("perlin_worley_noise_3d_pipeline".into()),
            layout: vec![layout.descriptor.clone()],
            shader: load_embedded_asset!(world, "perlin_worley_noise_3d.wgsl"),
            ..default()
        });

        Self { pipeline }
    }
}

/// The generated 3D Perlin–Worley texture.
#[derive(Resource)]
pub struct PerlinWorleyNoiseTexture {
    pub texture: CachedTexture,
    pub size: UVec3,
}

#[derive(Resource)]
pub struct PerlinWorleyNoiseBindGroup {
    pub bind_group: BindGroup,
}

#[derive(Resource)]
pub struct PerlinWorleyNoiseParamsBuffer {
    pub buffer: Buffer,
}

/// Tracks if the 3D noise has been generated.
#[derive(Resource, Default)]
pub struct PerlinWorleyNoiseGenerated(pub bool);

pub fn init_perlin_worley_noise_texture(
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut commands: bevy_ecs::system::Commands,
) {
    let size = PerlinWorleyTextureSize::default();

    let texture_descriptor = TextureDescriptor {
        label: Some("perlin_worley_noise_3d_texture"),
        size: Extent3d {
            width: size.size.x,
            height: size.size.y,
            depth_or_array_layers: size.size.z,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D3,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };

    let texture = texture_cache.get(&render_device, texture_descriptor);
    commands.insert_resource(PerlinWorleyNoiseTexture {
        texture,
        size: size.size,
    });
}

pub fn init_perlin_worley_noise_params_buffer(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
) {
    let params = PerlinWorleyNoiseParams::default();
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("perlin_worley_noise_params_buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    commands.insert_resource(PerlinWorleyNoiseParamsBuffer { buffer });
}

pub fn prepare_perlin_worley_noise_bind_group(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
    layout: Res<PerlinWorleyNoiseBindGroupLayout>,
    texture: Res<PerlinWorleyNoiseTexture>,
    params_buffer: Res<PerlinWorleyNoiseParamsBuffer>,
) {
    let bind_group = render_device.create_bind_group(
        "perlin_worley_noise_bind_group",
        &layout.layout,
        &BindGroupEntries::with_indices((
            (13, &texture.texture.default_view),
            (14, params_buffer.buffer.as_entire_binding()),
        )),
    );

    commands.insert_resource(PerlinWorleyNoiseBindGroup { bind_group });
}

pub fn get_3d_dispatch_size(texture_size: UVec3) -> UVec3 {
    const WORKGROUP_SIZE: u32 = 4;
    UVec3::new(
        texture_size.x.div_ceil(WORKGROUP_SIZE),
        texture_size.y.div_ceil(WORKGROUP_SIZE),
        texture_size.z.div_ceil(WORKGROUP_SIZE),
    )
}

/// Generate the Perlin–Worley 3D texture once (after the pipeline is ready).
pub fn generate_perlin_worley_noise_once(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<PerlinWorleyNoisePipeline>,
    bind_group: Option<Res<PerlinWorleyNoiseBindGroup>>,
    texture: Res<PerlinWorleyNoiseTexture>,
    mut generated: ResMut<PerlinWorleyNoiseGenerated>,
) {
    if generated.0 {
        return;
    }

    let Some(bind_group) = bind_group else {
        return;
    };

    let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
        return;
    };

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("perlin_worley_noise_generation"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("perlin_worley_noise_3d_pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group.bind_group, &[]);

        let dispatch_size = get_3d_dispatch_size(texture.size);
        compute_pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, dispatch_size.z);
    }

    let command_buffer = encoder.finish();
    render_queue.submit([command_buffer]);
    generated.0 = true;
}
