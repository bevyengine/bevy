//! Tileable 3D Worley detail noise (Nubis/HZD 32³) for cloud erosion.

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

use super::perlin_worley_noise::{
    get_3d_dispatch_size, NoiseMipDownsampleBindGroupLayout, NoiseMipDownsamplePipeline,
};

#[derive(Clone, Copy, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct DetailNoiseParams {
    pub base_frequency: u32,
    pub _pad0: u32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for DetailNoiseParams {
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
pub struct DetailNoiseTextureSize {
    pub size: UVec3,
}

impl Default for DetailNoiseTextureSize {
    fn default() -> Self {
        Self {
            size: UVec3::new(32, 32, 32),
        }
    }
}

fn mip_count_for_size(size: UVec3) -> u32 {
    let min_dim = size.x.min(size.y).min(size.z).max(1);
    min_dim.ilog2() + 1
}

#[derive(Resource)]
pub struct DetailNoiseBindGroupLayout {
    pub layout: BindGroupLayout,
    pub descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for DetailNoiseBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let descriptor = BindGroupLayoutDescriptor::new(
            "detail_noise_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        13,
                        texture_storage_3d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                    (14, uniform_buffer::<DetailNoiseParams>(false)),
                ),
            ),
        );
        let layout =
            render_device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);
        Self { layout, descriptor }
    }
}

#[derive(Resource)]
pub struct DetailNoisePipeline {
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for DetailNoisePipeline {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layout = world.resource::<DetailNoiseBindGroupLayout>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("detail_noise_3d_pipeline".into()),
            layout: vec![layout.descriptor.clone()],
            shader: load_embedded_asset!(world, "detail_noise_3d.wesl"),
            ..default()
        });
        Self { pipeline }
    }
}

#[derive(Resource)]
pub struct DetailNoiseTexture {
    pub texture: CachedTexture,
    pub size: UVec3,
    pub mip_level_count: u32,
    pub mip_views: Vec<TextureView>,
}

#[derive(Resource)]
pub struct DetailNoiseBindGroup {
    pub bind_group: BindGroup,
}

#[derive(Resource)]
pub struct DetailNoiseParamsBuffer {
    pub buffer: Buffer,
}

#[derive(Resource, Default)]
pub struct DetailNoiseGenerated(pub bool);

pub fn init_detail_noise_texture(
    render_device: Res<RenderDevice>,
    mut commands: bevy_ecs::system::Commands,
) {
    let size = DetailNoiseTextureSize::default();
    let mip_level_count = mip_count_for_size(size.size);
    let texture_descriptor = TextureDescriptor {
        label: Some("detail_noise_3d_texture"),
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
            label: Some(&format!("detail_noise_3d_mip{mip}")),
            format: Some(TextureFormat::Rgba16Float),
            dimension: Some(TextureViewDimension::D3),
            aspect: TextureAspect::All,
            base_mip_level: mip,
            mip_level_count: Some(1),
            ..default()
        }));
    }
    commands.insert_resource(DetailNoiseTexture {
        texture: CachedTexture {
            texture,
            default_view,
        },
        size: size.size,
        mip_level_count,
        mip_views,
    });
}

pub fn init_detail_noise_params_buffer(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
) {
    let params = DetailNoiseParams::default();
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("detail_noise_params_buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    commands.insert_resource(DetailNoiseParamsBuffer { buffer });
}

pub fn prepare_detail_noise_bind_group(
    mut commands: bevy_ecs::system::Commands,
    render_device: Res<RenderDevice>,
    layout: Res<DetailNoiseBindGroupLayout>,
    texture: Res<DetailNoiseTexture>,
    params_buffer: Res<DetailNoiseParamsBuffer>,
) {
    let bind_group = render_device.create_bind_group(
        "detail_noise_bind_group",
        &layout.layout,
        &BindGroupEntries::with_indices((
            (13, &texture.mip_views[0]),
            (14, params_buffer.buffer.as_entire_binding()),
        )),
    );
    commands.insert_resource(DetailNoiseBindGroup { bind_group });
}

pub fn generate_detail_noise_once(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<DetailNoisePipeline>,
    mip_pipeline: Res<NoiseMipDownsamplePipeline>,
    bind_group: Option<Res<DetailNoiseBindGroup>>,
    mip_layout: Res<NoiseMipDownsampleBindGroupLayout>,
    texture: Res<DetailNoiseTexture>,
    mut generated: ResMut<DetailNoiseGenerated>,
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
    let Some(mip_compute) = pipeline_cache.get_compute_pipeline(mip_pipeline.pipeline) else {
        return;
    };

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("detail_noise_generation"),
    });
    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("detail_noise_3d_pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group.bind_group, &[]);
        let dispatch_size = get_3d_dispatch_size(texture.size);
        compute_pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, dispatch_size.z);
    }

    let mut mip_size = texture.size;
    for mip in 1..texture.mip_level_count {
        mip_size = UVec3::new(
            (mip_size.x / 2).max(1),
            (mip_size.y / 2).max(1),
            (mip_size.z / 2).max(1),
        );
        let mip_label = format!("detail_noise_mip_downsample_{mip}");
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
                label: Some("detail_noise_mip_downsample"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(mip_compute);
            compute_pass.set_bind_group(0, &mip_bind, &[]);
            let dispatch_size = get_3d_dispatch_size(mip_size);
            compute_pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, dispatch_size.z);
        }
    }

    render_queue.submit([encoder.finish()]);
    generated.0 = true;
}
