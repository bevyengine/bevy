mod node;

use bevy_ecs::query::QueryItem;
use bevy_render::camera::Camera;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
pub use node::TonemappingNode;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::BevyDefault;
use bevy_render::{render_resource::*, RenderApp};

use bevy_reflect::TypeUuid;

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;

const TONEMAPPING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17015368199668024512);

const TONEMAPPING_SHARED_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2499430578245347910);

const BLIT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2982361071241723543);

pub struct TonemappingPlugin;

impl Plugin for TonemappingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TONEMAPPING_SHADER_HANDLE,
            "tonemapping.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TONEMAPPING_SHARED_SHADER_HANDLE,
            "tonemapping_shared.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, BLIT_SHADER_HANDLE, "blit.wgsl", Shader::from_wgsl);

        app.add_plugin(ExtractComponentPlugin::<Tonemapping>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app.init_resource::<TonemappingPipeline>();
    }
}

#[derive(Resource)]
pub struct TonemappingPipeline {
    hdr_texture_bind_group: BindGroupLayout,
    tonemapping_pipeline_id: CachedRenderPipelineId,
    blit_pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for TonemappingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let tonemap_texture_bind_group = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("tonemapping_hdr_texture_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let tonemap_descriptor = RenderPipelineDescriptor {
            label: Some("tonemapping pipeline".into()),
            layout: Some(vec![tonemap_texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TONEMAPPING_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        };

        let blit_descriptor = RenderPipelineDescriptor {
            label: Some("blit pipeline".into()),
            layout: Some(vec![tonemap_texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLIT_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        };
        let mut cache = render_world.resource_mut::<PipelineCache>();
        TonemappingPipeline {
            hdr_texture_bind_group: tonemap_texture_bind_group,
            tonemapping_pipeline_id: cache.queue_render_pipeline(tonemap_descriptor),
            blit_pipeline_id: cache.queue_render_pipeline(blit_descriptor),
        }
    }
}

#[derive(Component, Clone)]
pub struct Tonemapping {
    pub is_enabled: bool,
}

impl ExtractComponent for Tonemapping {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}
