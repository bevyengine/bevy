use core::num::NonZeroU32;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d},
        *,
    },
    renderer::RenderDevice,
    RenderApp,
};
use binding_types::texture_2d_array;

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;

pub const BLIT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(2312396983770133547);

/// Adds support for specialized "blit pipelines", which can be used to write one texture to another.
pub struct BlitPlugin;

impl Plugin for BlitPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLIT_SHADER_HANDLE, "blit.wgsl", Shader::from_wgsl);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.allow_ambiguous_resource::<SpecializedRenderPipelines<BlitPipeline>>();
        }
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<BlitPipeline>()
            .init_resource::<SpecializedRenderPipelines<BlitPipeline>>();
    }
}

#[derive(Resource)]
pub struct BlitPipeline {
    pub texture_bind_group: BindGroupLayout,
    pub texture_bind_group_multiview: BindGroupLayout,
    pub sampler: Sampler,
}

impl FromWorld for BlitPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let texture_bind_group = render_device.create_bind_group_layout(
            "blit_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    sampler(SamplerBindingType::NonFiltering),
                ),
            ),
        );

        let texture_bind_group_multiview = render_device.create_bind_group_layout(
            "multiview_blit_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d_array(TextureSampleType::Float { filterable: false }),
                    sampler(SamplerBindingType::NonFiltering),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        BlitPipeline {
            texture_bind_group,
            texture_bind_group_multiview,
            sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct BlitPipelineKey {
    pub texture_format: TextureFormat,
    pub blend_state: Option<BlendState>,
    pub multiview: Option<NonZeroU32>,
    pub samples: u32,
}

impl SpecializedRenderPipeline for BlitPipeline {
    type Key = BlitPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("blit pipeline".into()),
            layout: vec![if key.multiview.is_some() {
                self.texture_bind_group_multiview.clone()
            } else {
                self.texture_bind_group.clone()
            }],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLIT_SHADER_HANDLE,
                shader_defs: if key.multiview.is_some() {
                    vec!["MULTIVIEW".into()]
                } else {
                    vec![]
                },
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: key.blend_state,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.samples,
                ..Default::default()
            },
            push_constant_ranges: Vec::new(),
            multiview: None,
            zero_initialize_workgroup_memory: false,
        }
    }
}
