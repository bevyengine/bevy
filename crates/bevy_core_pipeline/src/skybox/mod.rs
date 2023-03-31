mod node;

pub use node::*;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    prelude::{shape::Cube, Mesh},
    render_asset::RenderAssets,
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, BlendState, BufferBindingType,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
        DepthStencilState, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
        RenderPipelineDescriptor, SamplerBindingType, Shader, ShaderDefVal, ShaderStages,
        ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, StencilFaceState,
        StencilState, TextureFormat, TextureSampleType, TextureViewDimension, VertexBufferLayout,
        VertexState,
    },
    renderer::RenderDevice,
    texture::{BevyDefault, Image},
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniforms},
    Render, RenderApp, RenderSet,
};

const SKYBOX_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 55594763423201);

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SKYBOX_SHADER_HANDLE, "skybox.wgsl", Shader::from_wgsl);

        app.add_plugin(ExtractComponentPlugin::<Skybox>::default());

        let mesh = Mesh::from(Cube::new(1.0));
        let vertex_buffer_layout = mesh.get_mesh_vertex_buffer_layout().layout().clone();
        let handle = app.world.resource_mut::<Assets<Mesh>>().add(mesh);

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        let render_device = render_app.world.resource::<RenderDevice>().clone();

        render_app
            .insert_resource(SkyboxMesh { handle })
            .insert_resource(SkyboxPipeline::new(vertex_buffer_layout, &render_device))
            .init_resource::<SpecializedRenderPipelines<SkyboxPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_skybox_pipelines.in_set(RenderSet::Prepare),
                    queue_skybox_bind_groups.in_set(RenderSet::Queue),
                ),
            );
    }
}

/// Adds a skybox to a 3D camera.
///
/// Note that this component does not (currently) affect the scene's lighting.
/// To do so, use `EnvironmentMapLight` alongside this component.
///
/// See also <https://en.wikipedia.org/wiki/Skybox_(video_games)>.
#[derive(Component, ExtractComponent, Clone)]
pub struct Skybox(pub Handle<Image>);

// ----------------------------------------------------------------------------

#[derive(Resource)]
struct SkyboxMesh {
    handle: Handle<Mesh>,
}

#[derive(Resource)]
struct SkyboxPipeline {
    bind_group_layout: BindGroupLayout,
    vertex_buffer_layout: VertexBufferLayout,
}

impl SkyboxPipeline {
    fn new(vertex_buffer_layout: VertexBufferLayout, render_device: &RenderDevice) -> Self {
        let bind_group_layout_descriptor = BindGroupLayoutDescriptor {
            label: Some("skybox_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                    count: None,
                },
            ],
        };

        Self {
            bind_group_layout: render_device
                .create_bind_group_layout(&bind_group_layout_descriptor),
            vertex_buffer_layout,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct SkyboxPipelineKey {
    hdr: bool,
    samples: u32,
}

impl SpecializedRenderPipeline for SkyboxPipeline {
    type Key = SkyboxPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = vec![
            ShaderDefVal::UInt("MAX_DIRECTIONAL_LIGHTS".to_string(), 1),
            ShaderDefVal::UInt("MAX_CASCADES_PER_LIGHT".to_string(), 1),
        ];

        RenderPipelineDescriptor {
            label: Some("skybox_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: SKYBOX_SHADER_HANDLE.typed(),
                shader_defs: shader_defs.clone(),
                entry_point: "skybox_vertex".into(),
                buffers: vec![self.vertex_buffer_layout.clone()],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: SKYBOX_SHADER_HANDLE.typed(),
                shader_defs,
                entry_point: "skybox_fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
        }
    }
}

#[derive(Component)]
struct SkyboxPipelineId(CachedRenderPipelineId);

fn prepare_skybox_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyboxPipeline>>,
    pipeline: Res<SkyboxPipeline>,
    msaa: Res<Msaa>,
    views: Query<(Entity, &ExtractedView), With<Skybox>>,
) {
    for (entity, view) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            SkyboxPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
            },
        );

        commands
            .entity(entity)
            .insert(SkyboxPipelineId(pipeline_id));
    }
}

#[derive(Component)]
struct SkyboxBindGroup(BindGroup);

fn queue_skybox_bind_groups(
    mut commands: Commands,
    pipeline: Res<SkyboxPipeline>,
    view_uniforms: Res<ViewUniforms>,
    images: Res<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &Skybox)>,
) {
    for (entity, skybox) in &views {
        if let (Some(skybox), Some(view_uniforms)) =
            (images.get(&skybox.0), view_uniforms.uniforms.binding())
        {
            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("skybox_bind_group"),
                layout: &pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&skybox.texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&skybox.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: view_uniforms,
                    },
                ],
            });

            commands.entity(entity).insert(SkyboxBindGroup(bind_group));
        }
    }
}
