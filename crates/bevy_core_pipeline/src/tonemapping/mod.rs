use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::camera::Camera;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::renderer::RenderDevice;
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, RenderApp, RenderStage};

mod node;

pub use node::TonemappingNode;

const TONEMAPPING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17015368199668024512);

const TONEMAPPING_SHARED_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2499430578245347910);

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

        app.register_type::<Tonemapping>();

        app.add_plugin(ExtractComponentPlugin::<Tonemapping>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TonemappingPipeline>()
                .init_resource::<SpecializedRenderPipelines<TonemappingPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_view_tonemapping_pipelines);
        }
    }
}

#[derive(Resource)]
pub struct TonemappingPipeline {
    texture_bind_group: BindGroupLayout,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TonemappingPipelineKey {
    deband_dither: bool,
}

impl SpecializedRenderPipeline for TonemappingPipeline {
    type Key = TonemappingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.deband_dither {
            shader_defs.push("DEBAND_DITHER".into());
        }
        RenderPipelineDescriptor {
            label: Some("tonemapping pipeline".into()),
            layout: Some(vec![self.texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TONEMAPPING_SHADER_HANDLE.typed(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
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

        TonemappingPipeline {
            texture_bind_group: tonemap_texture_bind_group,
        }
    }
}

#[derive(Component)]
pub struct ViewTonemappingPipeline(CachedRenderPipelineId);

pub fn queue_view_tonemapping_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TonemappingPipeline>>,
    upscaling_pipeline: Res<TonemappingPipeline>,
    view_targets: Query<(Entity, &Tonemapping)>,
) {
    for (entity, tonemapping) in view_targets.iter() {
        if let Tonemapping::Enabled { deband_dither } = tonemapping {
            let key = TonemappingPipelineKey {
                deband_dither: *deband_dither,
            };
            let pipeline = pipelines.specialize(&mut pipeline_cache, &upscaling_pipeline, key);

            commands
                .entity(entity)
                .insert(ViewTonemappingPipeline(pipeline));
        }
    }
}

#[derive(Component, Clone, Reflect, Default)]
#[reflect(Component)]
pub enum Tonemapping {
    #[default]
    Disabled,
    Enabled {
        deband_dither: bool,
    },
}

impl Tonemapping {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Tonemapping::Enabled { .. })
    }
}

impl ExtractComponent for Tonemapping {
    type Query = &'static Self;
    type Filter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
