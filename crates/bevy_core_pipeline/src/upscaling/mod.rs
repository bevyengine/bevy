use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render::renderer::RenderDevice;
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, RenderApp, RenderStage};

mod node;

pub use node::UpscalingNode;

const UPSCALING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 14589267395627146578);

pub struct UpscalingPlugin;

impl Plugin for UpscalingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UPSCALING_SHADER_HANDLE,
            "upscaling.wgsl",
            Shader::from_wgsl
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<UpscalingPipeline>()
                .init_resource::<SpecializedRenderPipelines<UpscalingPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_view_upscaling_pipelines);
        }
    }
}

#[derive(Resource)]
pub struct UpscalingPipeline {
    texture_bind_group: BindGroupLayout,
}

impl FromWorld for UpscalingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let texture_bind_group =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("upscaling_texture_bind_group_layout"),
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

        UpscalingPipeline { texture_bind_group }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum UpscalingMode {
    Filtering,
    Nearest,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct UpscalingPipelineKey {
    upscaling_mode: UpscalingMode,
    texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for UpscalingPipeline {
    type Key = UpscalingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("upscaling pipeline".into()),
            layout: Some(vec![self.texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: UPSCALING_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
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

#[derive(Component)]
pub struct ViewUpscalingPipeline(CachedRenderPipelineId);

fn queue_view_upscaling_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UpscalingPipeline>>,
    upscaling_pipeline: Res<UpscalingPipeline>,
    view_targets: Query<(Entity, &ViewTarget)>,
) {
    for (entity, view_target) in view_targets.iter() {
        let key = UpscalingPipelineKey {
            upscaling_mode: UpscalingMode::Filtering,
            texture_format: view_target.out_texture_format(),
        };
        let pipeline = pipelines.specialize(&mut pipeline_cache, &upscaling_pipeline, key);

        commands
            .entity(entity)
            .insert(ViewUpscalingPipeline(pipeline));
    }
}
