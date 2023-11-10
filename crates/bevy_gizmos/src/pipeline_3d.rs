use crate::{
    line_gizmo_vertex_buffer_layouts, DrawLineGizmo, GizmoConfig, LineGizmo,
    LineGizmoUniformBindgroupLayout, SetLineGizmoBindGroup, LINE_SHADER_HANDLE,
};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core_pipeline::core_3d::{Transparent3d, CORE_3D_DEPTH_FORMAT};

use bevy_ecs::{
    prelude::Entity,
    query::With,
    schedule::IntoSystemConfigs,
    system::{lifetimeless::SRes, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_pbr::{MeshPipeline, PbrViewKey, SetMeshViewBindGroup};
use bevy_render::{
    pipeline_keys::{
        AddPipelineKey, KeyRepack, KeyShaderDefs, PipelineKey, PipelineKeys, SystemKey,
    },
    render_asset::{prepare_assets, RenderAssets},
    render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    view::{ExtractedView, HdrKey, MsaaKey, RenderLayers},
    Render, RenderApp, RenderSet,
};

pub struct LineGizmo3dPlugin;
impl Plugin for LineGizmo3dPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawLineGizmo3d>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .add_systems(
                Render,
                queue_line_gizmos_3d
                    .in_set(RenderSet::Queue)
                    .after(prepare_assets::<LineGizmo>),
            )
            .register_system_key::<GizmoConfigKey, With<ExtractedView>>();
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LineGizmoPipeline>();
    }
}

#[derive(Clone, Resource)]
struct LineGizmoPipeline {
    mesh_pipeline: MeshPipeline,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for LineGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        LineGizmoPipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            uniform_layout: render_world
                .resource::<LineGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PipelineKey)]
struct StripKey(bool);

#[derive(PipelineKey)]
#[custom_shader_defs]
struct GizmoConfigKey {
    perspective: bool,
}

impl SystemKey for GizmoConfigKey {
    type Param = SRes<GizmoConfig>;
    type Query = ();

    fn from_params(config: &Res<GizmoConfig>, _: ()) -> Option<Self>
    where
        Self: Sized,
    {
        Some(Self {
            perspective: config.line_perspective,
        })
    }
}

impl KeyShaderDefs for GizmoConfigKey {
    fn shader_defs(&self) -> Vec<ShaderDefVal> {
        self.perspective
            .then_some("PERSPECTIVE".into())
            .into_iter()
            .collect()
    }
}

#[derive(PipelineKey)]
struct LineGizmoPipelineKey {
    view_key: PbrViewKey,
    perspective: GizmoConfigKey,
    strip: StripKey,
}

impl SpecializedRenderPipeline for LineGizmoPipeline {
    type Key = LineGizmoPipelineKey;

    fn specialize(&self, key: PipelineKey<Self::Key>) -> RenderPipelineDescriptor {
        let shader_defs = key.shader_defs();

        let view_key = key.extract::<PbrViewKey>();
        let view_layout = self.mesh_pipeline.get_view_layout(view_key).clone();
        let layout = vec![view_layout, self.uniform_layout.clone()];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: LINE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: line_gizmo_vertex_buffer_layouts(key.strip.0),
            },
            fragment: Some(FragmentState {
                shader: LINE_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: view_key.extract::<HdrKey>().format(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout,
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: view_key.extract::<MsaaKey>().samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo Pipeline".into()),
            push_constant_ranges: vec![],
        }
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
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    config: Res<GizmoConfig>,
    line_gizmos: Query<(Entity, &Handle<LineGizmo>)>,
    line_gizmo_assets: Res<RenderAssets<LineGizmo>>,
    mut views: Query<(
        &mut RenderPhase<Transparent3d>,
        Option<&RenderLayers>,
        &PipelineKeys,
    )>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo3d>().unwrap();

    for (mut transparent_phase, render_layers, keys) in &mut views {
        let render_layers = render_layers.copied().unwrap_or_default();
        if !config.render_layers.intersects(&render_layers) {
            continue;
        }

        let Some(view_key) = keys.get_packed_key::<PbrViewKey>() else {
            continue;
        };
        let Some(config_key) = keys.get_packed_key::<GizmoConfigKey>() else {
            continue;
        };

        for (entity, handle) in &line_gizmos {
            let Some(line_gizmo) = line_gizmo_assets.get(handle) else {
                continue;
            };

            let strip_key = pipeline_cache.pack_key(&StripKey(line_gizmo.strip));
            let gizmo_key = LineGizmoPipelineKey::repack((view_key, config_key, strip_key));

            let pipeline = pipelines.specialize(&pipeline_cache, &pipeline, gizmo_key);

            transparent_phase.add(Transparent3d {
                entity,
                draw_function,
                pipeline,
                distance: 0.,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}
