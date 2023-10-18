use std::borrow::Cow;

use super::{BloomSettings, BLOOM_SHADER_HANDLE, BLOOM_TEXTURE_FORMAT};
use crate::mipmap_generator::{Mipmap, MipmapDebugNames, MipmapPipeline, MipmapPipelineIds};
use bevy_asset::Handle;
use bevy_ecs::{
    prelude::{Component, Entity},
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::Vec4;
use bevy_render::render_resource::*;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomDownsamplingMipmapper {
    prefilter: bool,
}

/// The uniform struct extracted from [`BloomSettings`] attached to a Camera.
/// Will be available for use in the Bloom shader.
#[derive(Component, ShaderType, Clone)]
pub struct BloomUniforms {
    // Precomputed values used when thresholding, see https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/#3.4
    pub threshold_precomputations: Vec4,
    pub viewport: Vec4,
    pub aspect: f32,
}

impl Mipmap for BloomDownsamplingMipmapper {
    fn debug_names() -> &'static MipmapDebugNames {
        static DEBUG_NAMES: MipmapDebugNames = MipmapDebugNames {
            bind_group_layout: "bloom_downsampling_bind_group_layout",
            first_bind_group: "bloom_downsampling_first_bind_group",
            first_pass: "bloom_downsampling_first_pass",
            first_pipeline: "bloom_downsampling_first_pipeline",
            rest_bind_group: "bloom_downsampling_rest_bind_group",
            rest_pass: "bloom_downsampling_rest_pass",
            rest_pipeline: "bloom_downsampling_rest_pipeline",
            texture: "bloom_texture",
        };

        &DEBUG_NAMES
    }

    fn shader_entry_point(first: bool) -> Cow<'static, str> {
        if first {
            "downsample_first".into()
        } else {
            "downsample".into()
        }
    }

    fn shader() -> Handle<Shader> {
        BLOOM_SHADER_HANDLE
    }

    fn texture_format() -> TextureFormat {
        BLOOM_TEXTURE_FORMAT
    }

    fn add_custom_bind_group_layout_entries(entries: &mut Vec<BindGroupLayoutEntry>) {
        // Downsampling settings binding
        entries.push(BindGroupLayoutEntry {
            binding: 2,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(BloomUniforms::min_size()),
            },
            visibility: ShaderStages::FRAGMENT,
            count: None,
        });
    }

    fn add_custom_shader_defs(&self, shader_defs: &mut Vec<ShaderDefVal>) {
        if self.prefilter {
            shader_defs.push("USE_THRESHOLD".into());
        }
    }

    fn mip_levels_to_omit() -> u32 {
        1
    }
}

pub fn prepare_downsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<MipmapPipeline<BloomDownsamplingMipmapper>>>,
    pipeline: Res<MipmapPipeline<BloomDownsamplingMipmapper>>,
    views: Query<(Entity, &BloomSettings)>,
) {
    for (entity, settings) in &views {
        let mipmapper = BloomDownsamplingMipmapper {
            prefilter: settings.prefilter_settings.threshold > 0.0,
        };

        commands.entity(entity).insert(MipmapPipelineIds::new(
            mipmapper,
            &pipeline_cache,
            &mut pipelines,
            &pipeline,
        ));
    }
}
