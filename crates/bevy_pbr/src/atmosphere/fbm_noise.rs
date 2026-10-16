//! 3D FBM Noise Generation for Atmospheric Effects
//!
//! This module generates a 3D texture filled with Fractional Brownian Motion (FBM) noise,
//! which can be used for atmospheric effects like clouds, fog, and volumetric effects.

use bevy_asset::load_embedded_asset;
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::CachedTexture,
};
use bevy_utils::default;

/// Parameters for controlling FBM noise generation
#[derive(Clone, Copy, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct FbmNoiseParams {
    /// Number of octaves for the FBM noise (more octaves = more detail)
    pub octaves: u32,
    /// Base frequency of the noise
    pub frequency: f32,
    /// Base amplitude of the noise
    pub amplitude: f32,
    /// Frequency multiplier for each octave (typically 2.0)
    pub lacunarity: f32,
    /// Amplitude multiplier for each octave (typically 0.5)
    pub persistence: f32,
    /// Cloud life-cycle phase, `evolution_time / lifetime + 0.5` ([`crate::CloudLayer`]).
    pub life_t: f32,
    /// 1 when the life cycle is on (`lifetime > 0`).
    pub life_enabled: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

impl Default for FbmNoiseParams {
    fn default() -> Self {
        Self {
            octaves: 4,
            frequency: 1.0,
            amplitude: 1.0,
            lacunarity: 2.0,
            persistence: 0.5,
            life_t: 0.5,
            life_enabled: 0,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

/// Size of the 2D noise texture
#[derive(Clone, Copy)]
pub struct NoiseTextureSize {
    pub size: UVec2,
}

impl Default for NoiseTextureSize {
    fn default() -> Self {
        Self {
            // Procedural weather map (design.md §1.3 / §3.2): 512² RGBA.
            // R coverage, G type, B density scale, A spare.
            size: UVec2::new(512, 512),
        }
    }
}

/// Resource containing the bind group layout for the FBM noise pass
#[derive(Resource)]
pub struct FbmNoiseBindGroupLayout {
    pub layout: BindGroupLayout,
    pub descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for FbmNoiseBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let descriptor = BindGroupLayoutDescriptor::new(
            "fbm_noise_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        // 2D noise texture storage
                        13,
                        texture_storage_2d(
                            // Weather map (coverage, type, density scale, spare).
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                    (
                        // FBM parameters uniform buffer
                        14,
                        uniform_buffer::<FbmNoiseParams>(false),
                    ),
                ),
            ),
        );

        let layout =
            render_device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

/// Resource containing the compute pipeline for FBM noise generation
#[derive(Resource)]
pub struct FbmNoisePipeline {
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for FbmNoisePipeline {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layout = world.resource::<FbmNoiseBindGroupLayout>();

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("fbm_noise_2d_pipeline".into()),
            layout: vec![layout.descriptor.clone()],
            shader: load_embedded_asset!(world, "fbm_noise_3d.wesl"),
            ..default()
        });

        Self { pipeline }
    }
}

/// Resource containing the generated 2D noise texture
#[derive(Resource)]
pub struct FbmNoiseTexture {
    pub texture: CachedTexture,
    pub size: UVec2,
}

/// Resource containing the bind group for the noise generation pass
#[derive(Resource)]
pub struct FbmNoiseBindGroup {
    pub bind_group: BindGroup,
}

/// Resource containing the uniform buffer for FBM parameters
#[derive(Resource)]
pub struct FbmNoiseParamsBuffer {
    pub buffer: Buffer,
}

/// System to initialize the FBM noise texture.
///
/// Allocates GPU memory directly instead of the render [`TextureCache`](bevy_render::texture::TextureCache)
/// so this `RenderStartup` system does not need to run after `init_gpu_resource::<TextureCache>`.
pub fn init_fbm_noise_texture(
    render_device: Res<RenderDevice>,
    mut commands: bevy_ecs::system::Commands,
) {
    let size = NoiseTextureSize::default();

    let texture_descriptor = TextureDescriptor {
        label: Some("cloud_weather_map_texture"),
        size: Extent3d {
            width: size.size.x,
            height: size.size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        // Weather map (coverage, type, density scale, spare).
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[],
    };

    let texture = render_device.create_texture(&texture_descriptor);
    let default_view = texture.create_view(&TextureViewDescriptor::default());

    commands.insert_resource(FbmNoiseTexture {
        texture: CachedTexture {
            texture,
            default_view,
        },
        size: size.size,
    });
}

/// System to prepare the FBM noise bind group
pub fn prepare_fbm_noise_bind_group(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
    layout: Res<FbmNoiseBindGroupLayout>,
    texture: Res<FbmNoiseTexture>,
    params_buffer: Res<FbmNoiseParamsBuffer>,
) {
    let bind_group = render_device.create_bind_group(
        "fbm_noise_bind_group",
        &layout.layout,
        &BindGroupEntries::with_indices((
            (13, &texture.texture.default_view),
            (14, params_buffer.buffer.as_entire_binding()),
        )),
    );

    commands.insert_resource(FbmNoiseBindGroup { bind_group });
}

/// System to initialize the FBM noise parameters buffer
pub fn init_fbm_noise_params_buffer(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
) {
    let params = FbmNoiseParams::default();
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("fbm_noise_params_buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    commands.insert_resource(FbmNoiseParamsBuffer { buffer });
}

/// Returns the dispatch workgroup counts for the 2D noise texture based on its size
pub fn get_noise_dispatch_size(texture_size: UVec2) -> UVec2 {
    const WORKGROUP_SIZE: u32 = 8;
    UVec2::new(
        texture_size.x.div_ceil(WORKGROUP_SIZE),
        texture_size.y.div_ceil(WORKGROUP_SIZE),
    )
}

/// Resource to track if noise has been generated
#[derive(Resource, Default)]
pub struct NoiseGenerated(pub bool);

/// System to generate the weather map: once, and again whenever the cloud life-cycle clock
/// ([`crate::CloudLayer::evolution_time`]) advances (the map holds the blended generations).
pub fn generate_fbm_noise_once(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<FbmNoisePipeline>,
    bind_group: Option<Res<FbmNoiseBindGroup>>,
    texture: Res<FbmNoiseTexture>,
    params_buffer: Option<Res<FbmNoiseParamsBuffer>>,
    layers: bevy_ecs::system::Query<&crate::CloudLayer>,
    mut last_life_t: bevy_ecs::system::Local<Option<f32>>,
    mut noise_generated: ResMut<NoiseGenerated>,
) {
    let life = layers
        .iter()
        .next()
        .filter(|layer| layer.lifetime > 0.0)
        .map(|layer| layer.evolution_time / layer.lifetime + 0.5);
    // Re-bake only when the phase moved by > 1/2000 of a cycle (a weight change far below
    // visibility; at 1x with a 20 min life time that is every ~0.6 s).
    let unchanged = match (life, *last_life_t) {
        (Some(t), Some(last)) => (t - last).abs() < 5e-4,
        (None, _) => true,
        _ => false,
    };
    if noise_generated.0 && unchanged {
        return;
    }
    if let Some(params_buffer) = params_buffer.as_ref() {
        let params = FbmNoiseParams {
            life_t: life.unwrap_or(0.5),
            life_enabled: life.is_some() as u32,
            ..FbmNoiseParams::default()
        };
        render_queue.write_buffer(&params_buffer.buffer, 0, bytemuck::cast_slice(&[params]));
    }

    let Some(bind_group) = bind_group else {
        return;
    };

    let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
        return;
    };

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("fbm_noise_generation"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("fbm_noise_2d_pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group.bind_group, &[]);

        let dispatch_size = get_noise_dispatch_size(texture.size);
        compute_pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, 1);
    }

    let command_buffer = encoder.finish();
    render_queue.submit([command_buffer]);

    noise_generated.0 = true;
    *last_life_t = life;
}
