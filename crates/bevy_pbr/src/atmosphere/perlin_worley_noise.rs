//! Tileable 3D Perlin–Worley Noise Generation for Volumetric Clouds
//!
//! Generates a 3D texture (RGBA) similar to the reference Shadertoy / Nubis shape noise:
//! - R: Perlin–Worley (remapped billowy Perlin by low-frequency Worley FBM)
//! - G: Worley FBM (base frequency)
//! - B: Worley FBM (2x frequency)
//! - A: Worley FBM (4x frequency)
//!
//! Default size is 128³ with a full mip chain (box-filter downsampled) for LOD sampling.

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
    texture::CachedTexture,
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
            // Nubis/HZD shape noise: 128³ RGBA (design.md §1.3). RGBA16F keeps
            // the existing storage/sample path; ~16 MB base + mips.
            size: UVec3::new(128, 128, 128),
        }
    }
}

fn mip_count_for_size(size: UVec3) -> u32 {
    let min_dim = size.x.min(size.y).min(size.z).max(1);
    min_dim.ilog2() + 1
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
                        // 3D noise texture storage (single mip)
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
pub struct NoiseMipDownsampleBindGroupLayout {
    pub layout: BindGroupLayout,
    pub descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for NoiseMipDownsampleBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let descriptor = BindGroupLayoutDescriptor::new(
            "noise_mip_downsample_3d_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        13,
                        texture_3d(TextureSampleType::Float { filterable: false }),
                    ),
                    (
                        14,
                        texture_storage_3d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
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
            shader: load_embedded_asset!(world, "perlin_worley_noise_3d.wesl"),
            ..default()
        });

        Self { pipeline }
    }
}

#[derive(Resource)]
pub struct NoiseMipDownsamplePipeline {
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for NoiseMipDownsamplePipeline {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layout = world.resource::<NoiseMipDownsampleBindGroupLayout>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("noise_mip_downsample_3d_pipeline".into()),
            layout: vec![layout.descriptor.clone()],
            shader: load_embedded_asset!(world, "noise_mip_downsample_3d.wesl"),
            ..default()
        });
        Self { pipeline }
    }
}

/// The generated 3D Perlin–Worley texture (full mip chain).
#[derive(Resource)]
pub struct PerlinWorleyNoiseTexture {
    pub texture: CachedTexture,
    pub size: UVec3,
    pub mip_level_count: u32,
    /// Per-mip storage/sample views (one level each) for generation and downsample.
    pub mip_views: Vec<TextureView>,
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
/// Progressive generation stage for the 128³ shape noise.
///
/// `0` = generate base mip; `1..mip_level_count-1` = downsample that mip;
/// `>= mip_level_count` = finished. Spreads the one-time stall across frames
/// (design.md revisions / progress known issues).
#[derive(Resource, Default)]
pub struct PerlinWorleyNoiseGenerated(pub u32);

/// Allocates GPU memory directly for the same reason as `init_fbm_noise_texture` in `fbm_noise`.
pub fn init_perlin_worley_noise_texture(
    render_device: Res<RenderDevice>,
    mut commands: bevy_ecs::system::Commands,
) {
    let size = PerlinWorleyTextureSize::default();
    let mip_level_count = mip_count_for_size(size.size);

    let texture_descriptor = TextureDescriptor {
        label: Some("perlin_worley_noise_3d_texture"),
        size: Extent3d {
            width: size.size.x,
            height: size.size.y,
            depth_or_array_layers: size.size.z,
        },
        mip_level_count,
        sample_count: 1,
        dimension: TextureDimension::D3,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[],
    };

    let texture = render_device.create_texture(&texture_descriptor);
    let default_view = texture.create_view(&TextureViewDescriptor::default());
    let mut mip_views = Vec::with_capacity(mip_level_count as usize);
    for mip in 0..mip_level_count {
        mip_views.push(texture.create_view(&TextureViewDescriptor {
            label: Some(&format!("perlin_worley_noise_3d_mip{mip}")),
            format: Some(TextureFormat::Rgba16Float),
            dimension: Some(TextureViewDimension::D3),
            aspect: TextureAspect::All,
            base_mip_level: mip,
            mip_level_count: Some(1),
            ..default()
        }));
    }
    commands.insert_resource(PerlinWorleyNoiseTexture {
        texture: CachedTexture {
            texture,
            default_view,
        },
        size: size.size,
        mip_level_count,
        mip_views,
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
    // Bind mip 0 for the base-frequency generation pass.
    let bind_group = render_device.create_bind_group(
        "perlin_worley_noise_bind_group",
        &layout.layout,
        &BindGroupEntries::with_indices((
            (13, &texture.mip_views[0]),
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

/// Generate the Perlin–Worley 3D texture once (after the pipeline is ready), then fill mips.
pub fn generate_perlin_worley_noise_once(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<PerlinWorleyNoisePipeline>,
    mip_pipeline: Res<NoiseMipDownsamplePipeline>,
    bind_group: Option<Res<PerlinWorleyNoiseBindGroup>>,
    mip_layout: Res<NoiseMipDownsampleBindGroupLayout>,
    texture: Res<PerlinWorleyNoiseTexture>,
    mut generated: ResMut<PerlinWorleyNoiseGenerated>,
) {
    // Stage 0 = base; 1..mip_count-1 = that mip; >= mip_count = done.
    if generated.0 >= texture.mip_level_count {
        return;
    }

    let Some(bind_group) = bind_group else {
        return;
    };

    let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
        return;
    };
    let Some(mip_compute) = pipeline_cache.get_compute_pipeline(mip_pipeline.pipeline) else {
        return;
    };

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("perlin_worley_noise_generation"),
    });

    if generated.0 == 0 {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("perlin_worley_noise_3d_pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group.bind_group, &[]);
        let dispatch_size = get_3d_dispatch_size(texture.size);
        compute_pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, dispatch_size.z);
        drop(compute_pass);
        let command_buffer = encoder.finish();
        render_queue.submit([command_buffer]);
        generated.0 = 1;
        return;
    }

    // Downsample the whole remaining mip chain in one submission: it is cheap (≤ 64³) and
    // spreading it over frames let distant (high-mip) cloud samples read cleared mips.
    for mip in generated.0..texture.mip_level_count {
    let mut mip_size = texture.size;
    for _ in 1..=mip {
        mip_size = UVec3::new(
            (mip_size.x / 2).max(1),
            (mip_size.y / 2).max(1),
            (mip_size.z / 2).max(1),
        );
    }
    let mip_label = format!("perlin_worley_mip_downsample_{mip}");
    let mip_bind = render_device.create_bind_group(
        mip_label.as_str(),
        &mip_layout.layout,
        &BindGroupEntries::with_indices((
            (13, &texture.mip_views[(mip - 1) as usize]),
            (14, &texture.mip_views[mip as usize]),
        )),
    );
    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("perlin_worley_noise_mip_downsample"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(mip_compute);
        compute_pass.set_bind_group(0, &mip_bind, &[]);
        let dispatch_size = get_3d_dispatch_size(mip_size);
        compute_pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, dispatch_size.z);
    }

    }
    let command_buffer = encoder.finish();
    render_queue.submit([command_buffer]);
    generated.0 = texture.mip_level_count;
}
