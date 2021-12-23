mod node;

pub use node::TonemappingNode;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::BevyDefault;
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, RenderApp, RenderStage};

use bevy_reflect::TypeUuid;

const TONEMAPPING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 17015368199668024512);

pub struct TonemappingPlugin;

impl Plugin for TonemappingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TONEMAPPING_SHADER_HANDLE,
            "tonemapping.wgsl",
            Shader::from_wgsl
        );

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<TonemappingPipeline>()
            .init_resource::<SpecializedPipelines<TonemappingPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_tonemapping_bind_groups);
    }
}

pub struct TonemappingPipeline {
    hdr_texture_bind_group: BindGroupLayout,
}

impl FromWorld for TonemappingPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.get_resource::<RenderDevice>().unwrap();

        let hdr_texture_bind_group =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            hdr_texture_bind_group,
        }
    }
}

impl SpecializedPipeline for TonemappingPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("tonemapping pipeline".into()),
            layout: Some(vec![self.hdr_texture_bind_group.clone()]),
            vertex: VertexState {
                shader: TONEMAPPING_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "vs_main".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: TONEMAPPING_SHADER_HANDLE.typed(),
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
pub struct TonemappingTarget {
    pub hdr_texture_bind_group: BindGroup,
    pub pipeline: CachedPipelineId,
}

fn queue_tonemapping_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut render_pipeline_cache: ResMut<RenderPipelineCache>,
    mut pipelines: ResMut<SpecializedPipelines<TonemappingPipeline>>,
    tonemapping_pipeline: Res<TonemappingPipeline>,
    view_targets: Query<(Entity, &ViewTarget)>,
) {
    for (entity, target) in view_targets.iter() {
        let pipeline = pipelines.specialize(&mut render_pipeline_cache, &tonemapping_pipeline, ());

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &tonemapping_pipeline.hdr_texture_bind_group,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&target.hdr_texture),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        commands.entity(entity).insert(TonemappingTarget {
            hdr_texture_bind_group: bind_group,
            pipeline,
        });
    }
}
