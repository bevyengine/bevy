//! Tileable 2D curl noise (Nubis/HZD 128²) for cloud distortion.

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

#[derive(Clone, Copy, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CurlNoiseParams {
    pub base_frequency: u32,
    pub _pad0: u32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for CurlNoiseParams {
    fn default() -> Self {
        Self {
            base_frequency: 4,
            _pad0: 0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct CurlNoiseTextureSize {
    pub size: UVec2,
}

impl Default for CurlNoiseTextureSize {
    fn default() -> Self {
        Self {
            size: UVec2::new(128, 128),
        }
    }
}

fn mip_count_for_size_2d(size: UVec2) -> u32 {
    let min_dim = size.x.min(size.y).max(1);
    min_dim.ilog2() + 1
}

#[derive(Resource)]
pub struct CurlNoiseBindGroupLayout {
    pub layout: BindGroupLayout,
    pub descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for CurlNoiseBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let descriptor = BindGroupLayoutDescriptor::new(
            "curl_noise_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rg16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                    (14, uniform_buffer::<CurlNoiseParams>(false)),
                ),
            ),
        );
        let layout =
            render_device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);
        Self { layout, descriptor }
    }
}

#[derive(Resource)]
pub struct CurlNoisePipeline {
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for CurlNoisePipeline {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layout = world.resource::<CurlNoiseBindGroupLayout>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("curl_noise_2d_pipeline".into()),
            layout: vec![layout.descriptor.clone()],
            shader: load_embedded_asset!(world, "curl_noise_2d.wesl"),
            ..default()
        });
        Self { pipeline }
    }
}

#[derive(Resource)]
pub struct CurlNoiseTexture {
    pub texture: CachedTexture,
    pub size: UVec2,
    pub mip_level_count: u32,
    /// Single-mip storage view used by the generation pass.
    pub mip0_storage_view: TextureView,
}

#[derive(Resource)]
pub struct CurlNoiseBindGroup {
    pub bind_group: BindGroup,
}

#[derive(Resource)]
pub struct CurlNoiseParamsBuffer {
    pub buffer: Buffer,
}

#[derive(Resource, Default)]
pub struct CurlNoiseGenerated(pub bool);

pub fn init_curl_noise_texture(
    render_device: Res<RenderDevice>,
    mut commands: bevy_ecs::system::Commands,
) {
    let size = CurlNoiseTextureSize::default();
    // Generate mip 0 only for now; curl LOD is less critical than 3D shape/detail.
    // Keep a mip chain so filtered sampling works; fill only mip 0 (higher mips stay 0).
    let mip_level_count = mip_count_for_size_2d(size.size);
    let texture_descriptor = TextureDescriptor {
        label: Some("curl_noise_2d_texture"),
        size: Extent3d {
            width: size.size.x,
            height: size.size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rg16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = render_device.create_texture(&texture_descriptor);
    let default_view = texture.create_view(&TextureViewDescriptor::default());
    let mip0_storage_view = texture.create_view(&TextureViewDescriptor {
        label: Some("curl_noise_2d_mip0"),
        format: Some(TextureFormat::Rg16Float),
        dimension: Some(TextureViewDimension::D2),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        ..default()
    });
    commands.insert_resource(CurlNoiseTexture {
        texture: CachedTexture {
            texture,
            default_view,
        },
        size: size.size,
        mip_level_count,
        mip0_storage_view,
    });
}

pub fn init_curl_noise_params_buffer(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
) {
    let params = CurlNoiseParams::default();
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("curl_noise_params_buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    commands.insert_resource(CurlNoiseParamsBuffer { buffer });
}

pub fn prepare_curl_noise_bind_group(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
    layout: Res<CurlNoiseBindGroupLayout>,
    texture: Res<CurlNoiseTexture>,
    params_buffer: Res<CurlNoiseParamsBuffer>,
) {
    let bind_group = render_device.create_bind_group(
        "curl_noise_bind_group",
        &layout.layout,
        &BindGroupEntries::with_indices((
            (13, &texture.mip0_storage_view),
            (14, params_buffer.buffer.as_entire_binding()),
        )),
    );
    commands.insert_resource(CurlNoiseBindGroup { bind_group });
}

pub fn generate_curl_noise_once(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<CurlNoisePipeline>,
    bind_group: Option<Res<CurlNoiseBindGroup>>,
    texture: Res<CurlNoiseTexture>,
    mut generated: ResMut<CurlNoiseGenerated>,
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
        label: Some("curl_noise_generation"),
    });
    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("curl_noise_2d_pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group.bind_group, &[]);
        const WG: u32 = 8;
        compute_pass.dispatch_workgroups(
            texture.size.x.div_ceil(WG),
            texture.size.y.div_ceil(WG),
            1,
        );
    }
    render_queue.submit([encoder.finish()]);
    generated.0 = true;
}
