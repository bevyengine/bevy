use crate::{
    line_gizmo_vertex_buffer_layouts, DrawLineGizmo, GizmoConfig, LineGizmo, LineGizmoUniform,
    LINE_SHADER_HANDLE,
};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core_pipeline::core_3d::Transparent3d;

use bevy_ecs::{
    prelude::Entity,
    query::ROQueryItem,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_pbr::{
    MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup, MAX_CASCADES_PER_LIGHT,
    MAX_DIRECTIONAL_LIGHTS,
};
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, ViewTarget},
    Render, RenderApp, RenderSet,
};

pub struct LineGizmo3dPlugin;
impl Plugin for LineGizmo3dPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        render_app
            .add_render_command::<Transparent3d, DrawLineGizmo3d>()
            .init_resource::<GizmoLinePipeline>()
            .init_resource::<SpecializedRenderPipelines<GizmoLinePipeline>>()
            .add_systems(
                Render,
                (queue_polyline_bind_group, queue_line_gizmos_3d).in_set(RenderSet::Queue),
            );
    }
}

#[derive(Clone, Resource)]
struct GizmoLinePipeline {
    mesh_pipeline: MeshPipeline,
    layout: BindGroupLayout,
}

impl FromWorld for GizmoLinePipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(LineGizmoUniform::min_size().into()),
                },
                count: None,
            }],
            label: Some("polyline_layout"),
        });

        GizmoLinePipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            layout,
        }
    }
}

impl SpecializedRenderPipeline for GizmoLinePipeline {
    type Key = (bool, bool, MeshPipelineKey);

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let (perspective, strip, key) = key;

        let mut shader_defs = vec!["GIZMO_3D".into()];

        shader_defs.push(ShaderDefVal::Int(
            "MAX_DIRECTIONAL_LIGHTS".to_string(),
            MAX_DIRECTIONAL_LIGHTS as i32,
        ));
        shader_defs.push(ShaderDefVal::Int(
            "MAX_CASCADES_PER_LIGHT".to_string(),
            MAX_CASCADES_PER_LIGHT as i32,
        ));

        if perspective {
            shader_defs.push("PERSPECTIVE".into());
        }

        let format = if key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let view_layout = if key.msaa_samples() == 1 {
            self.mesh_pipeline.view_layout.clone()
        } else {
            self.mesh_pipeline.view_layout_multisampled.clone()
        };

        let layout = vec![view_layout, self.layout.clone()];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: LINE_SHADER_HANDLE.typed(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: line_gizmo_vertex_buffer_layouts(strip),
            },
            fragment: Some(FragmentState {
                shader: LINE_SHADER_HANDLE.typed(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout,
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo Pipeline".into()),
            push_constant_ranges: vec![],
        }
    }
}

#[derive(Resource)]
struct GizmoLineBindGroup {
    value: BindGroup,
}

fn queue_polyline_bind_group(
    mut commands: Commands,
    polyline_pipeline: Res<GizmoLinePipeline>,
    render_device: Res<RenderDevice>,
    polyline_uniforms: Res<ComponentUniforms<LineGizmoUniform>>,
) {
    if let Some(binding) = polyline_uniforms.uniforms().binding() {
        commands.insert_resource(GizmoLineBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("polyline_bind_group"),
                layout: &polyline_pipeline.layout,
            }),
        });
    }
}

struct SetLineGizmoBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetLineGizmoBindGroup<I> {
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<LineGizmoUniform>>;
    type Param = SRes<GizmoLineBindGroup>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        polyline_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.into_inner().value, &[polyline_index.index()]);
        RenderCommandResult::Success
    }
}

type DrawLineGizmo3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_line_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<GizmoLinePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<GizmoLinePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    config: Res<GizmoConfig>,
    line_gizmos: Query<(Entity, &Handle<LineGizmo>)>,
    line_gizmo_assets: Res<RenderAssets<LineGizmo>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo3d>().unwrap();

    for (view, mut transparent_phase) in &mut views {
        let polyline_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        for (entity, handle) in &line_gizmos {
            let line_gizmo = line_gizmo_assets.get(handle).unwrap();

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                (config.line_perspective, line_gizmo.strip, polyline_key),
            );

            transparent_phase.add(Transparent3d {
                entity,
                draw_function,
                pipeline,
                distance: 0.,
            });
        }
    }
}
