use bevy_core_pipeline::FullscreenShader;

use super::{Bloom, BLOOM_TEXTURE_FORMAT};
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_ecs::{
    prelude::{Component, Entity},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::{Vec2, Vec4};
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, texture_2d_array, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    view::ExtractedMultiview,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

#[derive(Component)]
pub struct BloomDownsamplingPipelineIds {
    pub main: CachedRenderPipelineId,
    pub first: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct BloomDownsamplingPipeline {
    /// Layout for the regular downsample passes (read bloom's own single-layer
    /// mip levels).
    pub bind_group_layout: BindGroupLayoutDescriptor,
    /// Layout for the *first* downsample pass when the camera is multiview —
    /// reads the camera's `texture_2d_array` main texture; the layer is picked
    /// via `@builtin(view_index)` in the fragment.
    pub bind_group_layout_multiview: BindGroupLayoutDescriptor,
    pub sampler: Sampler,
    /// The asset handle for the fullscreen vertex shader.
    pub fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    pub fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomDownsamplingPipelineKeys {
    prefilter: bool,
    first_downsample: bool,
    uniform_scale: bool,
    /// Number of layers in the source texture, used only for the
    /// `first_downsample` specialization (subsequent passes read bloom's own
    /// single-layer mips). `> 1` emits the MULTIVIEW shader-defs and picks
    /// the array bind-group layout.
    multiview_view_count: u32,
}

/// The uniform struct extracted from [`Bloom`] attached to a Camera.
/// Will be available for use in the Bloom shader.
#[derive(Component, ShaderType, Clone)]
pub struct BloomUniforms {
    // Precomputed values used when thresholding, see https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/#3.4
    pub threshold_precomputations: Vec4,
    pub viewport: Vec4,
    pub scale: Vec2,
    pub aspect: f32,
}

pub fn init_bloom_downsampling_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    // Bind group layout
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "bloom_downsampling_bind_group_layout_with_settings",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // Input texture binding
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Sampler binding
                sampler(SamplerBindingType::Filtering),
                // Downsampling settings binding
                uniform_buffer::<BloomUniforms>(true),
            ),
        ),
    );

    let bind_group_layout_multiview = BindGroupLayoutDescriptor::new(
        "bloom_downsampling_bind_group_layout_with_settings_multiview",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<BloomUniforms>(true),
            ),
        ),
    );

    // Sampler
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        ..Default::default()
    });

    commands.insert_resource(BloomDownsamplingPipeline {
        bind_group_layout,
        bind_group_layout_multiview,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "bloom.wgsl"),
    });
}

impl SpecializedRenderPipeline for BloomDownsamplingPipeline {
    type Key = BloomDownsamplingPipelineKeys;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let entry_point = if key.first_downsample {
            "downsample_first".into()
        } else {
            "downsample".into()
        };

        let mut shader_defs = vec![];

        if key.first_downsample {
            shader_defs.push("FIRST_DOWNSAMPLE".into());
        }

        if key.prefilter {
            shader_defs.push("USE_THRESHOLD".into());
        }

        if key.uniform_scale {
            shader_defs.push("UNIFORM_SCALE".into());
        }

        // Multiview only applies to the first downsample pass — subsequent
        // passes read bloom's own single-layer mip pyramid. Pick the array
        // layout + emit MULTIVIEW defs only when both conditions hold.
        let layout = if key.first_downsample && key.multiview_view_count > 1 {
            shader_defs.push("MULTIVIEW".into());
            shader_defs.push(ShaderDefVal::UInt(
                "MAX_VIEW_COUNT".into(),
                key.multiview_view_count,
            ));
            self.bind_group_layout_multiview.clone()
        } else {
            self.bind_group_layout.clone()
        };

        RenderPipelineDescriptor {
            label: Some(
                if key.first_downsample {
                    "bloom_downsampling_pipeline_first"
                } else {
                    "bloom_downsampling_pipeline"
                }
                .into(),
            ),
            layout: vec![layout],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                entry_point: Some(entry_point),
                targets: vec![Some(ColorTargetState {
                    format: BLOOM_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            ..default()
        }
    }
}

pub fn prepare_downsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BloomDownsamplingPipeline>>,
    pipeline: Res<BloomDownsamplingPipeline>,
    views: Query<(Entity, &Bloom, Option<&ExtractedMultiview>)>,
) {
    for (entity, bloom, multiview) in &views {
        let prefilter = bloom.prefilter.threshold > 0.0;
        let multiview_view_count = multiview.map_or(1, |m| m.subviews.len() as u32);

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomDownsamplingPipelineKeys {
                prefilter,
                first_downsample: false,
                uniform_scale: bloom.scale == Vec2::ONE,
                multiview_view_count: 1,
            },
        );

        let pipeline_first_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomDownsamplingPipelineKeys {
                prefilter,
                first_downsample: true,
                uniform_scale: bloom.scale == Vec2::ONE,
                multiview_view_count,
            },
        );

        commands
            .entity(entity)
            .insert(BloomDownsamplingPipelineIds {
                first: pipeline_first_id,
                main: pipeline_id,
            });
    }
}
