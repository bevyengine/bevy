//! Depth-pyramid generation with one input and one output mip per dispatch.

use bevy_app::App;
use bevy_asset::{embedded_asset, load_embedded_asset, Handle};
use bevy_render::{
    render_resource::{
        binding_types::{texture_2d, texture_2d_multisampled, texture_storage_2d},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, PipelineCache, ShaderStages, StorageTextureAccess,
        TextureFormat, TextureSampleType, TextureView, TextureViewDescriptor,
    },
    renderer::{RenderContext, RenderDevice},
};
use bevy_shader::Shader;
use bevy_utils::default;

use super::depth::{ViewDepthPyramid, DEPTH_PYRAMID_MIP_COUNT};

struct Pipeline {
    id: CachedComputePipelineId,
    layout: BindGroupLayout,
}

/// A depth-pyramid fallback selected once for devices with fewer than 12 storage textures.
pub struct Depth2MipsFallback {
    first: Pipeline,
    first_multisampled: Pipeline,
    reduce: Pipeline,
}

impl Depth2MipsFallback {
    pub fn load_shader(app: &mut App) -> Handle<Shader> {
        embedded_asset!(app, "depth_2mips_fallback.wesl");
        load_embedded_asset!(app, "depth_2mips_fallback.wesl")
    }

    pub fn new(
        device: &RenderDevice,
        cache: &PipelineCache,
        shader: Handle<Shader>,
    ) -> Option<Self> {
        if device.limits().max_storage_textures_per_shader_stage >= DEPTH_PYRAMID_MIP_COUNT as u32 {
            return None;
        }

        let pipeline = |input, shader_defs| {
            let layout = BindGroupLayoutDescriptor::new(
                "depth_2mips_fallback",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        input,
                        texture_storage_2d(
                            TextureFormat::R32Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            );
            Pipeline {
                id: cache.queue_compute_pipeline(ComputePipelineDescriptor {
                    label: Some("depth_2mips_fallback".into()),
                    layout: vec![layout.clone()],
                    shader: shader.clone(),
                    shader_defs,
                    entry_point: Some("main".into()),
                    ..default()
                }),
                layout: cache.get_bind_group_layout(&layout),
            }
        };

        Some(Self {
            first: pipeline(
                texture_2d(TextureSampleType::Depth),
                vec!["FIRST_MIP".into()],
            ),
            first_multisampled: pipeline(
                texture_2d_multisampled(TextureSampleType::Depth),
                vec!["FIRST_MIP".into(), "MULTISAMPLE".into()],
            ),
            reduce: pipeline(
                texture_2d(TextureSampleType::Float { filterable: false }),
                Vec::new(),
            ),
        })
    }

    fn first(&self, multisampled: bool) -> &Pipeline {
        if multisampled {
            &self.first_multisampled
        } else {
            &self.first
        }
    }

    pub fn create_bind_groups(
        &self,
        device: &RenderDevice,
        pyramid: &ViewDepthPyramid,
        source: &TextureView,
        multisampled: bool,
    ) -> (BindGroup, Vec<BindGroup>) {
        let mip_view = |mip: u32| {
            pyramid.mips.get(mip as usize).cloned().unwrap_or_else(|| {
                pyramid
                    .all_mips
                    .texture()
                    .create_view(&TextureViewDescriptor {
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        ..default()
                    })
                    .into()
            })
        };
        let mut input = mip_view(0);
        let first = device.create_bind_group(
            "depth_2mips_fallback first mip",
            &self.first(multisampled).layout,
            &BindGroupEntries::sequential((source, &input)),
        );
        let mut chain = Vec::with_capacity(pyramid.mip_count.saturating_sub(1) as usize);
        for mip in 1..pyramid.mip_count {
            let output = mip_view(mip);
            chain.push(device.create_bind_group(
                "depth_2mips_fallback mip",
                &self.reduce.layout,
                &BindGroupEntries::sequential((&input, &output)),
            ));
            input = output;
        }
        (first, chain)
    }

    pub fn downsample(
        &self,
        label: &str,
        ctx: &mut RenderContext,
        cache: &PipelineCache,
        pyramid: &ViewDepthPyramid,
        bind_groups: (&BindGroup, &[BindGroup]),
        multisampled: bool,
    ) {
        let (Some(first), Some(reduce)) = (
            cache.get_compute_pipeline(self.first(multisampled).id),
            cache.get_compute_pipeline(self.reduce.id),
        ) else {
            return;
        };
        let mut size = pyramid.all_mips.texture().size();
        let mut pass = ctx
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });
        for (mip, bind_group) in core::iter::once(bind_groups.0)
            .chain(bind_groups.1)
            .enumerate()
        {
            pass.set_pipeline(if mip == 0 { first } else { reduce });
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
            size.width = (size.width / 2).max(1);
            size.height = (size.height / 2).max(1);
        }
    }
}
