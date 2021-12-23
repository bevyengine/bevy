mod node;

pub use node::UpscalingNode;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::BevyDefault;
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, RenderApp, RenderStage};

use bevy_reflect::TypeUuid;

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

        app.sub_app_mut(RenderApp)
            .init_resource::<UpscalingPipeline>()
            .init_resource::<SpecializedPipelines<UpscalingPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_upscaling_bind_groups);
    }
}

pub struct UpscalingPipeline {
    ldr_texture_bind_group: BindGroupLayout,
}

impl FromWorld for UpscalingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.get_resource::<RenderDevice>().unwrap();

        let hdr_texture_bind_group =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("upscaling_ldr_texture_bind_group_layout"),
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

        UpscalingPipeline {
            ldr_texture_bind_group: hdr_texture_bind_group,
        }
    }
}

impl SpecializedPipeline for UpscalingPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("upscaling pipeline".into()),
            layout: Some(vec![self.ldr_texture_bind_group.clone()]),
            vertex: VertexState {
                shader: UPSCALING_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "vs_main".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: UPSCALING_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

#[derive(Component)]
pub struct UpscalingTarget {
    pub ldr_texture_bind_group: BindGroup,
    pub pipeline: CachedPipelineId,
}

fn queue_upscaling_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut render_pipeline_cache: ResMut<RenderPipelineCache>,
    mut pipelines: ResMut<SpecializedPipelines<UpscalingPipeline>>,
    upscaling_pipeline: Res<UpscalingPipeline>,
    view_targets: Query<(Entity, &ViewTarget)>,
) {
    for (entity, target) in view_targets.iter() {
        let pipeline = pipelines.specialize(&mut render_pipeline_cache, &upscaling_pipeline, ());

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &upscaling_pipeline.ldr_texture_bind_group,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&target.ldr_texture),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        commands.entity(entity).insert(UpscalingTarget {
            ldr_texture_bind_group: bind_group,
            pipeline,
        });
    }
}
