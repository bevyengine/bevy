use super::{
    billboard_gizmo_vertex_buffer_layouts, BillboardGizmoUniformBindgroupLayout,
    DrawBillboardGizmo, GpuBillboardGizmo, SetBillboardGizmoBindGroup,
};
use crate::{config::GizmoMeshConfig, GizmoRenderSystem};
use bevy_app::{App, Plugin};
use bevy_core_pipeline::{
    core_3d::{Transparent3d, CORE_3D_DEPTH_FORMAT},
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
};

use bevy_ecs::{
    prelude::Entity,
    query::Has,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_pbr::{MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup};
use bevy_render::{
    render_asset::prepare_assets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, SetItemPipeline,
        ViewSortedRenderPhases,
    },
    render_resource::*,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, RenderLayers, ViewTarget},
    Render, RenderApp,
};

use super::BILLBOARD_SHADER_HANDLE;

pub struct BillboardGizmo3dPlugin;
impl Plugin for BillboardGizmo3dPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawBillboardGizmo3d>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .add_systems(
                Render,
                queue_billboard_gizmos_3d
                    .in_set(GizmoRenderSystem::QueueGizmos3d)
                    .after(prepare_assets::<GpuBillboardGizmo>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
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
                .resource::<BillboardGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineGizmoPipelineKey {
    view_key: MeshPipelineKey,
    perspective: bool,
}

impl SpecializedRenderPipeline for LineGizmoPipeline {
    type Key = LineGizmoPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![
            #[cfg(feature = "webgl")]
            "SIXTEEN_BYTE_ALIGNMENT".into(),
        ];

        if key.perspective {
            shader_defs.push("PERSPECTIVE".into());
        }

        let format = if key.view_key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let view_layout = self
            .mesh_pipeline
            .get_view_layout(key.view_key.into())
            .clone();

        let layout = vec![view_layout, self.uniform_layout.clone()];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: BILLBOARD_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: billboard_gizmo_vertex_buffer_layouts(),
            },
            fragment: Some(FragmentState {
                shader: BILLBOARD_SHADER_HANDLE,
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
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.view_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo Pipeline".into()),
            push_constant_ranges: vec![],
        }
    }
}

type DrawBillboardGizmo3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetBillboardGizmoBindGroup<1>,
    DrawBillboardGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_billboard_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    billboard_gizmos: Query<(Entity, &GizmoMeshConfig)>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &Msaa,
        Option<&RenderLayers>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
    )>,
) {
    let draw_function = draw_functions
        .read()
        .get_id::<DrawBillboardGizmo3d>()
        .unwrap();

    for (
        view_entity,
        view,
        msaa,
        render_layers,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
    ) in &mut views
    {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let render_layers = render_layers.unwrap_or_default();

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        for (entity, config) in &billboard_gizmos {
            if !config.render_layers.intersects(render_layers) {
                continue;
            }

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineGizmoPipelineKey {
                    view_key,
                    perspective: config.billboard_perspective,
                },
            );

            transparent_phase.add(Transparent3d {
                entity,
                draw_function,
                pipeline,
                distance: 0.,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::NONE,
            });
        }
    }
}
